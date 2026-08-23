import assert from "node:assert/strict";
import test from "node:test";
import {
  CAPTURE_SELECTION_HANDLES,
  createFullImageSelection,
  imageBoundsToViewportBounds,
  moveSelectionBy,
  resolveSelectionNudge,
  resolveSelectPointerTarget,
  resolveSelectionDrag,
  resolveSelectionResize,
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

test("resizes from all eight handles while holding opposite edges fixed", () => {
  const initial = { x: 100, y: 120, width: 500, height: 300 };
  const expected = {
    "top-left": { x: 80, y: 90, width: 520, height: 330 },
    top: { x: 100, y: 90, width: 500, height: 330 },
    "top-right": { x: 100, y: 90, width: 540, height: 330 },
    right: { x: 100, y: 120, width: 540, height: 300 },
    "bottom-right": { x: 100, y: 120, width: 540, height: 350 },
    bottom: { x: 100, y: 120, width: 500, height: 350 },
    "bottom-left": { x: 80, y: 120, width: 520, height: 350 },
    left: { x: 80, y: 120, width: 520, height: 300 },
  };

  assert.deepEqual(
    CAPTURE_SELECTION_HANDLES.map(({ handle }) => handle),
    ["top-left", "top", "top-right", "right", "bottom-right", "bottom", "bottom-left", "left"],
  );
  assert.deepEqual(
    CAPTURE_SELECTION_HANDLES.map(({ horizontalEdge, verticalEdge }) => [horizontalEdge, verticalEdge]),
    [
      ["left", "top"],
      [null, "top"],
      ["right", "top"],
      ["right", null],
      ["right", "bottom"],
      [null, "bottom"],
      ["left", "bottom"],
      ["left", null],
    ],
  );
  for (const { handle } of CAPTURE_SELECTION_HANDLES) {
    assert.deepEqual(
      resolveSelectionResize(initial, handle, { x: handle.includes("left") ? -20 : 40, y: handle.includes("top") ? -30 : 50 }, imageSize),
      expected[handle],
      handle,
    );
  }
});

test("clamps resize handles to the source image and minimum dimensions", () => {
  const initial = { x: 100, y: 120, width: 500, height: 300 };

  assert.deepEqual(
    resolveSelectionResize(initial, "top-left", { x: -1000, y: -1000 }, imageSize),
    { x: 0, y: 0, width: 600, height: 420 },
  );
  assert.deepEqual(
    resolveSelectionResize(initial, "bottom-right", { x: 5000, y: 5000 }, imageSize),
    { x: 100, y: 120, width: 2460, height: 1320 },
  );
  assert.deepEqual(
    resolveSelectionResize(initial, "left", { x: 499, y: 0 }, imageSize),
    { x: 596, y: 120, width: 4, height: 300 },
  );
  assert.deepEqual(
    resolveSelectionResize(initial, "top", { x: 0, y: 299 }, imageSize),
    { x: 100, y: 416, width: 500, height: 4 },
  );
});

test("moves the complete selection by source pixels and clamps without resizing", () => {
  const selection = { x: 10, y: 20, width: 500, height: 300 };
  assert.deepEqual(moveSelectionBy(selection, { x: -1, y: 0 }, imageSize), {
    x: 9,
    y: 20,
    width: 500,
    height: 300,
  });
  assert.deepEqual(moveSelectionBy(selection, { x: 0, y: 1 }, imageSize), {
    x: 10,
    y: 21,
    width: 500,
    height: 300,
  });
  assert.deepEqual(
    moveSelectionBy({ x: 0, y: 0, width: 2560, height: 1440 }, { x: 1, y: 1 }, imageSize),
    { x: 0, y: 0, width: 2560, height: 1440 },
  );
  assert.deepEqual(
    moveSelectionBy({ x: 2060, y: 1140, width: 500, height: 300 }, { x: 5, y: 5 }, imageSize),
    { x: 2060, y: 1140, width: 500, height: 300 },
  );
});

test("resolves only unmodified Select-mode arrow nudges outside editing gestures", () => {
  const ready = { selectToolActive: true, editableTarget: false, composing: false, pointerActive: false };
  assert.deepEqual(resolveSelectionNudge({ ...ready, key: "ArrowLeft" }), { x: -1, y: 0 });
  assert.deepEqual(resolveSelectionNudge({ ...ready, key: "ArrowRight", repeat: true }), { x: 1, y: 0 });
  assert.deepEqual(resolveSelectionNudge({ ...ready, key: "ArrowUp" }), { x: 0, y: -1 });
  assert.deepEqual(resolveSelectionNudge({ ...ready, key: "ArrowDown" }), { x: 0, y: 1 });

  for (const blocked of [
    { ...ready, key: "ArrowLeft", selectToolActive: false },
    { ...ready, key: "ArrowLeft", editableTarget: true },
    { ...ready, key: "ArrowLeft", composing: true },
    { ...ready, key: "ArrowLeft", pointerActive: true },
    { ...ready, key: "ArrowLeft", metaKey: true },
    { ...ready, key: "ArrowLeft", ctrlKey: true },
    { ...ready, key: "ArrowLeft", altKey: true },
    { ...ready, key: "ArrowLeft", shiftKey: true },
    { ...ready, key: "Enter" },
  ]) {
    assert.equal(resolveSelectionNudge(blocked), null);
  }
});
