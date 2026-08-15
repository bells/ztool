import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  CANONICAL_IDENTITY_FILES,
  ICON_DIR,
  STATUS_ICON_SPECS,
  TRAY_ICON_SPECS,
  buildContactSheetSvg,
  normalizeSvg,
  readPngMetadata,
  readAttribute,
  validateGeneratedAssets,
  validateIconSystem,
  validateStatusSvg,
  validateXmlStructure,
} from "../../../scripts/icon-system.mjs";

test("canonical Zero icon inventory is complete and structurally valid", () => {
  assert.deepEqual(validateIconSystem(), []);
  assert.deepEqual(CANONICAL_IDENTITY_FILES, [
    "zero.svg",
    "zero-launch.svg",
    "zero-snap.svg",
    "zero-awake.svg",
    "zero-paper.svg",
  ]);
  assert.equal(STATUS_ICON_SPECS.length, 6);
  assert.equal(TRAY_ICON_SPECS.length, 7);
});

test("each status-compatible source uses the shared 24px currentColor contract", () => {
  for (const { file } of STATUS_ICON_SPECS) {
    const source = fs.readFileSync(path.join(ICON_DIR, file), "utf8");
    assert.equal(readAttribute(source, "viewBox"), "0 0 24 24");
    assert.deepEqual(validateStatusSvg(source, file), []);
  }
});

test("XML structure validation rejects unbalanced source", () => {
  assert.deepEqual(validateXmlStructure("<svg><path/></svg>"), []);
  assert.match(validateXmlStructure("<svg><path></svg>")[0], /mismatched closing tag/);
});

test("Zero uses the canonical 45 degree slash", () => {
  const source = fs.readFileSync(path.join(ICON_DIR, "zero.svg"), "utf8");
  assert.match(source, /<circle cx="12" cy="12" r="7\.5"\/>/);
  assert.match(source, /<path d="M5\.5 18\.5 18\.5 5\.5"\/>/);
});

test("Awake active state adds only the bounded liquid-level mark", () => {
  const base = fs.readFileSync(path.join(ICON_DIR, "zero-awake.svg"), "utf8");
  const active = fs.readFileSync(
    path.join(ICON_DIR, "zero-awake-active.svg"),
    "utf8",
  );
  const withoutStateMark = active.replace(/\s*<path d="M6\.5 14h7"\/>/, "");

  assert.match(active, /<path d="M6\.5 14h7"\/>/);
  assert.equal(normalizeSvg(withoutStateMark), normalizeSvg(base));
});

test("contact sheet includes every target size, theme, and identity", () => {
  const sheet = buildContactSheetSvg();

  for (const size of [16, 18, 22, 24, 128]) {
    assert.match(sheet, new RegExp(`${size}px`));
  }
  assert.match(sheet, /512px master/);
  assert.match(sheet, />Light</);
  assert.match(sheet, />Dark</);
  for (const { label } of STATUS_ICON_SPECS) {
    assert.ok(sheet.includes(label));
  }
  assert.deepEqual(validateXmlStructure(sheet), []);
});

test("generated tray and application assets have valid tracked formats", () => {
  assert.deepEqual(validateGeneratedAssets(), []);

  const metadata = readPngMetadata(
    fs.readFileSync(
      path.join(
        path.dirname(ICON_DIR),
        "..",
        "..",
        "src-tauri",
        "icons",
        "app-icon.png",
      ),
    ),
  );
  assert.deepEqual(metadata, {
    width: 512,
    height: 512,
    bitDepth: 8,
    colorType: 6,
    hasAlpha: true,
  });
});
