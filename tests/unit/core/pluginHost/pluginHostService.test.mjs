import assert from "node:assert/strict";
import test from "node:test";
import {
  createPluginHostService,
  PLUGIN_HOST_COMMANDS,
} from "/private/tmp/zero-tests/core/pluginHost/pluginHostServiceCore.js";

test("defines explicit Rust command names for plugin host lifecycle", () => {
  assert.deepEqual(PLUGIN_HOST_COMMANDS, {
    refreshMarket: "refresh_plugin_market",
    listMarketPlugins: "list_market_plugins",
    listPlugins: "list_plugins",
    validatePackage: "validate_plugin_package",
    installMarketPlugin: "install_market_plugin",
    installPluginPackage: "install_plugin_package",
    uninstallPlugin: "uninstall_plugin",
    setPluginEnabled: "set_plugin_enabled",
    restoreBundledPlugins: "restore_bundled_plugins",
  });
});

test("invokes plugin host commands through the injected bridge", async () => {
  const calls = [];
  const service = createPluginHostService(async (command, args) => {
    calls.push({ command, args });

    if (command === "refresh_plugin_market") {
      return {
        sourceUrl: "https://github.com/bells/zero/market.json",
        schemaVersion: 1,
        entries: [],
        stale: false,
      };
    }

    if (command === "list_market_plugins" || command === "list_plugins" || command === "restore_bundled_plugins") {
      return [];
    }

    if (command === "validate_plugin_package") {
      return {
        valid: true,
        issues: [],
        packagePath: args.input.packagePath,
        sha256: "0".repeat(64),
      };
    }

    return {
      name: "local-tool",
      version: "0.1.0",
      author: "watson",
      source: "local",
      enabled: true,
      health: "ready",
      manifest: {
        name: "local-tool",
        version: "0.1.0",
        author: "watson",
        main: "dist/index.html",
        permissions: ["ui.message"],
      },
      approvedPermissions: ["ui.message"],
    };
  });

  await service.refreshMarket();
  await service.listMarketPlugins();
  await service.listPlugins();
  await service.validatePackage({ packagePath: "/tmp/local-tool.zplugin" });
  await service.installPluginPackage({
    packagePath: "/tmp/local-tool.zplugin",
    approvedPermissions: ["ui.message"],
    enabled: true,
  });
  await service.installMarketPlugin({
    entry: {
      name: "local-tool",
      version: "0.1.0",
      author: "watson",
      repository: "https://github.com/watson/local-tool",
      releaseUrl: "https://github.com/watson/local-tool/releases/tag/v0.1.0",
      downloadUrl: "https://github.com/watson/local-tool/releases/download/v0.1.0/local-tool.zplugin",
      permissions: ["ui.message"],
    },
    approvedPermissions: ["ui.message"],
  });
  await service.uninstallPlugin({ name: "local-tool" });
  await service.setPluginEnabled({ name: "local-tool", enabled: false });
  await service.restoreBundledPlugins();

  assert.deepEqual(calls.map((call) => call.command), [
    "refresh_plugin_market",
    "list_market_plugins",
    "list_plugins",
    "validate_plugin_package",
    "install_plugin_package",
    "install_market_plugin",
    "uninstall_plugin",
    "set_plugin_enabled",
    "restore_bundled_plugins",
  ]);
  assert.deepEqual(calls[3].args, {
    input: { packagePath: "/tmp/local-tool.zplugin" },
  });
  assert.deepEqual(calls[7].args, {
    input: { name: "local-tool", enabled: false },
  });
});
