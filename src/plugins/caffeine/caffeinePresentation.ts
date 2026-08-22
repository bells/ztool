import type { SurfaceActivityState } from "../../core/windowing/surfaceActivityCore";

export interface CaffeinePresentationScheduler {
  now(): number;
  setInterval(callback: () => void, delayMs: number): unknown;
  clearInterval(timer: unknown): void;
}

export interface CaffeinePresentationGate {
  enabled: boolean;
  surfaceActivity: SurfaceActivityState;
  authoritativeSnapshotReady: boolean;
}

export function shouldRunCaffeinePresentationClock(
  gate: CaffeinePresentationGate,
): boolean {
  return gate.enabled
    && gate.surfaceActivity === "active"
    && gate.authoritativeSnapshotReady;
}

export function shouldRefreshExpiredCaffeine(
  gate: CaffeinePresentationGate,
  expiresAtMs: number | null,
  remainingMs: number | null,
): boolean {
  return shouldRunCaffeinePresentationClock(gate)
    && expiresAtMs !== null
    && remainingMs === 0;
}

export function startCaffeinePresentationClock(
  gate: CaffeinePresentationGate,
  scheduler: CaffeinePresentationScheduler,
  onTick: (now: number) => void,
): () => void {
  if (!shouldRunCaffeinePresentationClock(gate)) return () => undefined;

  onTick(scheduler.now());
  const timer = scheduler.setInterval(() => onTick(scheduler.now()), 1000);
  return () => scheduler.clearInterval(timer);
}
