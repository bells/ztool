import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { SNAP_MENU_ITEMS } from "/private/tmp/zero-tests/plugins/screenshot/snapMenuModel.js";
import { createScreenshotTranslator } from "/private/tmp/zero-tests/plugins/screenshot/i18n.js";

test("Snap menu initially exposes one typed localized Screenshot action", () => {
  assert.deepEqual(SNAP_MENU_ITEMS, [
    { id: "screenshot", labelKey: "screenshot.menu.screenshot" },
  ]);
  assert.equal(createScreenshotTranslator("zh-CN")("screenshot.menu.screenshot"), "截图");
  assert.equal(createScreenshotTranslator("en-US")("screenshot.menu.screenshot"), "Screenshot");
});

test("Snap menu autofocuses, dismisses through the host, and uses restricted handoff", () => {
  const source = fs.readFileSync("src/plugins/screenshot/SnapMenuApp.tsx", "utf8");
  assert.match(source, /useSurfaceActivity\(\)/);
  assert.match(source, /firstActionRef\.current\?\.focus/);
  assert.match(source, /event\.key === "Escape"/);
  assert.match(source, /invoke\("hide_current_surface"\)/);
  assert.match(source, /invoke<ScreenshotStartResult>\("start_snap_menu_screenshot"\)/);
  assert.doesNotMatch(source, /screen.?record|audio.?record|录屏|录音/i);
});

test("Rust registers and scopes the Snap menu screenshot command", () => {
  const command = fs.readFileSync("src-tauri/src/commands/screenshot.rs", "utf8");
  const app = fs.readFileSync("src-tauri/src/lib.rs", "utf8");
  assert.match(command, /pub async fn start_snap_menu_screenshot/);
  assert.match(command, /require_snap_menu_window/);
  assert.match(command, /start_screenshot_session\([^]*"copy"\.into\(\)\)/);
  assert.match(app, /commands::screenshot::start_snap_menu_screenshot/);
});
