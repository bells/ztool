import { invoke } from "@tauri-apps/api/core";
import {
  createPluginHostService,
  type PluginHostCommand,
  type PluginHostInvokeArgs,
} from "./pluginHostServiceCore";

export {
  createPluginHostService,
  PLUGIN_HOST_COMMANDS,
  type PluginHostCommand,
  type PluginHostInvokeArgs,
  type PluginHostInvokeBridge,
} from "./pluginHostServiceCore";

export const pluginHostService = createPluginHostService(
  <T>(command: PluginHostCommand, args?: PluginHostInvokeArgs) =>
    invoke<T>(command, args),
);
