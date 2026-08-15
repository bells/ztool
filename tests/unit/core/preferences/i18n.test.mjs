import assert from "node:assert/strict";
import test from "node:test";
import {
  createTranslator,
  resolveLanguage,
} from "/private/tmp/zero-tests/core/preferences/i18n.js";

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

test("host translator does not absorb plugin-owned messages", () => {
  assert.equal(createTranslator("zh-CN")("launcher.title"), "launcher.title");
  assert.equal(createTranslator("en-US")("wallpaper.apply"), "wallpaper.apply");
});
