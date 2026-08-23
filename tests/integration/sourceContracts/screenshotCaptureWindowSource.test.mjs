import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const screenshotServiceSource = await readFile(
  new URL("../../../src-tauri/src/services/screenshot.rs", import.meta.url),
  "utf8",
);
const screenshotCommandSource = await readFile(
  new URL("../../../src-tauri/src/commands/screenshot.rs", import.meta.url),
  "utf8",
);
const cargoSource = await readFile(
  new URL("../../../src-tauri/Cargo.toml", import.meta.url),
  "utf8",
);

test("opens the macOS capture overlay borderlessly without native fullscreen chrome", () => {
  const openCaptureWindowSource = screenshotServiceSource.slice(
    screenshotServiceSource.indexOf("fn open_capture_window"),
    screenshotServiceSource.indexOf("fn reveal_capture_window_on_main_thread"),
  );

  assert.match(openCaptureWindowSource, /primary_monitor\(\)/);
  assert.match(openCaptureWindowSource, /\.decorations\(false\)/);
  assert.match(openCaptureWindowSource, /\.visible\(false\)/);
  assert.match(openCaptureWindowSource, /set_position\(monitor_position\)/);
  assert.match(openCaptureWindowSource, /set_size\(monitor_size\)/);
  assert.ok(
    openCaptureWindowSource.indexOf("set_size(monitor_size)") <
      openCaptureWindowSource.indexOf("set_position(monitor_position)"),
    "the hidden window must be sized before positioning so macOS resize anchoring cannot shift its top edge",
  );
  assert.doesNotMatch(openCaptureWindowSource, /show_surface\(&capture_window\)/);
  assert.doesNotMatch(openCaptureWindowSource, /capture_window\.set_focus\(\)/);
  assert.doesNotMatch(openCaptureWindowSource, /\.title\(/);
  assert.doesNotMatch(openCaptureWindowSource, /\.fullscreen\(true\)/);
});

test("reveals the prepared capture window once from a scoped session command", () => {
  assert.match(screenshotCommandSource, /pub async fn reveal_screenshot_capture/);
  assert.match(screenshotServiceSource, /require_capture_window\(window_label\)\?/);
  assert.match(screenshotServiceSource, /validate_session\(&app, Some\(&session_id\)\)\?/);
  assert.match(screenshotServiceSource, /claim_reveal\(&session_id\)\?/);
  assert.match(screenshotServiceSource, /run_on_main_thread/);
  assert.match(screenshotServiceSource, /recv_timeout\(SCREENSHOT_REVEAL_TIMEOUT\)/);
  assert.match(screenshotServiceSource, /cleanup_failed_reveal/);
});

test("places only the macOS capture window above live system chrome", () => {
  assert.match(cargoSource, /\[target\.'cfg\(target_os = "macos"\)'\.dependencies\][\s\S]*objc2-app-kit/);
  assert.match(screenshotServiceSource, /NSScreenSaverWindowLevel/);
  assert.match(screenshotServiceSource, /NSWindowSharingType::None/);
  assert.match(screenshotServiceSource, /NSWindowAnimationBehavior::None/);
  assert.match(screenshotServiceSource, /NSWindowCollectionBehavior::CanJoinAllSpaces/);
  assert.match(screenshotServiceSource, /NSWindowCollectionBehavior::FullScreenAuxiliary/);
  assert.match(screenshotServiceSource, /NSWindowCollectionBehavior::Stationary/);
  assert.doesNotMatch(screenshotServiceSource, /simple_fullscreen|set_presentation_options/);
});
