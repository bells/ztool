export interface QueryTimeoutScheduler {
  setTimeout(callback: () => void, delayMs: number): unknown;
  clearTimeout(timer: unknown): void;
}

export interface QuerySchedulingMetrics {
  scheduled: number;
  executed: number;
  superseded: number;
  maxConcurrent: number;
}

export interface LatestQueryScheduler {
  schedule(query: string): void;
  flush(): Promise<void>;
  cancelPending(): void;
  dispose(): void;
  metrics(): QuerySchedulingMetrics;
}

export function mergeBoundedIconBatch(
  current: Record<string, string | null>,
  results: Array<{ itemId: string; dataUrl?: string }>,
  limit: number,
): Record<string, string | null> {
  const next = { ...current };
  for (const result of results) {
    delete next[result.itemId];
    next[result.itemId] = result.dataUrl ?? null;
  }
  const keys = Object.keys(next);
  for (let index = 0; index < keys.length - limit; index += 1) {
    delete next[keys[index]];
  }
  return next;
}

export function resolveActivationAfterFlush(
  requestedId: string | undefined,
  selectedId: string | null,
  result: { revision: number; items: Array<{ id: string }> } | null,
): { itemId: string; revision: number } | null {
  const itemId = requestedId ?? selectedId;
  if (!itemId || !result?.items.some((item) => item.id === itemId)) return null;
  return { itemId, revision: result.revision };
}

export function acceptsPresentationCompletion(
  requestGeneration: number,
  currentGeneration: number,
  activity: "active" | "hidden" | "disposed",
): boolean {
  return activity === "active" && requestGeneration === currentGeneration;
}

export function createLatestQueryScheduler(
  execute: (query: string) => Promise<unknown>,
  scheduler: QueryTimeoutScheduler,
  delayMs: number,
): LatestQueryScheduler {
  let pending: string | null = null;
  let timer: unknown;
  let inFlight: Promise<void> | null = null;
  let disposed = false;
  let concurrent = 0;
  const metrics: QuerySchedulingMetrics = {
    scheduled: 0,
    executed: 0,
    superseded: 0,
    maxConcurrent: 0,
  };

  const clearTimer = () => {
    if (timer === undefined) return;
    scheduler.clearTimeout(timer);
    timer = undefined;
  };

  const arm = () => {
    clearTimer();
    if (disposed || pending === null || inFlight) return;
    timer = scheduler.setTimeout(() => {
      timer = undefined;
      void runPending();
    }, delayMs);
  };

  const runPending = (): Promise<void> => {
    if (disposed || pending === null) return Promise.resolve();
    if (inFlight) return inFlight;
    clearTimer();
    const query = pending;
    pending = null;
    metrics.executed += 1;
    concurrent += 1;
    metrics.maxConcurrent = Math.max(metrics.maxConcurrent, concurrent);
    inFlight = execute(query)
      .then(() => undefined)
      .finally(() => {
        concurrent -= 1;
        inFlight = null;
        arm();
      });
    return inFlight;
  };

  return {
    schedule(query) {
      if (disposed) return;
      metrics.scheduled += 1;
      if (pending !== null) metrics.superseded += 1;
      pending = query;
      arm();
    },
    async flush() {
      clearTimer();
      while (!disposed && (pending !== null || inFlight)) {
        if (inFlight) await inFlight;
        else await runPending();
        clearTimer();
      }
    },
    cancelPending() {
      clearTimer();
      pending = null;
    },
    dispose() {
      disposed = true;
      clearTimer();
      pending = null;
    },
    metrics: () => ({ ...metrics }),
  };
}
