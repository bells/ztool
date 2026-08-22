import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const ENGINE_TOKENS = [
  "pdf2docx",
  "pymupdf",
  "opencv",
  "libreoffice",
  "soffice",
  "onlyoffice",
];
const FORBIDDEN_PACKAGE_NAMES = new Set([
  "pdf2docx",
  "pymupdf",
  "opencv-python",
  "opencv-python-headless",
  "python-shell",
]);
const FORBIDDEN_ARTIFACT_EXTENSIONS = new Set([
  ".appimage",
  ".dmg",
  ".dll",
  ".dylib",
  ".exe",
  ".gz",
  ".msi",
  ".pkg",
  ".py",
  ".pyc",
  ".pyd",
  ".pyz",
  ".so",
  ".tar",
  ".whl",
  ".zip",
]);
const SOURCE_EXTENSIONS = new Set([".js", ".mjs", ".rs", ".ts", ".tsx"]);

export const FILE_ENGINE_POLICY = "src-tauri/file-engine-policy.json";

export function verifyFileEnginePackaging(root = process.cwd()) {
  const issues = [];
  const policy = readJson(path.join(root, FILE_ENGINE_POLICY), issues);
  if (!policy) return issues;

  assertEmptyApprovalList(policy, "approvedEnginePackages", issues);
  assertEmptyApprovalList(policy, "approvedRuntimeDownloads", issues);
  inspectCandidatePolicy(root, policy, issues);
  inspectEngineBuildWorkflow(root, issues);
  inspectTauriConfigs(root, issues);
  inspectPackageManifest(root, issues);
  inspectCargoManifest(root, issues);
  inspectPackagingRoots(root, issues);
  inspectShippingSources(root, issues);
  return issues;
}

function inspectCandidatePolicy(root, policy, issues) {
  if (policy.schemaVersion !== 2) {
    issues.push(`${FILE_ENGINE_POLICY}: schemaVersion must be 2`);
  }
  const candidates = policy.candidateEnginePackages;
  if (!Array.isArray(candidates) || candidates.length !== 1) {
    issues.push(`${FILE_ENGINE_POLICY}: exactly one Zero File candidate package is required`);
    return;
  }
  const candidate = candidates[0];
  if (candidate.pluginId !== "zero.file" || candidate.version !== "1.0.0") {
    issues.push(`${FILE_ENGINE_POLICY}: the candidate must identify zero.file version 1.0.0`);
  }
  if (candidate.approved !== false || candidate.packageSha256 !== null) {
    issues.push(`${FILE_ENGINE_POLICY}: the candidate must remain unapproved until packaged gates pass`);
  }
  const expectedComponents = new Map([
    ["pdfjs-dist", ["6.2.108", "Apache-2.0"]],
    ["docx", ["9.7.1", "MIT"]],
    ["docx-preview", ["0.4.0", "Apache-2.0"]],
    ["WebKit/PDFKit", ["system", "macOS system framework"]],
  ]);
  for (const component of candidate.components ?? []) {
    const expected = expectedComponents.get(component.name);
    if (!expected || component.version !== expected[0] || component.license !== expected[1]) {
      issues.push(`${FILE_ENGINE_POLICY}: component ${component.name} is not pinned to its reviewed version/license`);
    }
    expectedComponents.delete(component.name);
  }
  if (expectedComponents.size > 0) {
    issues.push(`${FILE_ENGINE_POLICY}: component inventory is incomplete`);
  }
  const manifestPath = path.join(root, "public/file-engine/manifest.json");
  if (!fs.existsSync(manifestPath)) {
    issues.push("public/file-engine/manifest.json is missing; run file-engine:prepare");
  } else {
    const digest = crypto.createHash("sha256").update(fs.readFileSync(manifestPath)).digest("hex");
    if (candidate.assetManifestSha256 !== digest) {
      issues.push(`${FILE_ENGINE_POLICY}: candidate assetManifestSha256 does not match prepared assets`);
    }
    const prepared = readJson(manifestPath, issues);
    if (prepared) {
      if (prepared.measurements.installedBytes > candidate.measurements.installedBudgetBytes) {
        issues.push(`${FILE_ENGINE_POLICY}: prepared assets exceed the installed budget`);
      }
      if (prepared.measurements.compressedBytes > candidate.measurements.compressedBudgetBytes) {
        issues.push(`${FILE_ENGINE_POLICY}: prepared assets exceed the compressed budget`);
      }
    }
  }
  if (candidate.evidence?.packagedSmokePassed !== false) {
    issues.push(`${FILE_ENGINE_POLICY}: packaged smoke must remain false until a signed clean-profile run passes`);
  }
}

function inspectEngineBuildWorkflow(root, issues) {
  const required = [
    "scripts/prepare-file-engine.mjs",
    "scripts/package-file-engine.mjs",
    "vite.file-engine.config.ts",
    "src/plugins/file/engine/index.html",
    "src/plugins/file/engine/THIRD_PARTY_NOTICES.md",
  ];
  for (const relative of required) {
    if (!fs.existsSync(path.join(root, relative))) issues.push(`${relative} is required`);
  }
  const packageScriptPath = path.join(root, "scripts/package-file-engine.mjs");
  if (fs.existsSync(packageScriptPath)) {
    const source = fs.readFileSync(packageScriptPath, "utf8");
    if (!source.includes("ZERO_FILE_ENGINE_SIGNING_KEY")) {
      issues.push("scripts/package-file-engine.mjs must require an externally supplied signing key");
    }
    if (!source.includes("timingSafeEqual") || !source.includes("crypto.verify")) {
      issues.push("scripts/package-file-engine.mjs must verify the supplied key and generated signature");
    }
    if (/BEGIN (?:PRIVATE|OPENSSH) KEY/.test(source)) {
      issues.push("scripts/package-file-engine.mjs must not contain a private signing key");
    }
  }
  const nativePrint = path.join(root, "src-tauri/src/services/file/native_print.rs");
  if (fs.existsSync(nativePrint)) {
    const source = fs.readFileSync(nativePrint, "utf8");
    if (source.includes("NSPrintSaveJob") || source.includes("printOperationWithPrintInfo")) {
      issues.push("native_print.rs must use bounded WKWebView PDF capture, not an NSPrint spool job");
    }
    if (!source.includes("createPDFWithConfiguration_completionHandler") || !source.includes("PDFKit")) {
      issues.push("native_print.rs must retain WebKit capture and PDFKit merging");
    }
  }
}

export function auditTextForEngineDownloads(source, label = "source") {
  const issues = [];
  const lower = source.toLowerCase();
  const mentionsEngine = ENGINE_TOKENS.some((token) => lower.includes(token));
  if (!mentionsEngine) return issues;

  const downloadPatterns = [
    /fetch\s*\([^)]*(?:pdf2docx|pymupdf|opencv|libreoffice|soffice|onlyoffice)/is,
    /(?:reqwest|ureq)[\s\S]{0,240}(?:pdf2docx|pymupdf|opencv|libreoffice|soffice|onlyoffice)/i,
    /(?:curl|wget|pip|brew|winget|choco)(?:\.exe)?[\s\S]{0,160}(?:install|download)[\s\S]{0,160}(?:pdf2docx|pymupdf|opencv|libreoffice|soffice|onlyoffice)/i,
    /(?:pdf2docx|pymupdf|opencv|libreoffice|soffice|onlyoffice)[\s\S]{0,160}(?:curl|wget|pip|brew|winget|choco)(?:\.exe)?[\s\S]{0,160}(?:install|download)/i,
  ];
  if (downloadPatterns.some((pattern) => pattern.test(source))) {
    issues.push(`${label} contains an engine download or installation path`);
  }
  return issues;
}

function assertEmptyApprovalList(policy, key, issues) {
  if (!Array.isArray(policy[key])) {
    issues.push(`${FILE_ENGINE_POLICY}: ${key} must be an array`);
  } else if (policy[key].length !== 0) {
    issues.push(
      `${FILE_ENGINE_POLICY}: ${key} must stay empty until signed packaged release gates pass`,
    );
  }
}

function inspectTauriConfigs(root, issues) {
  const tauriRoot = path.join(root, "src-tauri");
  if (!fs.existsSync(tauriRoot)) return;
  for (const entry of fs.readdirSync(tauriRoot, { withFileTypes: true })) {
    if (!entry.isFile() || !/^tauri(?:\..+)?\.conf\.json$/.test(entry.name)) continue;
    const relative = path.join("src-tauri", entry.name);
    const config = readJson(path.join(root, relative), issues);
    if (!config) continue;
    const bundle = config.bundle ?? {};
    for (const [field, value] of [
      ["externalBin", bundle.externalBin],
      ["resources", bundle.resources],
    ]) {
      for (const item of stringLeaves(value)) {
        if (containsEngineToken(item)) {
          issues.push(`${relative}: bundle.${field} includes unapproved engine asset ${item}`);
        }
      }
    }
  }
}

function inspectPackageManifest(root, issues) {
  const packagePath = path.join(root, "package.json");
  const manifest = readJson(packagePath, issues);
  if (!manifest) return;
  for (const section of ["dependencies", "devDependencies", "optionalDependencies"]) {
    for (const dependency of Object.keys(manifest[section] ?? {})) {
      if (FORBIDDEN_PACKAGE_NAMES.has(dependency.toLowerCase())) {
        issues.push(`package.json: ${section} includes unapproved engine dependency ${dependency}`);
      }
    }
  }
  for (const [name, script] of Object.entries(manifest.scripts ?? {})) {
    issues.push(...auditTextForEngineDownloads(String(script), `package.json script ${name}`));
  }
}

function inspectCargoManifest(root, issues) {
  const cargoPath = path.join(root, "src-tauri", "Cargo.toml");
  if (!fs.existsSync(cargoPath)) return;
  const source = fs.readFileSync(cargoPath, "utf8");
  for (const dependency of ["pyo3", "pythonize", "rustpython-vm"]) {
    const pattern = new RegExp(`^\\s*${dependency.replace("-", "\\-")}\\s*=`, "mi");
    if (pattern.test(source)) {
      issues.push(`src-tauri/Cargo.toml includes unapproved embedded-Python dependency ${dependency}`);
    }
  }
}

function inspectPackagingRoots(root, issues) {
  const candidates = [
    { path: "resources", rejectEveryExecutable: true },
    { path: "public/engines", rejectEveryExecutable: true },
    { path: "src-tauri/binaries", rejectEveryExecutable: true },
    { path: "src-tauri/resources", rejectEveryExecutable: true },
    { path: "src-tauri/sidecars", rejectEveryExecutable: true },
    { path: "src-tauri/target/release/bundle", rejectEveryExecutable: false },
  ];
  for (const candidate of candidates) {
    const relativeRoot = candidate.path;
    const absoluteRoot = path.join(root, relativeRoot);
    if (!fs.existsSync(absoluteRoot)) continue;
    for (const file of filesRecursively(absoluteRoot)) {
      const relative = path.relative(root, file);
      const lower = relative.toLowerCase();
      const extension = path.extname(lower);
      if (containsEngineToken(lower)) {
        issues.push(`${relative}: unapproved engine-named artifact is present in a packaging root`);
      }
      if (
        candidate.rejectEveryExecutable &&
        FORBIDDEN_ARTIFACT_EXTENSIONS.has(extension)
      ) {
        issues.push(`${relative}: executable/archive artifact is not approved by the empty engine allowlist`);
      } else if (candidate.rejectEveryExecutable && looksExecutable(file)) {
        issues.push(`${relative}: executable artifact is not approved by the empty engine allowlist`);
      }
    }
  }
}

function inspectShippingSources(root, issues) {
  const roots = ["src", "src-tauri/src"];
  for (const relativeRoot of roots) {
    const absoluteRoot = path.join(root, relativeRoot);
    if (!fs.existsSync(absoluteRoot)) continue;
    for (const file of filesRecursively(absoluteRoot)) {
      if (!SOURCE_EXTENSIONS.has(path.extname(file))) continue;
      const relative = path.relative(root, file);
      const source = fs.readFileSync(file, "utf8");
      issues.push(...auditTextForEngineDownloads(source, relative));
    }
  }
  const buildScript = path.join(root, "src-tauri", "build.rs");
  if (fs.existsSync(buildScript)) {
    issues.push(
      ...auditTextForEngineDownloads(
        fs.readFileSync(buildScript, "utf8"),
        "src-tauri/build.rs",
      ),
    );
  }
}

function readJson(file, issues) {
  if (!fs.existsSync(file)) {
    issues.push(`${path.relative(process.cwd(), file)} is missing`);
    return undefined;
  }
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (error) {
    issues.push(`${path.relative(process.cwd(), file)} is invalid JSON: ${error.message}`);
    return undefined;
  }
}

function stringLeaves(value) {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value.flatMap(stringLeaves);
  if (value && typeof value === "object") return Object.values(value).flatMap(stringLeaves);
  return [];
}

function containsEngineToken(value) {
  const lower = value.toLowerCase();
  return ENGINE_TOKENS.some((token) => lower.includes(token));
}

function filesRecursively(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isSymbolicLink()) return [target];
    if (entry.isDirectory()) return filesRecursively(target);
    return entry.isFile() ? [target] : [];
  });
}

function looksExecutable(file) {
  const stat = fs.lstatSync(file);
  if (stat.isSymbolicLink()) return true;
  if ((stat.mode & 0o111) !== 0) return true;
  const descriptor = fs.openSync(file, "r");
  try {
    const header = Buffer.alloc(4);
    const length = fs.readSync(descriptor, header, 0, header.length, 0);
    if (length >= 2 && header.subarray(0, 2).equals(Buffer.from("MZ"))) return true;
    if (length >= 4 && header.equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))) return true;
    const magic = length === 4 ? header.readUInt32BE(0) : 0;
    return new Set([0xfeedface, 0xfeedfacf, 0xcafebabe, 0xcefaedfe, 0xcffaedfe]).has(magic);
  } finally {
    fs.closeSync(descriptor);
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const issues = verifyFileEnginePackaging();
  if (issues.length > 0) {
    for (const issue of issues) console.error(`- ${issue}`);
    process.exitCode = 1;
  } else {
    console.log("File engine packaging policy passed: the signed plugin candidate is reproducible and remains unapproved.");
  }
}
