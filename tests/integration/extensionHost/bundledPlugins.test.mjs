import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const plugins = [
  ["screenshot", "zero.snap", "plugins/screenshot", "accent-screenshot"],
  ["caffeine", "zero.awake", "plugins/caffeine", "accent-caffeine"],
  ["bingWallpaper", "zero.paper", "plugins/bingWallpaper", "accent-bing-wallpaper"],
  ["quickLauncher", "zero.launch", "plugins/quickLauncher", "accent-quick-launcher"],
  ["file", "zero.file", "plugins/file", "accent-file"],
];

test("each bundled plugin owns its manifest presentation localization and lazy renderer", () => {
  for (const [directory, id, main, accent] of plugins) {
    const source = fs.readFileSync(`src/plugins/${directory}/plugin.tsx`, "utf8");
    assert.match(source, /(?:name|id): FIRST_PARTY_PLUGIN_IDS\./);
    assert.match(source, new RegExp(`main: ["']${main}["']`));
    assert.match(source, new RegExp(`accentClass: ["']${accent}["']`));
    assert.match(source, /presentation:/);
    assert.match(source, /loadPanel:\s*\(\)\s*=>\s*import\(/);
    assert.ok(fs.existsSync(`src/plugins/${directory}/i18n.ts`));
    assert.match(source, new RegExp(id.split(".").join("\\.")));
  }
});

test("dedicated surfaces and panels stay behind plugin-owned dynamic imports", () => {
  const main = fs.readFileSync("src/main.tsx", "utf8");
  assert.doesNotMatch(main, /from ["']\.\/App["']/);
  assert.doesNotMatch(main, /from ["']\.\/plugins\/file\/engine\/FileEngineApp["']/);
  assert.match(main, /lazy\(loadRoutedApp\)/);

  const screenshot = fs.readFileSync("src/plugins/screenshot/plugin.tsx", "utf8");
  assert.match(screenshot, /capture:\s*\(\)\s*=>\s*import\("\.\/capture\/CaptureApp"\)/);
  assert.match(screenshot, /pin:\s*\(\)\s*=>\s*import\("\.\/capture\/PinApp"\)/);
  assert.match(screenshot, /"snap-menu":\s*\(\)\s*=>\s*import\("\.\/SnapMenuApp"\)/);
  const launch = fs.readFileSync("src/plugins/quickLauncher/plugin.tsx", "utf8");
  assert.match(launch, /launcher:\s*\(\)\s*=>\s*import\("\.\/QuickLauncherApp"\)/);
  const paper = fs.readFileSync("src/plugins/bingWallpaper/plugin.tsx", "utf8");
  assert.match(paper, /paper:\s*\(\)\s*=>\s*import\("\.\/PaperApp"\)/);

  const boundary = fs.readFileSync("src/appShell/LazyPluginPanel.tsx", "utf8");
  assert.match(boundary, /<Suspense fallback=/);
  assert.match(boundary, /getDerivedStateFromError/);
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

test("Zero File stays a trusted main-view plugin without extension permissions or a status item", () => {
  const source = fs.readFileSync("src/plugins/file/plugin.tsx", "utf8");

  assert.match(source, /permissions:\s*\[\]/);
  assert.match(source, /platforms:\s*\["macos",\s*"windows"\]/);
  assert.match(source, /views:\s*\[\{ id: "zero\.file\.main"/);
  assert.doesNotMatch(source, /statusBarItems/);
});
