import assert from "node:assert/strict";
import test from "node:test";
import {
  commitSelectionDimension,
  normalizeSelectionGeometry,
  resolveGeometryControlPosition,
  selectionGeometryFromBounds,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureGeometryModel.js";

const imageSize = { width: 1200, height: 800 };

test("normalizes bounds and clamps radius to half the shorter edge", () => {
  assert.deepEqual(
    normalizeSelectionGeometry(
      { bounds: { x: 100, y: 80, width: 300, height: 120 }, cornerRadius: 90 },
      imageSize,
    ),
    { bounds: { x: 100, y: 80, width: 300, height: 120 }, cornerRadius: 60 },
  );
  assert.deepEqual(selectionGeometryFromBounds({ x: 10, y: 20, width: 100, height: 80 }), {
    bounds: { x: 10, y: 20, width: 100, height: 80 },
    cornerRadius: 0,
  });
});

test("commits width and height from a fixed upper-left corner", () => {
  const geometry = { bounds: { x: 100, y: 80, width: 300, height: 200 }, cornerRadius: 80 };
  assert.deepEqual(commitSelectionDimension(geometry, "width", "450", imageSize), {
    valid: true,
    geometry: { bounds: { x: 100, y: 80, width: 450, height: 200 }, cornerRadius: 80 },
  });
  assert.deepEqual(commitSelectionDimension(geometry, "height", "40", imageSize), {
    valid: true,
    geometry: { bounds: { x: 100, y: 80, width: 300, height: 40 }, cornerRadius: 20 },
  });
  assert.equal(commitSelectionDimension(geometry, "width", "", imageSize).valid, false);
  assert.equal(commitSelectionDimension(geometry, "height", "3", imageSize).valid, false);
  assert.equal(commitSelectionDimension(geometry, "width", "12.5", imageSize).valid, false);
  assert.equal(commitSelectionDimension(geometry, "height", "Infinity", imageSize).valid, false);
  assert.deepEqual(commitSelectionDimension(geometry, "width", "invalid", imageSize).geometry, geometry);
});

test("limits oversized dimensions to image space", () => {
  const geometry = { bounds: { x: 1100, y: 700, width: 80, height: 80 }, cornerRadius: 30 };
  assert.deepEqual(commitSelectionDimension(geometry, "width", "9999", imageSize).geometry, {
    bounds: { x: 1100, y: 700, width: 100, height: 80 },
    cornerRadius: 30,
  });
});

test("positions geometry controls above, inside, and viewport-clamped", () => {
  const control = { width: 260, height: 38 };
  const viewport = { width: 1000, height: 700 };
  assert.deepEqual(resolveGeometryControlPosition({ x: 200, y: 100, width: 400, height: 300 }, control, viewport), {
    left: 200,
    top: 54,
    placement: "outside-top",
  });
  assert.equal(
    resolveGeometryControlPosition({ x: 20, y: 5, width: 400, height: 300 }, control, viewport).placement,
    "inside-top",
  );
  assert.equal(
    resolveGeometryControlPosition({ x: 960, y: 5, width: 20, height: 20 }, control, viewport).left,
    732,
  );
  const tinyViewport = resolveGeometryControlPosition(
    { x: 0, y: 0, width: 12, height: 12 },
    { width: 260, height: 38 },
    { width: 220, height: 44 },
  );
  assert.equal(tinyViewport.placement, "viewport-clamped");
  assert.ok(tinyViewport.left >= 8 && tinyViewport.top >= 8);
});
