import assert from "node:assert/strict";
import test from "node:test";
import {
  combineSurfaceActivity,
  createSurfaceActivityModel,
  SURFACE_ACTIVITY_EVENT,
} from "/private/tmp/zero-tests/core/windowing/surfaceActivityCore.js";

test("combines native visibility with document visibility without using focus", () => {
  assert.equal(combineSurfaceActivity("active", "visible"), "active");
  assert.equal(combineSurfaceActivity("active", "hidden"), "hidden");
  assert.equal(combineSurfaceActivity("hidden", "visible"), "hidden");
  assert.equal(combineSurfaceActivity("disposed", "visible"), "disposed");
  assert.equal(SURFACE_ACTIVITY_EVENT, "zero://surface-activity");
});

test("filters other windows and ignores late events after listener cleanup", () => {
  const changes = [];
  const model = createSurfaceActivityModel("tray", "visible", (state) => changes.push(state));

  model.acceptNative({ label: "main", state: "active" });
  assert.equal(model.snapshot(), "hidden");
  model.acceptNative({ label: "tray", state: "active" });
  assert.equal(model.snapshot(), "active");
  model.setDocumentVisibility("hidden");
  assert.equal(model.snapshot(), "hidden");
  model.setDocumentVisibility("visible");
  assert.equal(model.snapshot(), "active");
  model.dispose();
  model.acceptNative({ label: "tray", state: "active" });

  assert.equal(model.snapshot(), "disposed");
  assert.deepEqual(changes, ["active", "hidden", "active", "disposed"]);
});

test("multiple surfaces retain isolated lifecycle state", () => {
  const tray = createSurfaceActivityModel("tray", "visible", () => undefined);
  const main = createSurfaceActivityModel("main", "visible", () => undefined);
  const trayShown = { label: "tray", state: "active" };

  tray.acceptNative(trayShown);
  main.acceptNative(trayShown);

  assert.equal(tray.snapshot(), "active");
  assert.equal(main.snapshot(), "hidden");
});
