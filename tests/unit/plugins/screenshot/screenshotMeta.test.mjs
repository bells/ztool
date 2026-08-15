import assert from "node:assert/strict";
import test from "node:test";
import {
  SCREENSHOT_SHORTCUT,
  SCREENSHOT_USAGE_ITEMS,
} from "/private/tmp/zero-tests/plugins/screenshot/screenshotMeta.js";

test("uses a global shortcut suitable for macOS and Windows", () => {
  assert.equal(SCREENSHOT_SHORTCUT.accelerator, "CommandOrControl+Shift+A");
  assert.equal(SCREENSHOT_SHORTCUT.display, "⌘/Ctrl + Shift + A");
});

test("describes screenshot usage without mentioning source inspirations", () => {
  const text = SCREENSHOT_USAGE_ITEMS.join(" ");

  assert.match(text, /快捷键/);
  assert.match(text, /复制/);
  assert.match(text, /保存/);
  assert.doesNotMatch(text.toLowerCase(), /wechat|微信/);
});
