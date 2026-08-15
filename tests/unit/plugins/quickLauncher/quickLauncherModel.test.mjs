import assert from "node:assert/strict";
import test from "node:test";
import {
  canActivateLauncherItem,
  defaultLauncherSelection,
  groupLauncherResults,
  moveLauncherSelection,
  reconcileLauncherSelection,
  shouldDismissLauncher,
} from "/private/tmp/zero-tests/plugins/quickLauncher/quickLauncherModel.js";

const items = [
  { id: "a", kind: "application", title: "Alpha" },
  { id: "b", kind: "systemSetting", title: "Bluetooth" },
  { id: "c", kind: "application", title: "Code" },
];

test("launcher selection wraps in both directions", () => {
  assert.equal(defaultLauncherSelection(items), "a");
  assert.equal(moveLauncherSelection("a", items, -1), "c");
  assert.equal(moveLauncherSelection("c", items, 1), "a");
  assert.equal(moveLauncherSelection(null, items, 1), "a");
  assert.equal(moveLauncherSelection(null, items, -1), "c");
  assert.equal(moveLauncherSelection(null, [], 1), null);
  assert.equal(moveLauncherSelection("a", [items[0]], 1), "a");
});

test("launcher selection survives replacement only while its item exists", () => {
  assert.equal(reconcileLauncherSelection("c", [items[2], items[0]]), "c");
  assert.equal(reconcileLauncherSelection("b", [items[2], items[0]]), "c");
  assert.equal(reconcileLauncherSelection("a", []), null);
});

test("launcher grouping and activation guard stay deterministic", () => {
  const groups = groupLauncherResults(items);
  assert.deepEqual(groups.applications.map((item) => item.id), ["a", "c"]);
  assert.deepEqual(groups.settings.map((item) => item.id), ["b"]);
  assert.equal(canActivateLauncherItem("a", items, null), true);
  assert.equal(canActivateLauncherItem("a", items, "a"), false);
  assert.equal(canActivateLauncherItem("missing", items, null), false);
});

test("only the floating surface dismisses and blur waits for activation", () => {
  assert.equal(shouldDismissLauncher("panel", "escape", false), false);
  assert.equal(shouldDismissLauncher("floating", "escape", false), true);
  assert.equal(shouldDismissLauncher("floating", "blur", true), false);
  assert.equal(shouldDismissLauncher("floating", "blur", false), true);
  assert.equal(shouldDismissLauncher("floating", "activationSuccess", true), true);
});
