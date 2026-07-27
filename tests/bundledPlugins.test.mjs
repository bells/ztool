import assert from "node:assert/strict";
import test from "node:test";
import {
  BUNDLED_PLUGIN_MANIFESTS,
  bundledPluginKind,
  pluginAccentClass,
} from "/private/tmp/zero-bundled-plugins-test/plugins/pluginHost/bundledPlugins.js";

test("defines stable bundled plugin manifests with safe main paths", () => {
  assert.deepEqual(
    BUNDLED_PLUGIN_MANIFESTS.map((manifest) => ({
      name: manifest.name,
      main: manifest.main,
      runtime: manifest.runtime,
    })),
    [
      {
        name: "zero.snap",
        main: "plugins/screenshot",
        runtime: "webview",
      },
      {
        name: "zero.awake",
        main: "plugins/caffeine",
        runtime: "webview",
      },
      {
        name: "zero.paper",
        main: "plugins/bingWallpaper",
        runtime: "webview",
      },
      {
        name: "zero.launch",
        main: "plugins/quickLauncher",
        runtime: "webview",
      },
    ],
  );
  assert.equal(
    BUNDLED_PLUGIN_MANIFESTS.every((manifest) => !manifest.main.includes("..")),
    true,
  );
});

test("bundled manifests declare views commands permissions and platforms", () => {
  const screenshot = BUNDLED_PLUGIN_MANIFESTS[0];
  const caffeine = BUNDLED_PLUGIN_MANIFESTS[1];
  const bing = BUNDLED_PLUGIN_MANIFESTS[2];
  const launcher = BUNDLED_PLUGIN_MANIFESTS[3];

  assert.deepEqual(screenshot.platforms, ["macos", "windows", "linux"]);
  assert.deepEqual(caffeine.permissions, ["ui.message"]);
  assert.deepEqual(bing.permissions, ["network", "storage.plugin", "system.wallpaper"]);
  assert.equal(bing.id, "zero.paper");
  assert.equal(bing.author, "bells");
  assert.equal(launcher.id, "zero.launch");
  assert.equal(launcher.author, "bells");
  assert.deepEqual(launcher.platforms, ["macos", "windows"]);
  assert.deepEqual(launcher.permissions, [
    "system.apps.read",
    "system.apps.execute",
    "system.window.focus",
    "system.settings.open",
  ]);
  assert.equal(screenshot.contributes.views[0].id, "zero.snap.main");
  assert.equal(caffeine.contributes.commands[0].id, "zero.awake.toggle");
  assert.equal(bing.contributes.commands[1].id, "zero.paper.apply");
});

test("bundled manifests declare host-mediated status bar items", () => {
  const screenshot = BUNDLED_PLUGIN_MANIFESTS[0];
  const caffeine = BUNDLED_PLUGIN_MANIFESTS[1];
  const paper = BUNDLED_PLUGIN_MANIFESTS[2];
  const launch = BUNDLED_PLUGIN_MANIFESTS[3];

  assert.deepEqual(screenshot.contributes.statusBarItems, [
    {
      id: "zero.snap.status",
      title: "Zero Snap",
      icon: "screenshot",
      action: {
        type: "start-screenshot",
        commandId: "zero.snap.capture",
      },
      order: 20,
      visibleByDefault: true,
    },
  ]);
  assert.deepEqual(caffeine.contributes.statusBarItems, [
    {
      id: "zero.awake.status",
      title: "Zero Awake",
      icon: "caffeine-empty",
      activeIcon: "caffeine-full",
      action: {
        type: "toggle-caffeine",
        commandId: "zero.awake.toggle",
      },
      order: 10,
      visibleByDefault: true,
    },
  ]);
  assert.deepEqual(paper.contributes.statusBarItems, [
    {
      id: "zero.paper.status",
      title: "Zero Paper",
      icon: "paper",
      action: {
        type: "open-plugin",
      },
      order: 30,
      visibleByDefault: true,
    },
  ]);
  assert.deepEqual(launch.contributes.statusBarItems, [
    {
      id: "zero.launch.status",
      title: "Zero Launch",
      icon: "launch",
      action: {
        type: "open-plugin",
      },
      order: 40,
      visibleByDefault: true,
    },
  ]);
});

test("resolves bundled plugin kind and accent class from registry names", () => {
  assert.equal(bundledPluginKind("zero.snap"), "screenshot");
  assert.equal(bundledPluginKind("zero.awake"), "caffeine");
  assert.equal(bundledPluginKind("zero.paper"), "bing-wallpaper");
  assert.equal(bundledPluginKind("bing-wallpaper"), "bing-wallpaper");
  assert.equal(bundledPluginKind("zero.launch"), "quick-launcher");
  assert.equal(bundledPluginKind("ztool.quick-launcher"), "quick-launcher");
  assert.equal(bundledPluginKind("ztool.screenshot"), "screenshot");
  assert.equal(bundledPluginKind("ztool.caffeine"), "caffeine");
  assert.equal(bundledPluginKind("ztool.bing-wallpaper"), "bing-wallpaper");
  assert.equal(bundledPluginKind("quick-launcher"), "quick-launcher");
  assert.equal(bundledPluginKind("market-tool"), null);
  assert.equal(pluginAccentClass("zero.snap"), "accent-screenshot");
  assert.equal(pluginAccentClass("zero.paper"), "accent-bing-wallpaper");
  assert.equal(pluginAccentClass("zero.launch"), "accent-quick-launcher");
  assert.equal(pluginAccentClass("market-tool"), "accent-extension");
});
