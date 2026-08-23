import assert from "node:assert/strict";
import test from "node:test";
import {
  applyRoundedCanvasMask,
  isPointInsideRoundedRect,
  releaseCanvas,
  releaseDecodedImage,
  releaseObjectUrl,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureExport.js";

test("applies rounded export alpha with destination-in composition", () => {
  const calls = [];
  const context = {
    globalCompositeOperation: "source-over",
    fillStyle: "black",
    beginPath: () => calls.push("beginPath"),
    roundRect: (...args) => calls.push(["roundRect", ...args]),
    fill: () => calls.push("fill"),
    save: () => calls.push("save"),
    restore: () => calls.push("restore"),
  };
  applyRoundedCanvasMask(context, 200, 100, 30);
  assert.equal(context.globalCompositeOperation, "destination-in");
  assert.deepEqual(calls, ["save", "beginPath", ["roundRect", 0, 0, 200, 100, 30], "fill", "restore"]);
  assert.equal(isPointInsideRoundedRect({ x: 0, y: 0 }, 200, 100, 30), false);
  assert.equal(isPointInsideRoundedRect({ x: 200, y: 0 }, 200, 100, 30), false);
  assert.equal(isPointInsideRoundedRect({ x: 0, y: 100 }, 200, 100, 30), false);
  assert.equal(isPointInsideRoundedRect({ x: 200, y: 100 }, 200, 100, 30), false);
  assert.equal(isPointInsideRoundedRect({ x: 30, y: 30 }, 200, 100, 30), true);
  assert.equal(isPointInsideRoundedRect({ x: 100, y: 50 }, 200, 100, 30), true);
});

test("preserves rectangular corners when radius is zero and clamps excessive radii", () => {
  assert.equal(isPointInsideRoundedRect({ x: 0, y: 0 }, 200, 100, 0), true);
  assert.equal(isPointInsideRoundedRect({ x: 0, y: 0 }, 200, 100, 999), false);
  assert.equal(isPointInsideRoundedRect({ x: 50, y: 50 }, 200, 100, 999), true);
});

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
