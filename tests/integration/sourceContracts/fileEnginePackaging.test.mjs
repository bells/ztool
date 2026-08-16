import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  FILE_ENGINE_POLICY,
  auditTextForEngineDownloads,
  verifyFileEnginePackaging,
} from "../../../scripts/verify-file-engine-packaging.mjs";

const ROOT = process.cwd();

test("shipping configuration contains no bundled or downloaded file-conversion engine", () => {
  assert.deepEqual(verifyFileEnginePackaging(ROOT), []);

  const policy = JSON.parse(
    fs.readFileSync(path.join(ROOT, FILE_ENGINE_POLICY), "utf8"),
  );
  assert.deepEqual(policy.approvedBundledEngines, []);
  assert.deepEqual(policy.approvedRuntimeDownloads, []);
  assert.deepEqual(
    policy.detectableUserInstalledProviders.map((provider) => provider.id),
    ["libreOffice", "microsoftWordMacos", "microsoftWordWindows"],
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

function sourceFiles(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return /\.(?:rs|ts|tsx)$/.test(entry.name) ? [target] : [];
  });
}
