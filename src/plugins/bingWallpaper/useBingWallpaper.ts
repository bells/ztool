import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSurfaceActivity } from "../../core/windowing/useSurfaceActivity";

import {
  createActionGate,
  createRequestGate,
  errorMessage,
  loadBingWallpaperCacheFirst,
  nextBingWallpaperReloadVersion,
  previewBytesMatchDescriptor,
  shouldStartBingWallpaperPresentation,
} from "./bingWallpaperController";
import {
  createBingWallpaperNavigation,
  resolveBingWallpaperSelection,
  selectNewerBingWallpaper,
  selectOlderBingWallpaper,
  sortBingWallpapers,
} from "./bingWallpaperModel";
import { bingWallpaperService } from "./bingWallpaperService";
import type { BingWallpaperService } from "./bingWallpaperServiceCore";
import type {
  BingWallpaperActionResult,
  BingWallpaperSnapshot,
} from "./contracts";

export type BingWallpaperActionStatus = "applied" | "saved" | null;

interface BingWallpaperPreviewView {
  wallpaperId: string;
  resourceUrl: string;
}

export function useBingWallpaper(
  service: BingWallpaperService = bingWallpaperService,
) {
  const activity = useSurfaceActivity();
  const [snapshot, setSnapshot] = useState<BingWallpaperSnapshot | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [preview, setPreview] = useState<BingWallpaperPreviewView | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isPreviewLoading, setIsPreviewLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionStatus, setActionStatus] = useState<BingWallpaperActionStatus>(null);
  const [actionResult, setActionResult] = useState<BingWallpaperActionResult | null>(null);
  const [reloadVersion, setReloadVersion] = useState(0);
  const mountedRef = useRef(true);
  const actionGateRef = useRef(createActionGate());

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!shouldStartBingWallpaperPresentation(activity)) {
      setIsRefreshing(false);
      return;
    }
    const gate = createRequestGate();
    let snapshotCount = 0;
    setIsLoading(true);
    setIsRefreshing(false);
    setError(null);

    void loadBingWallpaperCacheFirst(
      service,
      gate,
      (nextSnapshot) => {
        snapshotCount += 1;
        const sortedItems = sortBingWallpapers(nextSnapshot.items);
        setSnapshot({ ...nextSnapshot, items: sortedItems });
        setSelectedId((current) =>
          resolveBingWallpaperSelection(sortedItems, current));
        setIsLoading(false);
        setIsRefreshing(snapshotCount === 1);
        if (snapshotCount > 1) {
          setIsRefreshing(false);
        }
      },
      (message) => {
        setError(message);
        setIsLoading(false);
        setIsRefreshing(false);
      },
    );

    return gate.dispose;
  }, [activity, reloadVersion, service]);

  const navigation = useMemo(
    () => createBingWallpaperNavigation(snapshot?.items ?? [], selectedId),
    [selectedId, snapshot?.items],
  );

  useEffect(() => {
    const selected = navigation.selected;
    if (!shouldStartBingWallpaperPresentation(activity) || !selected) {
      setPreview(null);
      setIsPreviewLoading(false);
      return;
    }

    const gate = createRequestGate();
    let ownedToken: string | null = null;
    let ownedResourceUrl: string | null = null;
    const releaseOwnedPreview = () => {
      if (ownedResourceUrl) {
        URL.revokeObjectURL(ownedResourceUrl);
        ownedResourceUrl = null;
      }
      if (ownedToken) {
        void service.releasePreview({ token: ownedToken }).catch(() => undefined);
        ownedToken = null;
      }
    };
    setPreview((current) =>
      current?.wallpaperId === selected.id ? current : null);
    setIsPreviewLoading(true);

    void service.preview({ wallpaperId: selected.id })
      .then(async (descriptor) => {
        ownedToken = descriptor.token;
        if (!gate.isCurrent()) {
          releaseOwnedPreview();
          return;
        }
        const buffer = await service.readPreview({ token: descriptor.token });
        if (!gate.isCurrent()) {
          releaseOwnedPreview();
          return;
        }
        const bytes = new Uint8Array(buffer);
        if (!previewBytesMatchDescriptor(bytes.byteLength, descriptor.byteLength)) {
          throw new Error("Wallpaper preview bytes did not match their descriptor.");
        }
        ownedResourceUrl = URL.createObjectURL(
          new Blob([bytes], { type: descriptor.mimeType }),
        );
        setPreview({ wallpaperId: descriptor.wallpaperId, resourceUrl: ownedResourceUrl });
        setError(null);
      })
      .catch((previewError: unknown) => {
        releaseOwnedPreview();
        if (gate.isCurrent()) {
          setPreview(null);
          setError(errorMessage(previewError));
        }
      })
      .finally(() => {
        if (gate.isCurrent()) {
          setIsPreviewLoading(false);
        }
      });

    return () => {
      gate.dispose();
      releaseOwnedPreview();
    };
  }, [activity, navigation.selected?.id, service]);

  const selectOlder = useCallback(() => {
    setSelectedId((current) =>
      selectOlderBingWallpaper(snapshot?.items ?? [], current));
    setActionStatus(null);
  }, [snapshot?.items]);

  const selectNewer = useCallback(() => {
    setSelectedId((current) =>
      selectNewerBingWallpaper(snapshot?.items ?? [], current));
    setActionStatus(null);
  }, [snapshot?.items]);

  const apply = useCallback(async () => {
    const selected = navigation.selected;
    if (!selected || !actionGateRef.current.tryStart("apply")) {
      return null;
    }
    setIsApplying(true);
    setError(null);
    setActionStatus(null);
    try {
      const result = await service.apply({ wallpaperId: selected.id });
      if (mountedRef.current) {
        setActionResult(result);
        setActionStatus("applied");
      }
      return result;
    } catch (applyError) {
      if (mountedRef.current) {
        setError(errorMessage(applyError));
      }
      return null;
    } finally {
      actionGateRef.current.finish("apply");
      if (mountedRef.current) {
        setIsApplying(false);
      }
    }
  }, [navigation.selected, service]);

  const save = useCallback(async () => {
    const selected = navigation.selected;
    if (!selected || !actionGateRef.current.tryStart("save")) {
      return null;
    }
    setIsSaving(true);
    setError(null);
    setActionStatus(null);
    try {
      const result = await service.save({ wallpaperId: selected.id });
      if (mountedRef.current) {
        setActionResult(result);
        setActionStatus("saved");
      }
      return result;
    } catch (saveError) {
      if (mountedRef.current) {
        setError(errorMessage(saveError));
      }
      return null;
    } finally {
      actionGateRef.current.finish("save");
      if (mountedRef.current) {
        setIsSaving(false);
      }
    }
  }, [navigation.selected, service]);

  const retry = useCallback(() => {
    setReloadVersion(nextBingWallpaperReloadVersion);
  }, []);

  return {
    snapshot,
    navigation,
    preview,
    isLoading,
    isRefreshing,
    isPreviewLoading,
    isSaving,
    isApplying,
    error,
    actionStatus,
    actionResult,
    selectOlder,
    selectNewer,
    apply,
    save,
    retry,
  };
}
