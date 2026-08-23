import assert from "node:assert/strict";
import test from "node:test";
import {
  TARGET_DRAG_THRESHOLD_CSS_PX,
  hasExceededTargetDragThreshold,
  normalizeScreenshotTargets,
  resolveScreenshotTargetAtPoint,
  resolveStableTargetClick,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureTargetModel.js";

const imageSize = { width: 1440, height: 900 };
const targets = [
  { id: "target-front", kind: "window", bounds: { x: 200, y: 100, width: 700, height: 500 } },
  { id: "target-back", kind: "window", bounds: { x: 100, y: 50, width: 1100, height: 700 } },
];

test("keeps valid native target order and clips source-pixel bounds", () => {
  assert.deepEqual(
    normalizeScreenshotTargets(
      [
        ...targets,
        { id: "partial", kind: "window", bounds: { x: 1300, y: 850, width: 300, height: 200 } },
        { id: "invalid", kind: "window", bounds: { x: 10, y: 10, width: 0, height: 10 } },
      ],
      imageSize,
    ),
    [
      ...targets,
      { id: "partial", kind: "window", bounds: { x: 1300, y: 850, width: 140, height: 50 } },
    ],
  );
});

test("resolves the frontmost window and falls back to the complete image", () => {
  const normalized = normalizeScreenshotTargets(targets, imageSize);
  assert.equal(resolveScreenshotTargetAtPoint(normalized, { x: 300, y: 200 }, imageSize).id, "target-front");
  assert.equal(resolveScreenshotTargetAtPoint(normalized, { x: 1100, y: 650 }, imageSize).id, "target-back");
  assert.deepEqual(resolveScreenshotTargetAtPoint(normalized, { x: 20, y: 20 }, imageSize), {
    id: "screen",
    kind: "screen",
    bounds: { x: 0, y: 0, width: 1440, height: 900 },
  });
  assert.equal(resolveScreenshotTargetAtPoint(normalized, { x: -1, y: 20 }, imageSize), null);
});

test("uses a four CSS-pixel threshold before free selection takes over", () => {
  assert.equal(TARGET_DRAG_THRESHOLD_CSS_PX, 4);
  assert.equal(hasExceededTargetDragThreshold({ x: 10, y: 10 }, { x: 13, y: 12 }), false);
  assert.equal(hasExceededTargetDragThreshold({ x: 10, y: 10 }, { x: 14.1, y: 10 }), true);
});

test("commits only a stable click candidate", () => {
  const front = resolveScreenshotTargetAtPoint(targets, { x: 300, y: 200 }, imageSize);
  const back = resolveScreenshotTargetAtPoint(targets, { x: 1100, y: 650 }, imageSize);
  assert.deepEqual(resolveStableTargetClick(front, front, false), front?.bounds);
  assert.equal(resolveStableTargetClick(front, back, false), null);
  assert.equal(resolveStableTargetClick(front, front, true), null);
  assert.equal(resolveStableTargetClick(null, front, false), null);
});
