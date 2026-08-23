import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const [capture, controls, exportSource, styles, types, rust, cargo, messages] = await Promise.all([
  readFile(new URL("../../../src/plugins/screenshot/capture/CaptureApp.tsx", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/capture/SelectionGeometryControls.tsx", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/capture/captureExport.ts", import.meta.url), "utf8"),
  readFile(new URL("../../../src/App.css", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/capture/captureTypes.ts", import.meta.url), "utf8"),
  readFile(new URL("../../../src-tauri/src/services/screenshot.rs", import.meta.url), "utf8"),
  readFile(new URL("../../../src-tauri/Cargo.toml", import.meta.url), "utf8"),
  readFile(new URL("../../../src/plugins/screenshot/i18n.ts", import.meta.url), "utf8"),
]);

test("keeps the screenshot target contract symmetric and privacy-minimal", () => {
  for (const field of ["id", "kind", "bounds"]) {
    assert.match(types, new RegExp(`interface ScreenshotTargetCandidate[\\s\\S]*?\\b${field}:`));
    assert.match(rust, new RegExp(`struct ScreenshotTargetCandidate[\\s\\S]*?pub ${field}:`));
  }
  const tsCandidate = types.match(/interface ScreenshotTargetCandidate \{[\s\S]*?\n\}/)?.[0] ?? "";
  const rustCandidate = rust.match(/struct ScreenshotTargetCandidate \{[\s\S]*?\n\}/)?.[0] ?? "";
  for (const privateField of ["title", "appName", "processId", "pid", "z", "globalX", "globalY"]) {
    assert.doesNotMatch(tsCandidate, new RegExp(`\\b${privateField}\\b`, "i"));
    assert.doesNotMatch(rustCandidate, new RegExp(`\\b${privateField}\\b`, "i"));
  }
  assert.match(types, /targets: ScreenshotTargetCandidate\[\]/);
  assert.match(rust, /pub targets: Vec<ScreenshotTargetCandidate>/);
});

test("scopes xcap and the custom candidate provider to macOS", () => {
  const macDependencies = cargo.match(/\[target\.'cfg\(target_os = "macos"\)'\.dependencies\]([\s\S]*?)\n\[/)?.[1] ?? "";
  assert.match(macDependencies, /^xcap = "0\.9\.8"$/m);
  assert.equal((cargo.match(/^xcap\s*=/gm) ?? []).length, 1);
  assert.match(rust, /#\[cfg\(target_os = "macos"\)\]\s*mod capture_targets;/);
  assert.match(rust, /#\[cfg\(not\(target_os = "macos"\)\)\]/);
  assert.match(rust, /#\[cfg\(target_os = "windows"\)\]/);
  assert.match(rust, /#\[cfg\(not\(any\(target_os = "macos", target_os = "windows"\)\)\)\]/);
});

test("starts uncommitted and renders target preview plus perpendicular guides", () => {
  assert.match(capture, /useState<SelectionGeometry \| null>\(null\)/);
  assert.match(capture, /setSelection\(null\)/);
  assert.doesNotMatch(capture, /createFullImageSelection/);
  assert.match(capture, /kind: "pending-target"/);
  assert.match(capture, /resolveStableTargetClick/);
  assert.match(capture, /hasExceededTargetDragThreshold/);
  assert.match(capture, /onPointerCancel=\{rollbackSelectionPointerInteraction\}/);
  assert.match(capture, /onLostPointerCapture=\{rollbackSelectionPointerInteraction\}/);
  assert.match(capture, /capture-target-preview/);
  assert.match(capture, /capture-guide-horizontal/);
  assert.match(capture, /capture-guide-vertical/);
  assert.match(styles, /\.capture-target-preview[\s\S]*?pointer-events: none/);
  assert.match(styles, /\.capture-guide-horizontal/);
  assert.match(styles, /\.capture-guide-vertical/);
});

test("provides accessible isolated dimension and radius controls", () => {
  assert.match(controls, /role="group"/);
  assert.match(controls, /aria-label=\{labels\.width\}/);
  assert.match(controls, /aria-label=\{labels\.height\}/);
  assert.match(controls, /type="range"/);
  assert.match(controls, /aria-label=\{labels\.radius\}/);
  assert.match(controls, /event\.nativeEvent\.isComposing/);
  assert.match(controls, /event\.stopPropagation\(\)/);
  assert.match(controls, /event\.key === "Escape"/);
  assert.match(controls, /event\.key === "Enter" \|\| event\.key === "Tab"/);
  assert.match(controls, /onBlur=\{\(\) => \{[\s\S]*?commitDraft\("width"\)/);
  assert.match(controls, /onBlur=\{\(\) => \{[\s\S]*?commitDraft\("height"\)/);
  assert.match(styles, /\.capture-geometry-field input\[aria-invalid="true"\]/);
});

test("uses rounded SVG and Canvas geometry without adding unrelated effects", () => {
  assert.match(capture, /fillRule="evenodd"/);
  assert.match(capture, /geometry\.cornerRadius/);
  assert.match(exportSource, /ctx\.drawImage\([\s\S]*?applyRoundedCanvasMask\(ctx, target\.width, target\.height, cornerRadius\)/);
  assert.match(exportSource, /drawAnnotations\(ctx, annotations\.filter/);
  assert.doesNotMatch(exportSource, /capture-(?:target|guide|geometry|selection|toolbar)/);
  const selectionFrameRule = styles.match(/\.capture-selection-frame \{[^}]*\}/)?.[0] ?? "";
  assert.doesNotMatch(selectionFrameRule, /box-shadow/);
  for (const excluded of ["dropShadow", "ocr", "aspectRatio", "aspect-ratio"]) {
    assert.doesNotMatch(`${capture}\n${controls}`, new RegExp(excluded, "i"));
  }
});

test("localizes target guidance and geometry labels in both languages", () => {
  for (const key of [
    "screenshot.target.preview",
    "screenshot.target.guides",
    "screenshot.target.hint",
    "screenshot.geometry.width",
    "screenshot.geometry.height",
    "screenshot.geometry.radius",
  ]) {
    assert.equal((messages.match(new RegExp(`"${key.replaceAll(".", "\\.")}"`, "g")) ?? []).length, 2);
  }
});
