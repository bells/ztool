import assert from "node:assert/strict";
import test from "node:test";
import {
  createTranslator,
  resolveLanguage,
} from "/private/tmp/zero-i18n-test/plugins/preferences/i18n.js";

test("resolves system language to Chinese for zh locales", () => {
  assert.equal(resolveLanguage("system", "zh-CN"), "zh-CN");
  assert.equal(resolveLanguage("system", "zh-Hant"), "zh-CN");
});

test("resolves system language to English for non-Chinese locales", () => {
  assert.equal(resolveLanguage("system", "en-US"), "en-US");
  assert.equal(resolveLanguage("system", "fr-FR"), "en-US");
});

test("explicit language overrides system language", () => {
  assert.equal(resolveLanguage("en-US", "zh-CN"), "en-US");
  assert.equal(resolveLanguage("zh-CN", "en-US"), "zh-CN");
});

test("translator returns localized labels", () => {
  assert.equal(createTranslator("zh-CN")("nav.preferences"), "偏好");
  assert.equal(createTranslator("en-US")("nav.preferences"), "Prefs");
});

test("Bing wallpaper metadata actions and states are translated", () => {
  const zh = createTranslator("zh-CN");
  const en = createTranslator("en-US");

  assert.equal(zh("plugin.bingWallpaper.title"), "Zero Paper");
  assert.equal(en("plugin.bingWallpaper.title"), "Zero Paper");
  assert.equal(zh("plugin.quickLauncher.title"), "Zero Launch");
  assert.equal(en("plugin.quickLauncher.title"), "Zero Launch");
  assert.equal(zh("launcher.errorStale"), "该项目已变化，请刷新后重试");
  for (const key of [
    "wallpaper.download",
    "wallpaper.apply",
    "wallpaper.loading",
    "wallpaper.stale",
    "wallpaper.empty",
    "wallpaper.applied",
    "wallpaper.saved",
    "wallpaper.platformUnsupported",
  ]) {
    assert.notEqual(zh(key), key);
    assert.notEqual(en(key), key);
  }
});

test("Zero Snap icon toolbar labels are localized", () => {
  const zh = createTranslator("zh-CN");
  const en = createTranslator("en-US");

  for (const key of [
    "screenshot.toolbar.label",
    "screenshot.toolbar.select",
    "screenshot.toolbar.rectangle",
    "screenshot.toolbar.ellipse",
    "screenshot.toolbar.arrow",
    "screenshot.toolbar.pen",
    "screenshot.toolbar.text",
    "screenshot.toolbar.mosaic",
    "screenshot.toolbar.pin",
    "screenshot.toolbar.undo",
    "screenshot.toolbar.redo",
    "screenshot.toolbar.delete",
    "screenshot.toolbar.cancel",
    "screenshot.toolbar.save",
    "screenshot.toolbar.copy",
  ]) {
    assert.notEqual(zh(key), key);
    assert.notEqual(en(key), key);
  }
  assert.equal(zh("screenshot.toolbar.select"), "选择截图区域");
  assert.equal(en("screenshot.toolbar.copy"), "Copy screenshot");
});
