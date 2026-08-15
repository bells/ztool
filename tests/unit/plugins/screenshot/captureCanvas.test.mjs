import assert from "node:assert/strict";
import test from "node:test";
import {
  annotationBounds,
  clampBounds,
  drawAnnotations,
  hitTestAnnotation,
  isAnnotationLargeEnough,
  normalizeRect,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureCanvas.js";

test("normalizes and clamps rectangles", () => {
  assert.deepEqual(normalizeRect({ x: 80, y: 70 }, { x: 10, y: 15 }), {
    x: 10,
    y: 15,
    width: 70,
    height: 55,
  });
  assert.deepEqual(clampBounds({ x: -5, y: 10, width: 30, height: 20 }, 100, 100), {
    x: 0,
    y: 10,
    width: 25,
    height: 20,
  });
  assert.equal(clampBounds({ x: 110, y: 10, width: 20, height: 20 }, 100, 100), null);
});

test("computes text bounds and hit tests annotations from topmost to bottom", () => {
  const annotations = [
    {
      id: "rect-1",
      type: "rectangle",
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      color: "#55f280",
      strokeWidth: 4,
    },
    {
      id: "text-1",
      type: "text",
      x: 20,
      y: 35,
      text: "Hello",
      fontSize: 20,
      color: "#55f280",
      strokeWidth: 2,
    },
  ];

  assert.deepEqual(annotationBounds(annotations[1]), {
    x: 20,
    y: 15,
    width: 56,
    height: 27,
  });
  assert.equal(hitTestAnnotation(annotations, { x: 22, y: 20 })?.id, "text-1");
  assert.equal(hitTestAnnotation(annotations, { x: 90, y: 90 })?.id, "rect-1");
  assert.equal(hitTestAnnotation(annotations, { x: 200, y: 200 }), null);
});

test("computes multiline text and ellipse bounds", () => {
  assert.deepEqual(
    annotationBounds({
      id: "ellipse-1",
      type: "ellipse",
      x: 12,
      y: 18,
      width: 80,
      height: 44,
      color: "#55f280",
      strokeWidth: 4,
    }),
    { x: 12, y: 18, width: 80, height: 44 },
  );

  assert.deepEqual(
    annotationBounds({
      id: "text-multiline",
      type: "text",
      x: 20,
      y: 35,
      text: "Hello\n世界",
      fontSize: 20,
      color: "#55f280",
      strokeWidth: 2,
    }),
    { x: 20, y: 15, width: 56, height: 54 },
  );
});

test("draws an ellipse and every line of multiline text", () => {
  const ellipseCalls = [];
  const fillTextCalls = [];
  const context = {
    save() {},
    restore() {},
    beginPath() {},
    stroke() {},
    ellipse(...args) {
      ellipseCalls.push(args);
    },
    fillText(...args) {
      fillTextCalls.push(args);
    },
  };

  drawAnnotations(context, [
    {
      id: "ellipse-1",
      type: "ellipse",
      x: 12,
      y: 18,
      width: 80,
      height: 44,
      color: "#55f280",
      strokeWidth: 4,
    },
    {
      id: "text-multiline",
      type: "text",
      x: 20,
      y: 35,
      text: "Hello\n世界",
      fontSize: 20,
      color: "#55f280",
      strokeWidth: 2,
    },
  ]);

  assert.deepEqual(ellipseCalls, [[52, 40, 40, 22, 0, 0, Math.PI * 2]]);
  assert.deepEqual(fillTextCalls, [
    ["Hello", 20, 35],
    ["世界", 20, 62],
  ]);
});

test("treats axis-aligned arrows as large enough by distance", () => {
  assert.equal(
    isAnnotationLargeEnough({
      id: "arrow-horizontal",
      type: "arrow",
      from: { x: 10, y: 20 },
      to: { x: 70, y: 20 },
      color: "#55f280",
      strokeWidth: 5,
    }),
    true,
  );
  assert.equal(
    isAnnotationLargeEnough({
      id: "arrow-vertical",
      type: "arrow",
      from: { x: 10, y: 20 },
      to: { x: 10, y: 70 },
      color: "#55f280",
      strokeWidth: 5,
    }),
    true,
  );
  assert.equal(
    isAnnotationLargeEnough({
      id: "arrow-tiny",
      type: "arrow",
      from: { x: 10, y: 20 },
      to: { x: 12, y: 20 },
      color: "#55f280",
      strokeWidth: 5,
    }),
    false,
  );
});
