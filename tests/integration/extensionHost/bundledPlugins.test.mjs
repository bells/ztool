import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const plugins = [
  ["screenshot", "zero.snap", "plugins/screenshot", "accent-screenshot"],
  ["caffeine", "zero.awake", "plugins/caffeine", "accent-caffeine"],
  ["bingWallpaper", "zero.paper", "plugins/bingWallpaper", "accent-bing-wallpaper"],
  ["quickLauncher", "zero.launch", "plugins/quickLauncher", "accent-quick-launcher"],
];

test("each bundled plugin owns its manifest presentation localization and renderer", () => {
  for (const [directory, id, main, accent] of plugins) {
    const source = fs.readFileSync(`src/plugins/${directory}/plugin.tsx`, "utf8");
    assert.match(source, /(?:name|id): FIRST_PARTY_PLUGIN_IDS\./);
    assert.match(source, new RegExp(`main: ["']${main}["']`));
    assert.match(source, new RegExp(`accentClass: ["']${accent}["']`));
    assert.match(source, /presentation:/);
    assert.match(source, /renderPanel\(language\)/);
    assert.ok(fs.existsSync(`src/plugins/${directory}/i18n.ts`));
    assert.match(source, new RegExp(id.split(".").join("\\.")));
  }
});

test("the app shell is the only composition root for concrete plugin descriptors", () => {
  const composition = fs.readFileSync(
    "src/appShell/bundledPluginModules.ts",
    "utf8",
  );
  for (const [directory] of plugins) {
    assert.match(composition, new RegExp(`plugins/${directory}/plugin`));
  }
  assert.match(composition, /createBundledPluginRegistry/);
  assert.match(composition, /createExtensionLauncherHostApis\(\s*quickLauncherService/);
});

test("plugin manifests keep host-mediated status bar contributions", () => {
  const combined = plugins
    .map(([directory]) => fs.readFileSync(`src/plugins/${directory}/plugin.tsx`, "utf8"))
    .join("\n");
  for (const contribution of [
    "zero.snap.status",
    "zero.awake.status",
    "zero.paper.status",
    "zero.launch.status",
  ]) {
    assert.match(combined, new RegExp(contribution.replaceAll(".", "\\.")));
  }
});
