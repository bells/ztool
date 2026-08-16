import assert from "node:assert/strict";
import test from "node:test";
import {
  createFileTranslator,
  fileErrorTranslationKey,
  fileMessages,
} from "/private/tmp/zero-tests/plugins/file/i18n.js";

test("File exposes the same complete vocabulary in Chinese and English", () => {
  const zhKeys = Object.keys(fileMessages["zh-CN"]).sort();
  const enKeys = Object.keys(fileMessages["en-US"]).sort();
  assert.deepEqual(zhKeys, enKeys);
  assert.ok(zhKeys.includes("action.convertAll"));
  assert.ok(zhKeys.includes("provider.pdfToDocxUnavailable"));
  assert.ok(zhKeys.includes("error.ocrRequired"));
});

test("language changes relocalize visible File copy without touching job state", () => {
  const zh = createFileTranslator("zh-CN");
  const en = createFileTranslator("en-US");
  assert.equal(zh("state.running"), "转换中");
  assert.equal(en("state.running"), "Converting");
  assert.equal(zh(fileErrorTranslationKey("passwordRequired")).includes("密码"), true);
  assert.equal(en(fileErrorTranslationKey("passwordRequired")).includes("password"), true);
});
