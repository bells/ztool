import type {
  QuickLauncherActivateInput,
  QuickLauncherActivationResult,
  QuickLauncherError,
  QuickLauncherIconInput,
  QuickLauncherIconBatchInput,
  QuickLauncherIconBatchResult,
  QuickLauncherIconResult,
  QuickLauncherIndexSnapshot,
  QuickLauncherRunningSnapshot,
  QuickLauncherSearchInput,
  QuickLauncherSearchResult,
} from "./contracts";

export const QUICK_LAUNCHER_COMMANDS = {
  snapshot: "get_quick_launcher_snapshot",
  refresh: "refresh_quick_launcher_index",
  search: "search_quick_launcher",
  icon: "get_quick_launcher_icon",
  icons: "get_quick_launcher_icons",
  refreshRunning: "refresh_quick_launcher_running_state",
  activate: "activate_quick_launcher_item",
  showWindow: "show_quick_launcher_window",
  hideWindow: "hide_quick_launcher_window",
} as const;

export type QuickLauncherCommand =
  (typeof QUICK_LAUNCHER_COMMANDS)[keyof typeof QUICK_LAUNCHER_COMMANDS];
export type QuickLauncherInvokeArgs =
  | { input: QuickLauncherSearchInput }
  | { input: QuickLauncherIconInput }
  | { input: QuickLauncherIconBatchInput }
  | { input: QuickLauncherActivateInput };

export interface QuickLauncherInvokeBridge {
  <T>(command: QuickLauncherCommand, args?: QuickLauncherInvokeArgs): Promise<T>;
}

export function createQuickLauncherService(invokeBridge: QuickLauncherInvokeBridge) {
  return {
    getSnapshot: () =>
      invokeBridge<QuickLauncherIndexSnapshot>(QUICK_LAUNCHER_COMMANDS.snapshot),
    refresh: () =>
      invokeBridge<QuickLauncherIndexSnapshot>(QUICK_LAUNCHER_COMMANDS.refresh),
    search: (query: string, limit = 24) =>
      invokeBridge<QuickLauncherSearchResult>(QUICK_LAUNCHER_COMMANDS.search, {
        input: buildSearchInput(query, limit),
      }),
    getIcon: (itemId: string, iconKey?: string) =>
      invokeBridge<QuickLauncherIconResult>(QUICK_LAUNCHER_COMMANDS.icon, {
        input: buildIconInput(itemId, iconKey),
      }),
    getIcons: (items: QuickLauncherIconInput[]) =>
      invokeBridge<QuickLauncherIconBatchResult>(QUICK_LAUNCHER_COMMANDS.icons, {
        input: { items },
      }),
    refreshRunning: () =>
      invokeBridge<QuickLauncherRunningSnapshot>(
        QUICK_LAUNCHER_COMMANDS.refreshRunning,
      ),
    activate: (itemId: string, revision: number) =>
      invokeBridge<QuickLauncherActivationResult>(QUICK_LAUNCHER_COMMANDS.activate, {
        input: buildActivateInput(itemId, revision),
      }),
    showWindow: () => invokeBridge<void>(QUICK_LAUNCHER_COMMANDS.showWindow),
    hideWindow: () => invokeBridge<void>(QUICK_LAUNCHER_COMMANDS.hideWindow),
  };
}

export function buildSearchInput(query: string, limit: number): QuickLauncherSearchInput {
  return { query, limit };
}

export function buildIconInput(itemId: string, iconKey?: string): QuickLauncherIconInput {
  return iconKey === undefined ? { itemId } : { itemId, iconKey };
}

export function buildActivateInput(
  itemId: string,
  revision: number,
): QuickLauncherActivateInput {
  return { itemId, revision };
}

export function normalizeQuickLauncherError(value: unknown): QuickLauncherError {
  if (isRecord(value) &&
    typeof value.operation === "string" &&
    typeof value.code === "string" &&
    typeof value.message === "string" &&
    typeof value.retryable === "boolean") {
    return {
      operation: value.operation,
      code: value.code,
      message: value.message,
      retryable: value.retryable,
    };
  }

  return {
    operation: "launcher.client",
    code: "launcher.unknown",
    message: value instanceof Error ? value.message : String(value),
    retryable: true,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
