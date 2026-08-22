import assert from "node:assert/strict";
import test from "node:test";
import {
  releaseCanvas,
  releaseDecodedImage,
  releaseObjectUrl,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureExport.js";

test("shrinks released canvas backing stores deterministically", () => {
  const canvas = { width: 8192, height: 4096 };
  releaseCanvas(canvas);
  assert.deepEqual(canvas, { width: 1, height: 1 });
  assert.doesNotThrow(() => releaseCanvas(null));
});

test("clears decoded image sources at terminal cleanup", () => {
  const image = { src: "blob:zero-screenshot" };
  releaseDecodedImage(image);
  assert.equal(image.src, "");
  assert.doesNotThrow(() => releaseDecodedImage(null));
});

test("ten frontend resource cycles end with no live canvas image or object URL", () => {
  const revoked = [];
  let liveObjectUrl = null;
  for (let cycle = 0; cycle < 10; cycle += 1) {
    const canvas = { width: 4096, height: 2160 };
    const image = { src: `blob:zero-screenshot-${cycle}` };
    liveObjectUrl = image.src;
    releaseCanvas(canvas);
    releaseDecodedImage(image);
    liveObjectUrl = releaseObjectUrl(liveObjectUrl, (url) => revoked.push(url));
    assert.deepEqual(canvas, { width: 1, height: 1 });
    assert.equal(image.src, "");
    assert.equal(liveObjectUrl, null);
  }
  assert.equal(revoked.length, 10);
  assert.equal(new Set(revoked).size, 10);
});
