import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

function sourceFiles(root, extension) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) return sourceFiles(target, extension);
    return entry.isFile() && target.endsWith(extension) ? [target] : [];
  });
}

test("native window lifecycle transitions are owned by the surface activity helper", () => {
  const directTransition = /\.(?:show|hide|close|destroy)\(\)/;
  const offenders = sourceFiles("src-tauri/src", ".rs")
    .filter((file) => !file.endsWith("services/surface_activity.rs"))
    .filter((file) => !file.endsWith("services/file/word_macos.rs"))
    .filter((file) => directTransition.test(fs.readFileSync(file, "utf8")));

  assert.deepEqual(offenders, []);
});

test("Rust and TypeScript share the surface activity event and states", () => {
  const rust = fs.readFileSync("src-tauri/src/services/surface_activity.rs", "utf8");
  const typescript = fs.readFileSync("src/core/windowing/surfaceActivityCore.ts", "utf8");
  const hook = fs.readFileSync("src/core/windowing/useSurfaceActivity.ts", "utf8");

  assert.match(rust, /zero:\/\/surface-activity/);
  assert.match(typescript, /zero:\/\/surface-activity/);
  for (const state of ["active", "hidden", "disposed"]) {
    assert.match(typescript, new RegExp(`"${state}"`));
  }
  assert.match(hook, /get_surface_activity/);
  assert.match(hook, /document\.visibilityState/);
  assert.doesNotMatch(hook, /isFocused|onFocusChanged/);
});

test("frontend hide and close actions use host-owned lifecycle commands", () => {
  const paper = fs.readFileSync("src/plugins/bingWallpaper/PaperApp.tsx", "utf8");
  const pin = fs.readFileSync("src/plugins/screenshot/capture/PinApp.tsx", "utf8");

  assert.match(paper, /invoke\("hide_current_surface"\)/);
  assert.match(pin, /invoke\("close_current_surface"\)/);
  assert.doesNotMatch(`${paper}\n${pin}`, /getCurrentWindow\(\)\.(?:hide|close)/);
});

test("Paper preview work follows activity and revokes opaque preview resources", () => {
  const hook = fs.readFileSync("src/plugins/bingWallpaper/useBingWallpaper.ts", "utf8");
  const contracts = fs.readFileSync("src/plugins/bingWallpaper/contracts.ts", "utf8");
  const rust = fs.readFileSync("src-tauri/src/services/bing_wallpaper/mod.rs", "utf8");

  assert.match(hook, /useSurfaceActivity\(\)/);
  assert.match(hook, /shouldStartBingWallpaperPresentation\(activity\)/);
  assert.match(hook, /URL\.createObjectURL/);
  assert.match(hook, /URL\.revokeObjectURL/);
  assert.match(hook, /service\.releasePreview/);
  assert.match(contracts, /token: string/);
  assert.match(contracts, /byteLength: number/);
  assert.doesNotMatch(contracts, /dataUrl: string/);
  assert.doesNotMatch(rust, /data_url/);
  assert.match(rust, /MAX_PREVIEW_BYTES/);
  assert.match(rust, /PREVIEW_MAX_WIDTH/);
  assert.match(rust, /PREVIEW_MAX_HEIGHT/);
});
