import assert from "node:assert/strict";
import test from "node:test";
import { createBingWallpaperTranslator } from "/private/tmp/zero-tests/plugins/bingWallpaper/i18n.js";
import { createCaffeineTranslator } from "/private/tmp/zero-tests/plugins/caffeine/i18n.js";
import { createQuickLauncherTranslator } from "/private/tmp/zero-tests/plugins/quickLauncher/i18n.js";
import { createScreenshotTranslator } from "/private/tmp/zero-tests/plugins/screenshot/i18n.js";

test("each plugin resolves its own bilingual presentation and action messages", () => {
  assert.equal(createCaffeineTranslator("zh-CN")("plugin.title"), "Zero Awake");
  assert.equal(createBingWallpaperTranslator("en-US")("wallpaper.apply"), "Set as desktop wallpaper");
  assert.equal(createQuickLauncherTranslator("zh-CN")("launcher.errorStale"), "该项目已变化，请刷新后重试");
  assert.equal(createScreenshotTranslator("en-US")("screenshot.toolbar.copy"), "Copy screenshot");
});

test("plugin translators do not silently consume another plugin namespace", () => {
  assert.equal(createCaffeineTranslator("en-US")("wallpaper.apply"), "wallpaper.apply");
  assert.equal(createScreenshotTranslator("zh-CN")("launcher.title"), "launcher.title");
});
