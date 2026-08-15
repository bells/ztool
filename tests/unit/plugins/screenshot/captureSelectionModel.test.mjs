import assert from "node:assert/strict";
import test from "node:test";
import {
  createFullImageSelection,
  imageBoundsToViewportBounds,
  resolveSelectPointerTarget,
  resolveSelectionDrag,
  viewportPointToImagePoint,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureSelectionModel.js";

const imageSize = { width: 2560, height: 1440 };

test("defaults the real screenshot selection to the complete source image", () => {
  assert.deepEqual(createFullImageSelection(imageSize), {
    x: 0,
    y: 0,
    width: 2560,
    height: 1440,
  });
});

test("normalizes and clamps selection drags while preserving the previous valid selection", () => {
  const previous = { x: 0, y: 0, width: 2560, height: 1440 };

  assert.deepEqual(
    resolveSelectionDrag(previous, { x: 900, y: 700 }, { x: 100, y: 120 }, imageSize),
    { x: 100, y: 120, width: 800, height: 580 },
  );
  assert.deepEqual(
    resolveSelectionDrag(previous, { x: 2400, y: 1300 }, { x: 2800, y: 1600 }, imageSize),
    { x: 2400, y: 1300, width: 160, height: 140 },
  );
  assert.deepEqual(
    resolveSelectionDrag(previous, { x: 10, y: 10 }, { x: 12, y: 12 }, imageSize),
    previous,
  );
});

test("maps source-image selection bounds through contain scale and letterbox offsets", () => {
  const viewport = { width: 1200, height: 800 };
  const source = { width: 2400, height: 1200 };

  assert.deepEqual(
    imageBoundsToViewportBounds({ x: 200, y: 100, width: 1000, height: 500 }, source, viewport),
    { x: 100, y: 150, width: 500, height: 250 },
  );
  assert.deepEqual(viewportPointToImagePoint({ x: 600, y: 400 }, source, viewport), {
    x: 1200,
    y: 600,
  });
  assert.equal(viewportPointToImagePoint({ x: 600, y: 99 }, source, viewport), null);
});

test("keeps rectangle annotations distinct from screenshot selection drags", () => {
  const rectangle = {
    id: "rect-1",
    type: "rectangle",
    x: 50,
    y: 60,
    width: 200,
    height: 120,
    color: "#55f280",
    strokeWidth: 4,
  };

  assert.deepEqual(resolveSelectPointerTarget([rectangle], { x: 100, y: 100 }), {
    kind: "annotation",
    annotationId: "rect-1",
  });
  assert.deepEqual(resolveSelectPointerTarget([rectangle], { x: 400, y: 400 }), {
    kind: "selection",
  });
});
