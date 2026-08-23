import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { resolveAppSurface } from "/private/tmp/zero-tests/appShell/appSurface.js";

test("routes known Tauri window labels to app surfaces", () => {
  assert.equal(resolveAppSurface("tray"), "tray");
  assert.equal(resolveAppSurface("main"), "main");
  assert.equal(resolveAppSurface("preferences"), "preferences");
  assert.equal(resolveAppSurface("about"), "about");
  assert.equal(resolveAppSurface("capture"), "capture");
  assert.equal(resolveAppSurface("launcher"), "launcher");
  assert.equal(resolveAppSurface("paper"), "paper");
  assert.equal(resolveAppSurface("snap-menu"), "snap-menu");
  assert.equal(resolveAppSurface("pin-123"), "pin");
});

test("falls back unknown window labels to the tray surface", () => {
  assert.equal(resolveAppSurface("unexpected-window"), "tray");
  assert.equal(resolveAppSurface(""), "tray");
});

test("Tauri capability includes every routed trusted window", () => {
  const capability = JSON.parse(
    readFileSync("src-tauri/capabilities/default.json", "utf8"),
  );
  for (const label of ["tray", "main", "preferences", "about", "capture", "launcher", "paper", "snap-menu"]) {
    assert.ok(capability.windows.includes(label), `${label} must be capability-scoped`);
  }
  assert.ok(capability.windows.includes("pin-*"));
});
