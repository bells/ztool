import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUTPUT_ROOT = path.join(ROOT, "public", "file-engine");
const INSTALLED_BUDGET = 45 * 1024 * 1024;
const COMPRESSED_BUDGET = 20 * 1024 * 1024;

const ASSETS = [
  ["node_modules/pdfjs-dist/build/pdf.worker.min.mjs", "pdf.worker-core.min.mjs"],
  ["src/plugins/file/engine/pdf.worker.bootstrap.mjs", "pdf.worker.min.mjs"],
  ["node_modules/pdfjs-dist/cmaps", "cmaps"],
  ["node_modules/pdfjs-dist/standard_fonts", "standard_fonts"],
  ["node_modules/pdfjs-dist/wasm", "wasm"],
  ["node_modules/pdfjs-dist/LICENSE", "licenses/pdfjs-dist-LICENSE"],
  ["node_modules/docx/LICENSE", "licenses/docx-LICENSE"],
  ["node_modules/docx-preview/LICENSE", "licenses/docx-preview-LICENSE"],
  ["src/plugins/file/engine/THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"],
];

export function prepareFileEngine() {
  fs.rmSync(OUTPUT_ROOT, { recursive: true, force: true });
  fs.mkdirSync(OUTPUT_ROOT, { recursive: true });

  for (const [source, destination] of ASSETS) {
    copyRequired(path.join(ROOT, source), path.join(OUTPUT_ROOT, destination));
  }

  const files = listFiles(OUTPUT_ROOT).map((absolutePath) => {
    const bytes = fs.readFileSync(absolutePath);
    return {
      path: path.relative(OUTPUT_ROOT, absolutePath).split(path.sep).join("/"),
      bytes: bytes.length,
      gzipBytes: zlib.gzipSync(bytes, { level: 9 }).length,
      sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
    };
  });
  const installedBytes = files.reduce((sum, file) => sum + file.bytes, 0);
  const compressedBytes = files.reduce((sum, file) => sum + file.gzipBytes, 0);
  const manifest = {
    schemaVersion: 1,
    engineVersion: "1.0.0",
    protocolVersion: 1,
    components: {
      pdfjsDist: "6.2.108",
      docx: "9.7.1",
      docxPreview: "0.4.0",
    },
    assets: files,
    measurements: { installedBytes, compressedBytes },
  };

  if (installedBytes > INSTALLED_BUDGET) {
    throw new Error(`File engine assets exceed the 45 MiB installed budget: ${installedBytes}`);
  }
  if (compressedBytes > COMPRESSED_BUDGET) {
    throw new Error(`File engine assets exceed the 20 MiB compressed budget: ${compressedBytes}`);
  }

  fs.writeFileSync(
    path.join(OUTPUT_ROOT, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  return manifest;
}

function copyRequired(source, destination) {
  if (!fs.existsSync(source)) {
    throw new Error(`Required File engine asset is missing: ${path.relative(ROOT, source)}`);
  }
  const stat = fs.lstatSync(source);
  if (stat.isSymbolicLink()) {
    throw new Error(`File engine assets may not be symlinks: ${path.relative(ROOT, source)}`);
  }
  if (stat.isDirectory()) {
    fs.mkdirSync(destination, { recursive: true });
    for (const entry of fs.readdirSync(source)) {
      copyRequired(path.join(source, entry), path.join(destination, entry));
    }
    return;
  }
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.copyFileSync(source, destination);
}

function listFiles(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) return listFiles(target);
    if (!entry.isFile()) throw new Error(`Unsupported engine asset type: ${target}`);
    return [target];
  }).sort();
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const manifest = prepareFileEngine();
  console.log(
    `Prepared Zero File engine assets: ${manifest.measurements.installedBytes} bytes installed, ${manifest.measurements.compressedBytes} bytes gzip`,
  );
}
