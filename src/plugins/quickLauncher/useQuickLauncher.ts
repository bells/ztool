import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useSurfaceActivity } from "../../core/windowing/useSurfaceActivity";
import type {
  QuickLauncherActivationResult,
  QuickLauncherError,
  QuickLauncherIconInput,
  QuickLauncherIndexSnapshot,
  QuickLauncherResultItem,
  QuickLauncherRunningSnapshot,
  QuickLauncherSearchResult,
} from "./contracts";
import {
  canActivateLauncherItem,
  moveLauncherSelection,
  reconcileLauncherSelection,
} from "./quickLauncherModel";
import {
  acceptsPresentationCompletion,
  createLatestQueryScheduler,
  mergeBoundedIconBatch,
  resolveActivationAfterFlush,
} from "./quickLauncherScheduling";
import {
  normalizeQuickLauncherError,
  quickLauncherService,
} from "./quickLauncherService";

const QUERY_COALESCE_MS = 40;
const VISIBLE_ICON_LIMIT = 16;
const RUNNING_UPDATED_EVENT = "zero://quick-launcher/running-state-updated";

export interface QuickLauncherClient {
  getSnapshot(): Promise<QuickLauncherIndexSnapshot>;
  refresh(): Promise<QuickLauncherIndexSnapshot>;
  search(query: string, limit?: number): Promise<QuickLauncherSearchResult>;
  getIcons(items: QuickLauncherIconInput[]): Promise<{
    results: Array<{ itemId: string; dataUrl?: string }>;
  }>;
  refreshRunning(): Promise<QuickLauncherRunningSnapshot>;
  activate(itemId: string, revision: number): Promise<QuickLauncherActivationResult>;
}

export function useQuickLauncher(client: QuickLauncherClient = quickLauncherService) {
  const surfaceActivity = useSurfaceActivity();
  const [snapshot, setSnapshot] = useState<QuickLauncherIndexSnapshot | null>(null);
  const [query, setQueryState] = useState("");
  const [revision, setRevision] = useState(0);
  const [items, setItems] = useState<QuickLauncherResultItem[]>([]);
  const [selectedId, setSelectedIdState] = useState<string | null>(null);
  const [icons, setIcons] = useState<Record<string, string | null>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [activatingId, setActivatingId] = useState<string | null>(null);
  const [error, setError] = useState<QuickLauncherError | null>(null);
  const [lastAction, setLastAction] = useState<QuickLauncherActivationResult | null>(null);
  const mountedRef = useRef(false);
  const activityRef = useRef(surfaceActivity);
  const snapshotRef = useRef<QuickLauncherIndexSnapshot | null>(null);
  const queryRef = useRef("");
  const selectedIdRef = useRef<string | null>(null);
  const latestResultRef = useRef<QuickLauncherSearchResult | null>(null);
  const queryGenerationRef = useRef(0);
  const iconGenerationRef = useRef(0);
  const activationRef = useRef(false);

  activityRef.current = surfaceActivity;
  snapshotRef.current = snapshot;

  const applySearchResult = useCallback((result: QuickLauncherSearchResult) => {
    latestResultRef.current = result;
    setRevision(result.revision);
    setItems(result.items);
    setSelectedIdState((current) => {
      const next = reconcileLauncherSelection(current, result.items);
      selectedIdRef.current = next;
      return next;
    });
    setError(null);
  }, []);

  const runSearch = useCallback(async (nextQuery: string) => {
    const generation = ++queryGenerationRef.current;
    try {
      const result = await client.search(nextQuery, 24);
      if (!mountedRef.current || !acceptsPresentationCompletion(
        generation,
        queryGenerationRef.current,
        activityRef.current,
      )) return;
      applySearchResult(result);
    } catch (searchError) {
      if (mountedRef.current && generation === queryGenerationRef.current) {
        latestResultRef.current = null;
        setItems([]);
        selectedIdRef.current = null;
        setSelectedIdState(null);
        setError(normalizeQuickLauncherError(searchError));
      }
    } finally {
      if (mountedRef.current && generation === queryGenerationRef.current) {
        setIsLoading(false);
      }
    }
  }, [applySearchResult, client]);

  const queryScheduler = useMemo(() => createLatestQueryScheduler(
    runSearch,
    {
      setTimeout: (callback, delayMs) => window.setTimeout(callback, delayMs),
      clearTimeout: (timer) => window.clearTimeout(timer as number),
    },
    QUERY_COALESCE_MS,
  ), [runSearch]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      queryGenerationRef.current += 1;
      iconGenerationRef.current += 1;
      queryScheduler.cancelPending();
    };
  }, [queryScheduler]);

  useEffect(() => {
    if (surfaceActivity !== "active") {
      queryGenerationRef.current += 1;
      iconGenerationRef.current += 1;
      queryScheduler.cancelPending();
      setIsLoading(false);
      return;
    }

    let live = true;
    setIsLoading(true);
    client.getSnapshot()
      .then((nextSnapshot) => {
        if (!live || !mountedRef.current) return;
        snapshotRef.current = nextSnapshot;
        setSnapshot(nextSnapshot);
        queryScheduler.schedule(queryRef.current);
      })
      .catch((loadError) => {
        if (live && mountedRef.current) {
          setError(normalizeQuickLauncherError(loadError));
          setIsLoading(false);
        }
      });
    return () => {
      live = false;
    };
  }, [client, queryScheduler, surfaceActivity]);

  useEffect(() => {
    if (surfaceActivity !== "active") return;
    let disposed = false;
    let timer: number | undefined;
    let unlisten: (() => void) | undefined;

    const scheduleRefresh = (expiresAtMs: number) => {
      if (timer !== undefined) window.clearTimeout(timer);
      const delay = Math.max(250, expiresAtMs - Date.now());
      timer = window.setTimeout(refreshRunning, delay);
    };
    const refreshRunning = () => {
      void client.refreshRunning()
        .then((next) => {
          if (!disposed) scheduleRefresh(next.expiresAtMs);
        })
        .catch(() => {
          if (!disposed) scheduleRefresh(Date.now() + 2_000);
        });
    };

    void listen<QuickLauncherRunningSnapshot>(RUNNING_UPDATED_EVENT, () => {
      if (!disposed && snapshotRef.current) {
        queryScheduler.schedule(queryRef.current);
      }
    }).then((stop) => {
      if (disposed) {
        stop();
        return;
      }
      unlisten = stop;
      refreshRunning();
    });

    return () => {
      disposed = true;
      if (timer !== undefined) window.clearTimeout(timer);
      unlisten?.();
    };
  }, [client, queryScheduler, surfaceActivity]);

  useEffect(() => {
    if (surfaceActivity !== "active") return;
    const generation = ++iconGenerationRef.current;
    const iconItems = items.slice(0, VISIBLE_ICON_LIMIT).filter(
      (item) => item.kind === "application" && icons[item.id] === undefined,
    );
    if (iconItems.length === 0) return;

    void client.getIcons(iconItems.map((item) => (
      item.iconKey === undefined
        ? { itemId: item.id }
        : { itemId: item.id, iconKey: item.iconKey }
    )))
      .then((batch) => {
        if (!mountedRef.current || !acceptsPresentationCompletion(
          generation,
          iconGenerationRef.current,
          activityRef.current,
        )) return;
        setIcons((current) => mergeBoundedIconBatch(current, batch.results, 128));
      })
      .catch(() => {
        // Icon extraction is non-fatal and the bounded rows retain fallback glyphs.
      });
  }, [client, icons, items, surfaceActivity]);

  const setQuery = useCallback((nextQuery: string) => {
    queryRef.current = nextQuery;
    setQueryState(nextQuery);
    if (activityRef.current === "active" && snapshotRef.current) {
      setIsLoading(true);
      queryScheduler.schedule(nextQuery);
    }
  }, [queryScheduler]);

  const setSelectedId = useCallback((next: string | null) => {
    selectedIdRef.current = next;
    setSelectedIdState(next);
  }, []);

  const refresh = useCallback(async () => {
    if (isRefreshing) return;
    setIsRefreshing(true);
    setError(null);
    try {
      const nextSnapshot = await client.refresh();
      if (mountedRef.current) {
        snapshotRef.current = nextSnapshot;
        setSnapshot(nextSnapshot);
        queryScheduler.schedule(queryRef.current);
        await queryScheduler.flush();
      }
    } catch (refreshError) {
      if (mountedRef.current) setError(normalizeQuickLauncherError(refreshError));
    } finally {
      if (mountedRef.current) setIsRefreshing(false);
    }
  }, [client, isRefreshing, queryScheduler]);

  const activate = useCallback(async (requestedId?: string) => {
    await queryScheduler.flush();
    const latest = latestResultRef.current;
    const target = resolveActivationAfterFlush(requestedId, selectedIdRef.current, latest);
    if (!target || activationRef.current
      || !canActivateLauncherItem(target.itemId, latest?.items ?? [], activatingId)) return null;

    activationRef.current = true;
    setActivatingId(target.itemId);
    setError(null);
    try {
      const result = await client.activate(target.itemId, target.revision);
      if (mountedRef.current) {
        setLastAction(result);
        await runSearch(queryRef.current);
      }
      return result;
    } catch (activationError) {
      if (mountedRef.current) setError(normalizeQuickLauncherError(activationError));
      return null;
    } finally {
      activationRef.current = false;
      if (mountedRef.current) setActivatingId(null);
    }
  }, [activatingId, client, queryScheduler, runSearch]);

  const moveSelection = useCallback((direction: -1 | 1) => {
    setSelectedIdState((current) => {
      const next = moveLauncherSelection(current, items, direction);
      selectedIdRef.current = next;
      return next;
    });
  }, [items]);

  const resetTransient = useCallback(() => {
    queryGenerationRef.current += 1;
    queryScheduler.cancelPending();
    queryRef.current = "";
    setQueryState("");
    selectedIdRef.current = null;
    setSelectedIdState(null);
    setError(null);
    setLastAction(null);
    if (activityRef.current === "active" && snapshotRef.current) {
      queryScheduler.schedule("");
    }
  }, [queryScheduler]);

  return useMemo(() => ({
    snapshot,
    query,
    setQuery,
    revision,
    items,
    selectedId,
    setSelectedId,
    icons,
    isLoading,
    isRefreshing,
    activatingId,
    error,
    lastAction,
    refresh,
    activate,
    moveSelection,
    resetTransient,
  }), [
    snapshot, query, setQuery, revision, items, selectedId, setSelectedId, icons,
    isLoading, isRefreshing, activatingId, error, lastAction, refresh, activate,
    moveSelection, resetTransient,
  ]);
}

export type QuickLauncherController = ReturnType<typeof useQuickLauncher>;
