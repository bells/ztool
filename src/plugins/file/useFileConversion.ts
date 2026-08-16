import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  FileConversionBatchResult,
  FileConversionCandidate,
  FileConversionCapabilitySnapshot,
  FileConversionEnqueueItem,
  FileConversionError,
  FileConversionJobSnapshot,
} from "./contracts";
import {
  mergeFileConversionJob,
  reconcileFileConversionCandidates,
  reconcileInitialFileConversionJobs,
} from "./fileModel";
import {
  fileService,
  normalizeFileConversionError,
} from "./fileService";

export type FileConversionAction =
  | "choose"
  | "inspect"
  | "enqueue"
  | "start"
  | "cancel"
  | "remove"
  | "retry"
  | "clear"
  | "open"
  | "reveal";

export interface FileConversionActionError {
  action: FileConversionAction;
  owner: string | true;
  error: FileConversionError;
}

export interface FileConversionClient {
  getCapabilities(): Promise<FileConversionCapabilitySnapshot>;
  chooseInputs(): Promise<FileConversionCandidate[]>;
  inspectInputs(sourcePaths: string[]): Promise<FileConversionCandidate[]>;
  enqueue(items: FileConversionEnqueueItem[]): Promise<FileConversionBatchResult>;
  listJobs(): Promise<FileConversionJobSnapshot[]>;
  start(): Promise<FileConversionJobSnapshot[]>;
  cancel(jobId: string): Promise<FileConversionJobSnapshot[]>;
  remove(jobId: string): Promise<FileConversionJobSnapshot>;
  retry(jobId: string): Promise<FileConversionJobSnapshot>;
  clearCompleted(): Promise<FileConversionJobSnapshot[]>;
  open(jobId: string): Promise<void>;
  reveal(jobId: string): Promise<void>;
  subscribe(
    handler: (snapshot: FileConversionJobSnapshot) => void,
  ): Promise<() => void>;
}

export function useFileConversion(
  client: FileConversionClient = fileService,
) {
  const [capabilities, setCapabilities] =
    useState<FileConversionCapabilitySnapshot | null>(null);
  const [candidates, setCandidates] = useState<FileConversionCandidate[]>([]);
  const [jobs, setJobs] = useState<FileConversionJobSnapshot[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [busy, setBusy] = useState<Partial<Record<FileConversionAction, string | true>>>({});
  const [error, setError] = useState<FileConversionError | null>(null);
  const [actionError, setActionError] =
    useState<FileConversionActionError | null>(null);
  const mountedRef = useRef(false);
  const actionGateRef = useRef(new Set<FileConversionAction>());

  useEffect(() => {
    mountedRef.current = true;
    let hydrated = false;
    let bufferedEvents: FileConversionJobSnapshot[] = [];
    let disposed = false;
    let unlisten: (() => void) | undefined;

    client
      .subscribe((snapshot) => {
        if (disposed) return;
        if (!hydrated) {
          bufferedEvents.push(snapshot);
          return;
        }
        setJobs((current) => mergeFileConversionJob(current, snapshot));
      })
      .then((disposeListener) => {
        if (disposed) {
          disposeListener();
          return;
        }
        unlisten = disposeListener;
        return Promise.all([client.getCapabilities(), client.listJobs()]);
      })
      .then((loaded) => {
        if (!loaded || disposed) return;
        const [nextCapabilities, snapshot] = loaded;
        setCapabilities(nextCapabilities);
        setJobs(reconcileInitialFileConversionJobs(snapshot, bufferedEvents));
        bufferedEvents = [];
        hydrated = true;
        setError(null);
      })
      .catch((loadError: unknown) => {
        if (!disposed) setError(normalizeFileConversionError(loadError));
      })
      .finally(() => {
        if (!disposed) setIsLoading(false);
      });

    return () => {
      disposed = true;
      mountedRef.current = false;
      actionGateRef.current.clear();
      unlisten?.();
    };
  }, [client]);

  const runAction = useCallback(
    async <T,>(
      action: FileConversionAction,
      owner: string | true,
      operation: () => Promise<T>,
    ): Promise<T | null> => {
      if (actionGateRef.current.has(action)) return null;
      actionGateRef.current.add(action);
      setBusy((current) => ({ ...current, [action]: owner }));
      setError(null);
      setActionError(null);
      try {
        return await operation();
      } catch (actionError) {
        if (mountedRef.current) {
          const normalized = normalizeFileConversionError(actionError);
          setError(normalized);
          setActionError({ action, owner, error: normalized });
        }
        return null;
      } finally {
        actionGateRef.current.delete(action);
        if (mountedRef.current) {
          setBusy((current) => {
            const next = { ...current };
            delete next[action];
            return next;
          });
        }
      }
    },
    [],
  );

  const choose = useCallback(async () => {
    const result = await runAction("choose", true, () => client.chooseInputs());
    if (result && mountedRef.current) {
      setCandidates((current) =>
        reconcileFileConversionCandidates(current, result),
      );
    }
    return result;
  }, [client, runAction]);

  const inspect = useCallback(
    async (sourcePaths: string[]) => {
      const result = await runAction("inspect", true, () =>
        client.inspectInputs(sourcePaths),
      );
      if (result && mountedRef.current) {
        setCandidates((current) =>
          reconcileFileConversionCandidates(current, result),
        );
      }
      return result;
    },
    [client, runAction],
  );

  const enqueue = useCallback(
    async (items: FileConversionEnqueueItem[]) => {
      const result = await runAction("enqueue", true, () => client.enqueue(items));
      if (result && mountedRef.current) {
        setJobs((current) =>
          result.jobs.reduce(mergeFileConversionJob, current),
        );
        setCandidates(result.rejectedCandidates);
      }
      return result;
    },
    [client, runAction],
  );

  const start = useCallback(async () => {
    const result = await runAction("start", true, () => client.start());
    if (result && mountedRef.current) {
      setJobs((current) => result.reduce(mergeFileConversionJob, current));
    }
    return result;
  }, [client, runAction]);

  const cancel = useCallback(
    async (jobId: string) => {
      const result = await runAction("cancel", jobId, () => client.cancel(jobId));
      if (result && mountedRef.current) {
        setJobs((current) => result.reduce(mergeFileConversionJob, current));
      }
      return result;
    },
    [client, runAction],
  );

  const remove = useCallback(
    async (jobId: string) => {
      const result = await runAction("remove", jobId, () => client.remove(jobId));
      if (result && mountedRef.current) {
        setJobs((current) => current.filter((job) => job.id !== jobId));
      }
      return result;
    },
    [client, runAction],
  );

  const retry = useCallback(
    async (jobId: string) => {
      const result = await runAction("retry", jobId, () => client.retry(jobId));
      if (result && mountedRef.current) {
        setJobs((current) => mergeFileConversionJob(current, result));
      }
      return result;
    },
    [client, runAction],
  );

  const clearCompleted = useCallback(async () => {
    const removed = await runAction("clear", true, () => client.clearCompleted());
    if (removed && mountedRef.current) {
      const removedIds = new Set(removed.map((job) => job.id));
      setJobs((current) => current.filter((job) => !removedIds.has(job.id)));
    }
    return removed;
  }, [client, runAction]);

  const open = useCallback(
    (jobId: string) => runAction("open", jobId, () => client.open(jobId)),
    [client, runAction],
  );
  const reveal = useCallback(
    (jobId: string) => runAction("reveal", jobId, () => client.reveal(jobId)),
    [client, runAction],
  );

  return useMemo(
    () => ({
      capabilities,
      candidates,
      setCandidates,
      jobs,
      isLoading,
      busy,
      error,
      actionError,
      choose,
      inspect,
      enqueue,
      start,
      cancel,
      remove,
      retry,
      clearCompleted,
      open,
      reveal,
    }),
    [
      capabilities,
      candidates,
      jobs,
      isLoading,
      busy,
      error,
      actionError,
      choose,
      inspect,
      enqueue,
      start,
      cancel,
      remove,
      retry,
      clearCompleted,
      open,
      reveal,
    ],
  );
}

export type FileConversionController = ReturnType<typeof useFileConversion>;
