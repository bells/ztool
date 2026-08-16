import { invoke } from "@tauri-apps/api/core";
import {
  createGlobalShortcutService,
  type GlobalShortcutCommand,
} from "./globalShortcutServiceCore";

export {
  GLOBAL_SHORTCUT_COMMANDS,
  createGlobalShortcutService,
  shortcutStatusMessageKey,
  type GlobalShortcutActionId,
  type GlobalShortcutCommand,
  type GlobalShortcutInvokeBridge,
  type GlobalShortcutRegistrationState,
  type GlobalShortcutSnapshot,
} from "./globalShortcutServiceCore";

export const globalShortcutService = createGlobalShortcutService(
  <T>(command: GlobalShortcutCommand) => invoke<T>(command),
);
