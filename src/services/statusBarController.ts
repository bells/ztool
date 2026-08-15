import type { PluginRecord } from "../core/pluginHost/contracts";
import {
  createStatusBarPreview,
  getStatusBarFallbackItems,
  resolveStatusBarPreferenceItems,
  type StatusBarItemSnapshot,
  type StatusBarPreferenceItem,
  type StatusBarSettingsSnapshot,
  type UpdateStatusBarSettingsInput,
} from "./statusBarModel.js";

export type StatusBarMessageKey =
  | "statusBar.message.ready"
  | "statusBar.message.loading"
  | "statusBar.message.saving"
  | "statusBar.message.error";

export interface StatusBarUiStateInput {
  records: PluginRecord[];
  settings: StatusBarSettingsSnapshot;
  items: StatusBarItemSnapshot[];
  isLoading: boolean;
  isBusy: boolean;
  error: string | null;
}

export interface StatusBarUiState {
  previewItems: StatusBarItemSnapshot[];
  preferenceItems: StatusBarPreferenceItem[];
  fallbackItems: StatusBarItemSnapshot[];
  messageKey: StatusBarMessageKey;
  messageDetail: string | null;
}

export function applyStatusBarSettingsUpdate(
  settings: StatusBarSettingsSnapshot,
  input: UpdateStatusBarSettingsInput,
): StatusBarSettingsSnapshot {
  return {
    enabled: input.enabled ?? settings.enabled,
    showPluginItemsOnLaunch:
      input.showPluginItemsOnLaunch ?? settings.showPluginItemsOnLaunch,
    pluginItemsCollapsed:
      input.pluginItemsCollapsed ?? settings.pluginItemsCollapsed,
    visiblePluginItems: {
      ...settings.visiblePluginItems,
      ...(input.visiblePluginItems ?? {}),
    },
  };
}

export function statusBarPluginVisibilityInput(
  pluginName: string,
  visible: boolean,
): UpdateStatusBarSettingsInput {
  return {
    visiblePluginItems: {
      [pluginName]: visible,
    },
  };
}

export function createStatusBarUiState({
  records,
  settings,
  items,
  isLoading,
  isBusy,
  error,
}: StatusBarUiStateInput): StatusBarUiState {
  return {
    previewItems: createStatusBarPreview(items),
    preferenceItems: resolveStatusBarPreferenceItems({ records, settings }),
    fallbackItems: getStatusBarFallbackItems(items),
    messageKey: resolveStatusBarMessageKey({ isLoading, isBusy, error }),
    messageDetail: error,
  };
}

function resolveStatusBarMessageKey({
  isLoading,
  isBusy,
  error,
}: {
  isLoading: boolean;
  isBusy: boolean;
  error: string | null;
}): StatusBarMessageKey {
  if (error) {
    return "statusBar.message.error";
  }

  if (isLoading) {
    return "statusBar.message.loading";
  }

  if (isBusy) {
    return "statusBar.message.saving";
  }

  return "statusBar.message.ready";
}
