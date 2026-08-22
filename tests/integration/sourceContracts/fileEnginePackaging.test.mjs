import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  FILE_ENGINE_POLICY,
  auditTextForEngineDownloads,
  verifyFileEnginePackaging,
} from "../../../scripts/verify-file-engine-packaging.mjs";
import { prepareFileEngine } from "../../../scripts/prepare-file-engine.mjs";

const ROOT = process.cwd();

test("shipping policy records one unapproved signed Zero File engine candidate", () => {
  assert.deepEqual(verifyFileEnginePackaging(ROOT), []);

  const policy = JSON.parse(
    fs.readFileSync(path.join(ROOT, FILE_ENGINE_POLICY), "utf8"),
  );
  assert.deepEqual(policy.approvedEnginePackages, []);
  assert.deepEqual(policy.approvedRuntimeDownloads, []);
  assert.equal(policy.candidateEnginePackages.length, 1);
  assert.equal(policy.candidateEnginePackages[0].pluginId, "zero.file");
  assert.equal(policy.candidateEnginePackages[0].approved, false);
  assert.deepEqual(
    policy.compatibilityProviders.map((provider) => provider.id),
    ["libreOffice", "microsoftWordMacos", "microsoftWordWindows"],
  );
});

test("native DOCX export uses bounded WebKit capture and never creates a print spool job", () => {
  const source = fs.readFileSync(
    path.join(ROOT, "src-tauri/src/services/file/native_print.rs"),
    "utf8",
  );
  assert.match(source, /createPDFWithConfiguration_completionHandler/);
  assert.match(source, /PDFKit/);
  assert.doesNotMatch(source, /NSPrintSaveJob|printOperationWithPrintInfo/);
});

test("DOCX render readiness waits for images and fonts before measuring isolated export pages", () => {
  const source = fs.readFileSync(
    path.join(ROOT, "src/plugins/file/engine/docxToPdf.ts"),
    "utf8",
  );
  const css = fs.readFileSync(
    path.join(ROOT, "src/plugins/file/engine/fileEngine.css"),
    "utf8",
  );
  assert.match(source, /await renderAsync\(/);
  assert.match(source, /ignoreWidth:\s*false/);
  assert.match(source, /ignoreHeight:\s*false/);
  assert.match(source, /breakPages:\s*true/);
  assert.match(source, /ignoreLastRenderedPageBreak:\s*false/);
  assert.match(source, /await withTimeout\(waitForImages\(root, signal\)/);
  assert.match(source, /await withTimeout\(document\.fonts\.ready/);
  assert.match(source, /Embedded images timed out/);
  assert.match(source, /Document fonts timed out/);
  assert.match(source, /document\.body\.classList\.add\("zero-file-engine-export"\)/);
  assert.match(source, /measuredPageCount/);
  assert.match(source, /pageRects/);
  assert.match(css, /body\.zero-file-engine-export/);
  assert.match(css, /section\.zero-file-docx/);
  assert.match(css, /break-after:\s*page/);
});

test("engine runtime keeps cancellation, crash recovery, and panel lifetime outside document payloads", () => {
  const engineApp = fs.readFileSync(
    path.join(ROOT, "src/plugins/file/engine/FileEngineApp.tsx"),
    "utf8",
  );
  const bridge = fs.readFileSync(
    path.join(ROOT, "src-tauri/src/services/file/engine_bridge.rs"),
    "utf8",
  );
  const panelHook = fs.readFileSync(
    path.join(ROOT, "src/plugins/file/useFileConversion.ts"),
    "utf8",
  );
  assert.match(engineApp, /controllers\.get\(payload\.token\)\?\.abort\(\)/);
  assert.match(engineApp, /status:\s*cancelled \? "cancelled" : "failed"/);
  assert.match(bridge, /fail_all_sessions/);
  assert.match(bridge, /get_webview_window\(FILE_ENGINE_LABEL\)/);
  assert.match(bridge, /visible\(false\)/);
  assert.match(panelHook, /client\.listJobs\(\)/);
  assert.match(panelHook, /reconcileInitialFileConversionJobs\(snapshot, bufferedEvents\)/);
  assert.doesNotMatch(panelHook, /FILE_ENGINE_LABEL|zero-file-engine/);
});

test("release packaging refuses to proceed without the external Ed25519 signing key", async () => {
  const { packageFileEngine } = await import("../../../scripts/package-file-engine.mjs");
  await assert.rejects(
    packageFileEngine({ signingKeyBase64: "" }),
    /ZERO_FILE_ENGINE_SIGNING_KEY/,
  );
  await assert.rejects(
    packageFileEngine({ signingKeyBase64: Buffer.alloc(32, 9).toString("base64") }),
    /does not match Zero's pinned File release key/,
  );
});

test("packaging guard rejects engine installers but permits direct installed-provider execution", () => {
  assert.deepEqual(
    auditTextForEngineDownloads('Command::new(soffice_path).args(arguments)', "provider.rs"),
    [],
  );
  assert.deepEqual(
    auditTextForEngineDownloads(
      'fetch("https://example.invalid/pdf2docx-sidecar")',
      "download.ts",
    ),
    ["download.ts contains an engine download or installation path"],
  );
  assert.deepEqual(
    auditTextForEngineDownloads("pip install pdf2docx", "build.sh"),
    ["build.sh contains an engine download or installation path"],
  );
});

test("File conversion has no network or cloud fallback path", () => {
  const nativeRoot = path.join(ROOT, "src-tauri", "src", "services", "file");
  const frontendRoot = path.join(ROOT, "src", "plugins", "file");
  const sources = [nativeRoot, frontendRoot].flatMap(sourceFiles);
  for (const sourcePath of sources) {
    const source = fs.readFileSync(sourcePath, "utf8");
    assert.doesNotMatch(source, /\b(?:reqwest|ureq|fetch|XMLHttpRequest)\b/);
    assert.doesNotMatch(source, /https?:\/\//);
  }
});

test("approved browser assets are reproducible and stay within package budgets", () => {
  const manifest = prepareFileEngine();
  assert.equal(manifest.components.pdfjsDist, "6.2.108");
  assert.equal(manifest.components.docx, "9.7.1");
  assert.equal(manifest.components.docxPreview, "0.4.0");
  assert.ok(manifest.measurements.installedBytes <= 45 * 1024 * 1024);
  assert.ok(manifest.measurements.compressedBytes <= 20 * 1024 * 1024);
  assert.ok(manifest.assets.some((asset) => asset.path === "pdf.worker.min.mjs"));
  assert.ok(manifest.assets.some((asset) => asset.path.startsWith("cmaps/")));
  assert.ok(manifest.assets.some((asset) => asset.path.startsWith("standard_fonts/")));
  assert.ok(manifest.assets.some((asset) => asset.path.startsWith("wasm/")));
  assert.ok(manifest.assets.every((asset) => /^[a-f0-9]{64}$/.test(asset.sha256)));
});

test("conversion corpus pins truthful built-in quality profiles", () => {
  const corpus = JSON.parse(
    fs.readFileSync(path.join(ROOT, "tests/fixtures/fileConversion/expected.json"), "utf8"),
  );
  const cases = new Map(corpus.cases.map((fixture) => [fixture.file, fixture]));
  assert.equal(cases.get("rich-layout.docx").expectedQualityProfile, "webRenderedPdf");
  assert.equal(cases.get("large-structured.docx").expectedQualityProfile, "webRenderedPdf");
  assert.equal(cases.get("rich-layout.pdf").expectedQualityProfile, "layoutPreserving");
  assert.equal(cases.get("image-only-scan.pdf").expectedPreflight, "valid");
  assert.equal(cases.get("image-only-scan.pdf").expectedQualityProfile, "layoutPreserving");
  assert.equal(cases.get("large-structured.pdf").expectedQualityProfile, "editableReconstruction");
  assert.ok(cases.get("large-structured.docx").coverage.includes("wordPagination"));
});

function sourceFiles(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return /\.(?:rs|ts|tsx)$/.test(entry.name) ? [target] : [];
  });
}
