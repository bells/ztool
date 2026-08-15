import type {
  InstallMarketPluginInput,
  InstallPluginPackageInput,
  PluginIdentityInput,
  PluginLifecycleResult,
  PluginMarketEntry,
  PluginMarketSnapshot,
  PluginPackageValidationReport,
  PluginRecord,
  SetPluginEnabledInput,
  ValidatePluginPackageInput,
} from "./contracts";

export const PLUGIN_HOST_COMMANDS = {
  refreshMarket: "refresh_plugin_market",
  listMarketPlugins: "list_market_plugins",
  listPlugins: "list_plugins",
  validatePackage: "validate_plugin_package",
  installMarketPlugin: "install_market_plugin",
  installPluginPackage: "install_plugin_package",
  uninstallPlugin: "uninstall_plugin",
  setPluginEnabled: "set_plugin_enabled",
  restoreBundledPlugins: "restore_bundled_plugins",
} as const;

export type PluginHostCommand =
  (typeof PLUGIN_HOST_COMMANDS)[keyof typeof PLUGIN_HOST_COMMANDS];

export type PluginHostInvokeArgs =
  | { input: ValidatePluginPackageInput }
  | { input: InstallPluginPackageInput }
  | { input: InstallMarketPluginInput }
  | { input: PluginIdentityInput }
  | { input: SetPluginEnabledInput };

export type PluginHostInvokeBridge = <T>(
  command: PluginHostCommand,
  args?: PluginHostInvokeArgs,
) => Promise<T>;

export function createPluginHostService(invokeCommand: PluginHostInvokeBridge) {
  return {
    refreshMarket(): Promise<PluginMarketSnapshot> {
      return invokeCommand<PluginMarketSnapshot>(PLUGIN_HOST_COMMANDS.refreshMarket);
    },

    listMarketPlugins(): Promise<PluginMarketEntry[]> {
      return invokeCommand<PluginMarketEntry[]>(PLUGIN_HOST_COMMANDS.listMarketPlugins);
    },

    listPlugins(): Promise<PluginRecord[]> {
      return invokeCommand<PluginRecord[]>(PLUGIN_HOST_COMMANDS.listPlugins);
    },

    validatePackage(
      input: ValidatePluginPackageInput,
    ): Promise<PluginPackageValidationReport> {
      return invokeCommand<PluginPackageValidationReport>(
        PLUGIN_HOST_COMMANDS.validatePackage,
        { input },
      );
    },

    installMarketPlugin(input: InstallMarketPluginInput): Promise<PluginRecord> {
      return invokeCommand<PluginRecord>(PLUGIN_HOST_COMMANDS.installMarketPlugin, {
        input,
      });
    },

    installPluginPackage(input: InstallPluginPackageInput): Promise<PluginRecord> {
      return invokeCommand<PluginRecord>(PLUGIN_HOST_COMMANDS.installPluginPackage, {
        input,
      });
    },

    uninstallPlugin(input: PluginIdentityInput): Promise<PluginLifecycleResult> {
      return invokeCommand<PluginLifecycleResult>(
        PLUGIN_HOST_COMMANDS.uninstallPlugin,
        { input },
      );
    },

    setPluginEnabled(input: SetPluginEnabledInput): Promise<PluginRecord> {
      return invokeCommand<PluginRecord>(PLUGIN_HOST_COMMANDS.setPluginEnabled, {
        input,
      });
    },

    restoreBundledPlugins(): Promise<PluginRecord[]> {
      return invokeCommand<PluginRecord[]>(
        PLUGIN_HOST_COMMANDS.restoreBundledPlugins,
      );
    },
  };
}
