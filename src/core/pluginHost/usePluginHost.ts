import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  PluginIdentityInput,
  PluginLifecycleResult,
  PluginRecord,
  SetPluginEnabledInput,
} from "./contracts";
import {
  createPluginHostState,
  pluginHostActionFailed,
  type PluginHostState,
} from "./pluginHostModel";
import { pluginHostService } from "./pluginHostService";

export interface PluginHostLifecycleService {
  listPlugins(): Promise<PluginRecord[]>;
  uninstallPlugin(input: PluginIdentityInput): Promise<PluginLifecycleResult>;
  setPluginEnabled(input: SetPluginEnabledInput): Promise<PluginRecord>;
  restoreBundledPlugins(): Promise<PluginRecord[]>;
}

export function usePluginHost(
  service: PluginHostLifecycleService = pluginHostService,
) {
  const [records, setRecords] = useState<PluginRecord[]>([]);
  const [selectedPluginName, setSelectedPluginName] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const derivedState = useMemo(
    () => createPluginHostState(records, selectedPluginName),
    [records, selectedPluginName],
  );

  const hostState: PluginHostState = {
    ...derivedState,
    isLoading,
    isBusy,
    error,
  };

  const loadPlugins = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const nextRecords = await service.listPlugins();
      setRecords(nextRecords);
      return nextRecords;
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : String(loadError));
      throw loadError;
    } finally {
      setIsLoading(false);
    }
  }, [service]);

  useEffect(() => {
    loadPlugins().catch(() => {
      // Error state is already captured for the shell; keep host surfaces mounted.
    });
  }, [loadPlugins]);

  useEffect(() => {
    if (derivedState.activePlugin?.name && derivedState.activePlugin.name !== selectedPluginName) {
      setSelectedPluginName(derivedState.activePlugin.name);
    }

    if (!derivedState.activePlugin && selectedPluginName) {
      setSelectedPluginName(null);
    }
  }, [derivedState.activePlugin, selectedPluginName]);

  const runLifecycleAction = useCallback(
    async <T,>(action: () => Promise<T>) => {
      setIsBusy(true);
      setError(null);
      try {
        const result = await action();
        const nextRecords = await service.listPlugins();
        setRecords(nextRecords);
        return result;
      } catch (actionError) {
        const failed = pluginHostActionFailed(hostState, actionError);
        setError(failed.error);
        throw actionError;
      } finally {
        setIsBusy(false);
      }
    },
    [hostState, service],
  );

  const setPluginEnabled = useCallback(
    (input: SetPluginEnabledInput) =>
      runLifecycleAction(() => service.setPluginEnabled(input)),
    [runLifecycleAction, service],
  );

  const uninstallPlugin = useCallback(
    (input: PluginIdentityInput) =>
      runLifecycleAction(() => service.uninstallPlugin(input)),
    [runLifecycleAction, service],
  );

  const restoreBundledPlugins = useCallback(
    () => runLifecycleAction(() => service.restoreBundledPlugins()),
    [runLifecycleAction, service],
  );

  return {
    ...hostState,
    reload: loadPlugins,
    setSelectedPluginName,
    setPluginEnabled,
    uninstallPlugin,
    restoreBundledPlugins,
  };
}

export type PluginHostController = ReturnType<typeof usePluginHost>;
