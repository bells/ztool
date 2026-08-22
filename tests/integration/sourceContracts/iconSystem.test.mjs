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

test("Zero uses one shared solid top-narrow duck-egg with a negative 45 degree slash", () => {
  const source = fs.readFileSync(path.join(ICON_DIR, "zero.svg"), "utf8");
  const appSource = fs.readFileSync(
    path.join(ICON_DIR, "zero-app-icon.svg"),
    "utf8",
  );

  const eggPath = "M12 2.1C9 2.1 6.48 5.29 5.04 9.14C2.64 15.41 5.28 21.24 12 21.9C18.72 21.24 21.36 15.41 18.96 9.14C17.52 5.29 15 2.1 12 2.1ZM8.54 17.34L17.34 8.54A1.32 1.32 0 0 0 15.46 6.66L6.66 15.46A1.32 1.32 0 0 0 8.54 17.34Z";

  assert.ok(source.includes(`<path d="${eggPath}`));
  assert.match(source, /fill="currentColor" stroke="none" fill-rule="evenodd"/);
  assert.doesNotMatch(source, /<(?:circle|ellipse)\b/);
  assert.match(
    appSource,
    new RegExp(`<path d="${eggPath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`),
  );
  assert.match(appSource, /fill="#FFFFFF" fill-rule="evenodd" transform="translate\(50 50\) scale\(17\.1666667\)"/);
  assert.doesNotMatch(appSource, /<(?:circle|ellipse)\b/);
});

test("Awake is steam-free and active state adds only the bounded liquid-level mark", () => {
  const base = fs.readFileSync(path.join(ICON_DIR, "zero-awake.svg"), "utf8");
  const active = fs.readFileSync(
    path.join(ICON_DIR, "zero-awake-active.svg"),
    "utf8",
  );
  const withoutStateMark = active.replace(/\s*<path d="M6\.5 10h7"\/>/, "");

  for (const source of [base, active]) {
    assert.doesNotMatch(source, /M10 7c-2-2 2-2 0-5/);
    assert.match(source, /<path d="M4 6h12v7a6 6 0 0 1-12 0V6Z"\/>/);
    assert.match(source, /<path d="M16 8h1\.5a3 3 0 0 1 0 6H16"\/>/);
    assert.match(source, /<path d="M3 20h17"\/>/);
  }
  assert.match(active, /<path d="M6\.5 10h7"\/>/);
  assert.equal(normalizeSvg(withoutStateMark), normalizeSvg(base));
});

test("Launch uses the canonical sparse rocket instead of the terminal prompt", () => {
  const source = fs.readFileSync(
    path.join(ICON_DIR, "zero-launch.svg"),
    "utf8",
  );

  assert.match(
    source,
    /<path d="M14 5c2\.5-1\.5 4\.5-1\.5 6-1-\.5 4-2\.5 7\.5-6 10\.5L9\.5 10C11 8 12\.5 6\.5 14 5Z"\/>/,
  );
  assert.match(source, /<path d="M10 10H6l-2 4 6 1"\/>/);
  assert.match(source, /<path d="M14 14v4l-4 2-1-6"\/>/);
  assert.match(source, /<path d="m9 16-4 4"\/>/);
  assert.doesNotMatch(source, /m5 6 6 6-6 6/);
  assert.doesNotMatch(source, /M13\.5 18H19/);
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
