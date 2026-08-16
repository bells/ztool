export type GlobalShortcutActionId = "snapCapture" | "launchPanel";

export type GlobalShortcutRegistrationState =
  | "active"
  | "inactive"
  | "conflict"
  | "unsupported";

export interface GlobalShortcutSnapshot {
  id: GlobalShortcutActionId;
  pluginName: string;
  accelerator: string;
  enabled: boolean;
  registered: boolean;
  platformSupported: boolean;
  registrationState: GlobalShortcutRegistrationState;
  diagnosticCode?: string;
}

export const GLOBAL_SHORTCUT_COMMANDS = {
  getSnapshots: "get_global_shortcut_snapshots",
} as const;

export type GlobalShortcutCommand =
  (typeof GLOBAL_SHORTCUT_COMMANDS)[keyof typeof GLOBAL_SHORTCUT_COMMANDS];

export type GlobalShortcutInvokeBridge = <T>(
  command: GlobalShortcutCommand,
) => Promise<T>;

export function createGlobalShortcutService(
  invokeCommand: GlobalShortcutInvokeBridge,
) {
  return {
    getSnapshots: () =>
      invokeCommand<GlobalShortcutSnapshot[]>(GLOBAL_SHORTCUT_COMMANDS.getSnapshots),
  };
}

export function shortcutStatusMessageKey(
  state: GlobalShortcutRegistrationState,
):
  | "prefs.shortcuts.active"
  | "prefs.shortcuts.inactive"
  | "prefs.shortcuts.conflict"
  | "prefs.shortcuts.unsupported" {
  switch (state) {
    case "active":
      return "prefs.shortcuts.active";
    case "conflict":
      return "prefs.shortcuts.conflict";
    case "unsupported":
      return "prefs.shortcuts.unsupported";
    case "inactive":
      return "prefs.shortcuts.inactive";
  }
}
