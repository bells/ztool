import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const [serviceSource, commandSource, captureSource, pinSource, exportSource] = await Promise.all([
  readFile(new URL("../../../src-tauri/src/services/screenshot.rs", import.meta.url), "utf8"),
  readFile(new URL("../../../src-tauri/src/commands/screenshot.rs", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/capture/CaptureApp.tsx", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/capture/PinApp.tsx", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/capture/captureExport.ts", import.meta.url), "utf8"),
]);

test("uses scoped raw screenshot media instead of Base64 JSON contracts", () => {
  for (const source of [serviceSource, commandSource, captureSource, pinSource]) {
    assert.doesNotMatch(source, /image_base64|png_base64|data:image\/png;base64/);
  }
  assert.match(commandSource, /Result<tauri::ipc::Response, ScreenshotError>/);
  assert.match(commandSource, /tauri::ipc::Request<'_>/);
  assert.match(commandSource, /InvokeBody::Raw/);
  assert.match(serviceSource, /ScreenshotMediaDescriptor/);
  assert.match(serviceSource, /PrepareScreenshotCommitInput/);
  assert.match(serviceSource, /ScreenshotUploadLease/);
});

test("binds screenshot reads and uploads to opaque tokens and calling windows", () => {
  assert.match(serviceSource, /require_capture_window\(window_label\)/);
  assert.match(serviceSource, /resolve_media\(&input\.token, window_label\)/);
  assert.match(serviceSource, /validate_resource_token/);
  assert.match(commandSource, /x-zero-screenshot-lease/);
  const prepareInput = serviceSource.slice(
    serviceSource.indexOf("pub struct PrepareScreenshotCommitInput"),
    serviceSource.indexOf("pub struct ScreenshotUploadLease"),
  );
  assert.doesNotMatch(prepareInput, /savePath|save_path|PathBuf/);
  assert.match(commandSource, /rfd::AsyncFileDialog/);
});

test("releases frontend and native screenshot resources at terminal boundaries", () => {
  assert.match(captureSource, /releaseObjectUrl\(objectUrl\)/);
  assert.match(captureSource, /receivedBytes\?\.fill\(0\)/);
  assert.match(captureSource, /pngBytes\?\.fill\(0\)/);
  assert.match(pinSource, /releaseObjectUrl\(objectUrl\)/);
  assert.match(pinSource, /imageRef\.current\.src = ""/);
  assert.match(exportSource, /canvas\.width = 1/);
  assert.match(exportSource, /canvas\.height = 1/);
  assert.match(serviceSource, /tauri::WindowEvent::Destroyed/);
  assert.match(serviceSource, /remove_pin\(&cleanup_label\)/);
});

test("retains the Windows system screenshot launcher without macOS media assumptions", () => {
  assert.match(serviceSource, /#\[cfg\(target_os = "windows"\)\]/);
  assert.match(serviceSource, /\.arg\("ms-screenclip:"\)/);
  assert.match(serviceSource, /SnippingTool\.exe/);
  assert.match(serviceSource, /#\[cfg\(target_os = "macos"\)\]\s*fn capture_fullscreen_png/);
});
