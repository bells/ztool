import assert from "node:assert/strict";
import test from "node:test";
import {
  CAPTURE_TOOLBAR_GAP,
  CAPTURE_TOOLBAR_INSET,
  CAPTURE_TOOLBAR_SAFE_MARGIN,
  resolveCaptureToolbarPosition,
} from "/private/tmp/zero-capture-test/captureToolbarModel.js";

const toolbar = { width: 600, height: 56 };
const viewport = { width: 1440, height: 900 };

test("places the toolbar immediately below a normal selection", () => {
  assert.deepEqual(
    resolveCaptureToolbarPosition({ x: 200, y: 100, width: 900, height: 500 }, toolbar, viewport),
    {
      left: 500,
      top: 600 + CAPTURE_TOOLBAR_GAP,
      placement: "outside-bottom",
    },
  );
});

test("places the toolbar inside a full-screen or bottom-edge selection", () => {
  assert.deepEqual(
    resolveCaptureToolbarPosition({ x: 0, y: 0, width: 1440, height: 900 }, toolbar, viewport),
    {
      left: 1440 - toolbar.width - CAPTURE_TOOLBAR_SAFE_MARGIN,
      top: 900 - toolbar.height - CAPTURE_TOOLBAR_INSET,
      placement: "inside-bottom",
    },
  );
});

test("places the toolbar above a short bottom selection", () => {
  assert.deepEqual(
    resolveCaptureToolbarPosition({ x: 300, y: 780, width: 800, height: 60 }, toolbar, viewport),
    {
      left: 500,
      top: 780 - toolbar.height - CAPTURE_TOOLBAR_GAP,
      placement: "outside-top",
    },
  );
});

test("clamps toolbar alignment at both horizontal viewport edges", () => {
  assert.equal(
    resolveCaptureToolbarPosition({ x: 0, y: 100, width: 300, height: 400 }, toolbar, viewport).left,
    CAPTURE_TOOLBAR_SAFE_MARGIN,
  );
  assert.equal(
    resolveCaptureToolbarPosition({ x: 1400, y: 100, width: 100, height: 400 }, toolbar, viewport).left,
    viewport.width - toolbar.width - CAPTURE_TOOLBAR_SAFE_MARGIN,
  );
});

test("recalculates from current toolbar and viewport dimensions", () => {
  const selection = { x: 200, y: 100, width: 900, height: 500 };
  const initial = resolveCaptureToolbarPosition(selection, toolbar, viewport);
  const resized = resolveCaptureToolbarPosition(
    selection,
    { width: 480, height: 64 },
    { width: 1200, height: 660 },
  );

  assert.notDeepEqual(resized, initial);
  assert.equal(resized.left, 620);
  assert.equal(resized.placement, "inside-bottom");
});
