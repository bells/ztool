import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const screenshotServiceSource = await readFile(
  new URL("../src-tauri/src/services/screenshot.rs", import.meta.url),
  "utf8",
);

test("opens the macOS capture overlay borderlessly without native fullscreen chrome", () => {
  const openCaptureWindowSource = screenshotServiceSource.slice(
    screenshotServiceSource.indexOf("fn open_capture_window"),
    screenshotServiceSource.indexOf("fn platform_name"),
  );

  assert.match(openCaptureWindowSource, /primary_monitor\(\)/);
  assert.match(openCaptureWindowSource, /\.decorations\(false\)/);
  assert.match(openCaptureWindowSource, /\.visible\(false\)/);
  assert.match(openCaptureWindowSource, /set_position\(monitor_position\)/);
  assert.match(openCaptureWindowSource, /set_size\(monitor_size\)/);
  assert.doesNotMatch(openCaptureWindowSource, /\.title\(/);
  assert.doesNotMatch(openCaptureWindowSource, /\.fullscreen\(true\)/);
});
