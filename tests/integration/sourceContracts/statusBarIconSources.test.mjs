import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import {
  STATUS_BAR_ICON_FILES,
  statusBarIconSource,
} from "/private/tmp/zero-tests/components/statusBarIconSources.js";

const expectedFiles = {
  zero: "zero.svg",
  launch: "zero-launch.svg",
  "caffeine-empty": "zero-awake.svg",
  "caffeine-full": "zero-awake-active.svg",
  screenshot: "zero-snap.svg",
  paper: "zero-paper.svg",
  extension: "extension.svg",
};

test("maps every first-party and compatibility icon id to canonical artwork", () => {
  assert.deepEqual(STATUS_BAR_ICON_FILES, expectedFiles);

  for (const [icon, file] of Object.entries(expectedFiles)) {
    assert.equal(statusBarIconSource(icon).endsWith(`/assets/icons/${file}`), true);
  }
});

test("keeps legacy semantic ids on the new Zero artwork", () => {
  assert.equal(STATUS_BAR_ICON_FILES.screenshot, "zero-snap.svg");
  assert.equal(STATUS_BAR_ICON_FILES["caffeine-empty"], "zero-awake.svg");
  assert.equal(
    STATUS_BAR_ICON_FILES["caffeine-full"],
    "zero-awake-active.svg",
  );
});

test("CSS masks inherit their surface foreground in light and dark themes", () => {
  const css = fs.readFileSync("src/App.css", "utf8");

  assert.match(
    css,
    /\.status-bar-glyph\s*\{[^}]*color:\s*inherit;/s,
  );
  assert.match(
    css,
    /\.status-bar-glyph-mask\s*\{[^}]*background-color:\s*currentColor;/s,
  );
});
