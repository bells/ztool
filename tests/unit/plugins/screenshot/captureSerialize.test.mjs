import assert from "node:assert/strict";
import test from "node:test";
import {
  buildPrepareScreenshotCommitPayload,
  buildScreenshotUploadOptions,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureSerialize.js";

test("builds the typed screenshot commit preparation payload", () => {
  assert.deepEqual(buildPrepareScreenshotCommitPayload("session-1", "save"), {
    input: {
      sessionId: "session-1",
      action: "save",
    },
  });
});

test("scopes a raw screenshot upload to its one-time lease", () => {
  assert.deepEqual(
    buildScreenshotUploadOptions({
      token: "upload-1",
      sessionId: "session-1",
      action: "pin",
      maxBytes: 1024,
      expiresAtMs: 1234,
    }),
    {
      headers: {
        "x-zero-screenshot-lease": "upload-1",
        "x-zero-screenshot-session": "session-1",
        "x-zero-screenshot-action": "pin",
      },
    },
  );
});
