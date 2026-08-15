import assert from "node:assert/strict";
import test from "node:test";
import {
  buildCommitScreenshotPayload,
  buildPinScreenshotPayload,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureSerialize.js";

test("builds commit payload using Rust command field names", () => {
  assert.deepEqual(
    buildCommitScreenshotPayload({
      sessionId: "session-1",
      action: "save",
      pngBase64: "data:image/png;base64,abc",
      savePath: "/tmp/capture.png",
    }),
    {
      input: {
        session_id: "session-1",
        action: "save",
        png_base64: "data:image/png;base64,abc",
        save_path: "/tmp/capture.png",
      },
    },
  );
});

test("builds pin payload using Rust command field names", () => {
  assert.deepEqual(buildPinScreenshotPayload("session-1", "png-data"), {
    input: {
      session_id: "session-1",
      png_base64: "png-data",
    },
  });
});
