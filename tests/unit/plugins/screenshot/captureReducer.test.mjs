import assert from "node:assert/strict";
import test from "node:test";
import {
  captureReducer,
  initialHistoryState,
} from "/private/tmp/zero-tests/plugins/screenshot/capture/captureReducer.js";

const rect = {
  id: "rect-1",
  type: "rectangle",
  x: 10,
  y: 20,
  width: 120,
  height: 80,
  color: "#55f280",
  strokeWidth: 4,
};

test("adds, selects, deletes, undoes, and redoes annotations", () => {
  const added = captureReducer(initialHistoryState, { type: "add", annotation: rect });
  assert.equal(added.annotations.length, 1);
  assert.equal(added.selectedId, "rect-1");

  const removed = captureReducer(added, { type: "removeSelected" });
  assert.deepEqual(removed.annotations, []);
  assert.equal(removed.selectedId, null);

  const undone = captureReducer(removed, { type: "undo" });
  assert.deepEqual(undone.annotations, [rect]);

  const redone = captureReducer(undone, { type: "redo" });
  assert.deepEqual(redone.annotations, []);
});

test("clear records undo history only when annotations exist", () => {
  const emptyCleared = captureReducer(initialHistoryState, { type: "clear" });
  const emptyUndone = captureReducer(emptyCleared, { type: "undo" });
  assert.deepEqual(emptyUndone.annotations, []);

  const added = captureReducer(initialHistoryState, { type: "add", annotation: rect });
  const cleared = captureReducer(added, { type: "clear" });
  assert.deepEqual(cleared.annotations, []);
  const clearUndone = captureReducer(cleared, { type: "undo" });
  assert.deepEqual(clearUndone.annotations, [rect]);
});
