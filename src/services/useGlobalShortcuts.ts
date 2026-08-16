import { useCallback, useEffect, useState } from "react";
import {
  globalShortcutService,
  type GlobalShortcutSnapshot,
} from "./globalShortcuts";

export interface GlobalShortcutServiceApi {
  getSnapshots(): Promise<GlobalShortcutSnapshot[]>;
}

export function useGlobalShortcuts(
  refreshKey = "",
  service: GlobalShortcutServiceApi = globalShortcutService,
) {
  const [snapshots, setSnapshots] = useState<GlobalShortcutSnapshot[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const next = await service.getSnapshots();
      setSnapshots(next);
      return next;
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : String(loadError));
      throw loadError;
    } finally {
      setIsLoading(false);
    }
  }, [refreshKey, service]);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    service
      .getSnapshots()
      .then((next) => {
        if (!cancelled) {
          setSnapshots(next);
          setError(null);
        }
      })
      .catch((loadError) => {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : String(loadError));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [service]);

  return { snapshots, isLoading, error, reload };
}
