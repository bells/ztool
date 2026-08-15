import { useCallback, useEffect, useMemo, useState } from "react";
import type { PluginRecord } from "../core/pluginHost/contracts";
import {
  applyStatusBarSettingsUpdate,
  createStatusBarUiState,
  statusBarPluginVisibilityInput,
} from "./statusBarController";
import { statusBarService } from "./statusBar";
import {
  DEFAULT_STATUS_BAR_SETTINGS,
  type StatusBarItemSnapshot,
  type StatusBarSettingsSnapshot,
  type UpdateStatusBarSettingsInput,
} from "./statusBarModel";

export interface StatusBarServiceApi {
  getSettings(): Promise<StatusBarSettingsSnapshot>;
  updateSettings(input: UpdateStatusBarSettingsInput): Promise<StatusBarSettingsSnapshot>;
  getItems(): Promise<StatusBarItemSnapshot[]>;
  runItemAction(input: { itemId: string }): Promise<void>;
}

export function useStatusBar(
  records: PluginRecord[],
  service: StatusBarServiceApi = statusBarService,
) {
  const [settings, setSettings] = useState<StatusBarSettingsSnapshot>(
    DEFAULT_STATUS_BAR_SETTINGS,
  );
  const [items, setItems] = useState<StatusBarItemSnapshot[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const recordsKey = useMemo(() => statusBarRecordsKey(records), [records]);

  const load = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [nextSettings, nextItems] = await Promise.all([
        service.getSettings(),
        service.getItems(),
      ]);
      setSettings(nextSettings);
      setItems(nextItems);
      return {
        settings: nextSettings,
        items: nextItems,
      };
    } catch (loadError) {
      setError(formatStatusBarError(loadError));
      throw loadError;
    } finally {
      setIsLoading(false);
    }
  }, [service]);

  useEffect(() => {
    let cancelled = false;

    setIsLoading(true);
    setError(null);
    Promise.all([service.getSettings(), service.getItems()])
      .then(([nextSettings, nextItems]) => {
        if (!cancelled) {
          setSettings(nextSettings);
          setItems(nextItems);
        }
      })
      .catch((loadError) => {
        if (!cancelled) {
          setError(formatStatusBarError(loadError));
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
  }, [recordsKey, service]);

  const updateSettings = useCallback(
    async (input: UpdateStatusBarSettingsInput) => {
      const previousSettings = settings;
      setIsBusy(true);
      setError(null);
      setSettings(applyStatusBarSettingsUpdate(settings, input));
      try {
        const savedSettings = await service.updateSettings(input);
        const nextItems = await service.getItems();
        setSettings(savedSettings);
        setItems(nextItems);
        return savedSettings;
      } catch (updateError) {
        setSettings(previousSettings);
        setError(formatStatusBarError(updateError));
        throw updateError;
      } finally {
        setIsBusy(false);
      }
    },
    [service, settings],
  );

  const runItemAction = useCallback(
    async (itemId: string) => {
      setIsBusy(true);
      setError(null);
      try {
        await service.runItemAction({ itemId });
        setItems(await service.getItems());
      } catch (actionError) {
        setError(formatStatusBarError(actionError));
        throw actionError;
      } finally {
        setIsBusy(false);
      }
    },
    [service],
  );

  const uiState = useMemo(
    () =>
      createStatusBarUiState({
        records,
        settings,
        items,
        isLoading,
        isBusy,
        error,
      }),
    [records, settings, items, isLoading, isBusy, error],
  );

  return {
    settings,
    items,
    isLoading,
    isBusy,
    error,
    ...uiState,
    reload: load,
    updateSettings,
    runItemAction,
    setEnabled: (enabled: boolean) => updateSettings({ enabled }),
    setShowPluginItemsOnLaunch: (showPluginItemsOnLaunch: boolean) =>
      updateSettings({ showPluginItemsOnLaunch }),
    setPluginItemVisible: (pluginName: string, visible: boolean) =>
      updateSettings(statusBarPluginVisibilityInput(pluginName, visible)),
  };
}

function statusBarRecordsKey(records: PluginRecord[]) {
  return records
    .map((record) => {
      const statusItems = record.manifest.contributes?.statusBarItems
        ?.map((item) => `${item.id}:${item.icon}:${item.action.type}`)
        .join(",");
      return `${record.name}:${record.enabled}:${record.health}:${statusItems ?? ""}`;
    })
    .join("\u0000");
}

function formatStatusBarError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export type StatusBarController = ReturnType<typeof useStatusBar>;
