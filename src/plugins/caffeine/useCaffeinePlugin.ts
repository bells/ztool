import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSurfaceActivity } from "../../core/windowing/useSurfaceActivity";
import {
  CaffeineDurationMinutes,
  formatDurationClock,
  getRemainingMs,
} from "./caffeineDuration";
import {
  shouldRefreshExpiredCaffeine,
  startCaffeinePresentationClock,
} from "./caffeinePresentation";

interface CaffeineSnapshot {
  enabled: boolean;
  started_at_ms: number | null;
  duration_minutes: CaffeineDurationMinutes;
  expires_at_ms: number | null;
  message: string;
}

export function useCaffeinePlugin() {
  const surfaceActivity = useSurfaceActivity();
  const [snapshot, setSnapshot] = useState<CaffeineSnapshot>({
    enabled: false,
    started_at_ms: null,
    duration_minutes: null,
    expires_at_ms: null,
    message: "正在读取状态",
  });
  const [selectedDurationMinutes, setSelectedDurationMinutes] =
    useState<CaffeineDurationMinutes>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [now, setNow] = useState(Date.now());
  const [error, setError] = useState<string | null>(null);
  const [authoritativeSnapshotReady, setAuthoritativeSnapshotReady] =
    useState(false);

  const refresh = useCallback(async () => {
    const next = await invoke<CaffeineSnapshot>("get_caffeine_state");
    setSnapshot(next);
    if (next.enabled) {
      setSelectedDurationMinutes(next.duration_minutes);
    }
    setError(null);
  }, []);

  const setKeepAwake = useCallback(async (
    enabled: boolean,
    durationMinutes: CaffeineDurationMinutes,
  ) => {
    if (isBusy) return;

    setIsBusy(true);
    try {
      const next = await invoke<CaffeineSnapshot>("toggle_keep_awake", {
        enabled,
        durationMinutes,
      });
      setSnapshot(next);
      if (next.enabled) {
        setSelectedDurationMinutes(next.duration_minutes);
      }
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsBusy(false);
    }
  }, [isBusy]);

  const enable = useCallback(() => {
    setKeepAwake(true, selectedDurationMinutes);
  }, [selectedDurationMinutes, setKeepAwake]);

  const disable = useCallback(() => {
    setKeepAwake(false, null);
  }, [setKeepAwake]);

  const selectDuration = useCallback(
    (durationMinutes: CaffeineDurationMinutes) => {
      setSelectedDurationMinutes(durationMinutes);
      if (snapshot.enabled) {
        setKeepAwake(true, durationMinutes);
      }
    },
    [setKeepAwake, snapshot.enabled],
  );

  useEffect(() => {
    if (surfaceActivity !== "active") {
      setAuthoritativeSnapshotReady(false);
      return;
    }

    let live = true;
    setAuthoritativeSnapshotReady(false);
    refresh()
      .then(() => {
        if (live) setAuthoritativeSnapshotReady(true);
      })
      .catch((err) => {
        if (live) setError(String(err));
      });
    return () => {
      live = false;
    };
  }, [refresh, surfaceActivity]);

  const presentationGate = {
    enabled: snapshot.enabled,
    surfaceActivity,
    authoritativeSnapshotReady,
  };

  useEffect(() => startCaffeinePresentationClock(
    presentationGate,
    {
      now: Date.now,
      setInterval: (callback, delayMs) => window.setInterval(callback, delayMs),
      clearInterval: (timer) => window.clearInterval(timer as number),
    },
    setNow,
  ), [
    authoritativeSnapshotReady,
    snapshot.enabled,
    surfaceActivity,
  ]);

  const remainingMs = useMemo(
    () => getRemainingMs(snapshot.expires_at_ms, now),
    [now, snapshot.expires_at_ms],
  );

  useEffect(() => {
    if (!shouldRefreshExpiredCaffeine(
      presentationGate,
      snapshot.expires_at_ms,
      remainingMs,
    )) {
      return;
    }

    refresh().catch((err) => setError(String(err)));
  }, [
    authoritativeSnapshotReady,
    refresh,
    remainingMs,
    snapshot.enabled,
    snapshot.expires_at_ms,
    surfaceActivity,
  ]);

  const elapsed = useMemo(() => {
    if (!snapshot.enabled || !snapshot.started_at_ms) return "00:00";

    return formatDurationClock(now - snapshot.started_at_ms);
  }, [now, snapshot.enabled, snapshot.started_at_ms]);

  const remaining = useMemo(
    () => (remainingMs === null ? null : formatDurationClock(remainingMs)),
    [remainingMs],
  );

  return {
    enabled: snapshot.enabled,
    durationMinutes: snapshot.duration_minutes,
    error,
    elapsed,
    remaining,
    selectedDurationMinutes,
    isBusy,
    enable,
    disable,
    selectDuration,
    refresh,
  };
}
