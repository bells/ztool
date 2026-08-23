import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const captureSource = await readFile(
  new URL("../../../src/plugins/screenshot/capture/CaptureApp.tsx", import.meta.url),
  "utf8",
);
const captureStyles = await readFile(
  new URL("../../../src/App.css", import.meta.url),
  "utf8",
);

test("renders every capture tool and action with named Lucide icons", () => {
  for (const icon of [
    "MousePointer2",
    "Square",
    "Circle",
    "MoveUpRight",
    "Pencil",
    "Type",
    "Grid3X3",
    "Pin",
    "Undo2",
    "Redo2",
    "Trash2",
    "X",
    "Download",
    "Check",
  ]) {
    assert.match(captureSource, new RegExp(`\\b${icon}\\b`));
  }
  assert.doesNotMatch(
    captureSource,
    />\s*(Select|Rect|Arrow|Pen|Text|Mosaic|Pin|Undo|Redo|Del|Esc|Save|Copy)\s*</,
  );
});

test("keeps icon-only controls localized, stateful, and keyboard ordered", () => {
  assert.match(captureSource, /title=\{label\}/);
  assert.match(captureSource, /aria-label=\{label\}/);
  assert.match(captureSource, /aria-pressed=\{selected\}/);
  assert.match(captureSource, /disabled=\{history\.undoStack\.length === 0\}/);
  assert.match(captureSource, /disabled=\{history\.redoStack\.length === 0\}/);
  assert.match(captureSource, /disabled=\{!history\.selectedId\}/);
  assert.ok(captureSource.indexOf("<Undo2") < captureSource.indexOf("<Redo2"));
  assert.ok(captureSource.indexOf("<Redo2") < captureSource.indexOf("<Trash2"));
  assert.ok(captureSource.indexOf("<X") < captureSource.indexOf("<Download"));
  assert.ok(captureSource.indexOf("<Download") < captureSource.indexOf("<Check"));
});

test("crops commits to the real screenshot selection and anchors the toolbar to it", () => {
  assert.match(captureSource, /cropCanvasToPngBytes\(\s*sourceCanvas,\s*geometry\.bounds,\s*geometry\.cornerRadius/);
  assert.match(captureSource, /uploadSelection\(action, selection\)/);
  assert.match(captureSource, /releaseCanvas\(sourceCanvas\)/);
  assert.match(captureSource, /imageBoundsToViewportBounds\(/);
  assert.match(captureSource, /resolveCaptureToolbarPosition\(/);
  assert.doesNotMatch(captureSource, /annotationBounds\(/);
});

test("keeps multiline text focused and commits it by clicking outside", () => {
  assert.match(captureSource, /const textInputRef = useRef<HTMLTextAreaElement \| null>\(null\)/);
  assert.match(captureSource, /const textDraftRef = useRef<TextDraft \| null>\(null\)/);
  assert.match(captureSource, /requestAnimationFrame\(\(\) => \{/);
  assert.match(captureSource, /input\.focus\(\{ preventScroll: true \}\)/);
  assert.match(captureSource, /ref=\{textInputRef\}/);
  assert.match(captureSource, /<textarea/);
  assert.match(captureSource, /onBlur=\{commitTextDraft\}/);
  assert.match(
    captureSource,
    /if \(textDraftRef\.current\) \{\s*event\.preventDefault\(\);\s*commitTextDraft\(\);\s*return;/,
  );
  assert.match(captureSource, /const text = currentDraft\.value;\s*if \(text\.trim\(\)\.length === 0\)/);
  assert.doesNotMatch(captureSource, /event\.key === "Enter"/);
});

test("places the ellipse tool immediately after rectangle", () => {
  const rectangleIndex = captureSource.indexOf('tool: "rectangle"');
  const ellipseIndex = captureSource.indexOf('tool: "ellipse"');
  const arrowIndex = captureSource.indexOf('tool: "arrow"');
  assert.ok(rectangleIndex >= 0 && rectangleIndex < ellipseIndex && ellipseIndex < arrowIndex);
});

test("routes all eight resize handles through one typed selection interaction", () => {
  assert.match(captureSource, /type SelectionPointerInteraction =/);
  assert.match(captureSource, /kind: "pending-target"/);
  assert.match(captureSource, /kind: "create"/);
  assert.match(captureSource, /kind: "resize"/);
  assert.match(captureSource, /handle: SelectionResizeHandle/);
  assert.match(captureSource, /CAPTURE_SELECTION_HANDLES\.map/);
  assert.match(captureSource, /setPointerCapture\(event\.pointerId\)/);
  assert.match(captureSource, /resolveSelectionResize\(/);
  assert.match(captureSource, /onPointerCancel=\{handleResizePointerCancel\}/);
  assert.match(captureSource, /onLostPointerCapture=\{handleResizePointerCancel\}/);
  assert.match(captureSource, /selectionInteractionRef\.current = null/);
  assert.match(captureSource, /setSelectionDraft\(null\)/);
});

test("keeps resize feedback live but exports only the committed selection", () => {
  assert.match(captureSource, /const activeGeometry = selectionDraft \?\? selection/);
  assert.match(captureSource, /const activeSelection = activeGeometry\?\.bounds \?\? null/);
  assert.match(captureSource, /imageBoundsToViewportBounds\(\s*activeSelection/);
  assert.match(captureSource, /uploadSelection\(action, selection\)/);
  assert.match(captureSource, /tool === "select" \? " adjustable"/);
  assert.match(captureStyles, /\.capture-selection-frame\.adjustable \.capture-selection-handle\s*\{\s*pointer-events: auto/);
  assert.match(captureStyles, /\.capture-selection-handle\s*\{[^}]*width: 16px;[^}]*height: 16px;[^}]*pointer-events: none/s);
  assert.match(captureStyles, /\.capture-selection-handle\.top-left::after[^}]*border-radius: 50%/s);
  assert.match(captureStyles, /\.capture-selection-handle\.top::after[^}]*border-radius: 999px/s);
});

test("moves selections by source pixels only in an unmodified idle Select context", () => {
  assert.match(captureSource, /resolveSelectionNudge\(\{/);
  assert.match(captureSource, /selectToolActive: tool === "select"/);
  assert.match(captureSource, /composing: event\.isComposing \|\| Boolean\(textDraftRef\.current\)/);
  assert.match(captureSource, /pointerActive: Boolean\(selectionInteractionRef\.current\)/);
  assert.match(captureSource, /moveSelectionBy\(current\.bounds, selectionDelta/);
  assert.match(captureSource, /repeat: event\.repeat/);
});
