import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const ROOT = process.cwd();
const DIST_ROOT = path.join(ROOT, "dist");
const MANIFEST_PATH = path.join(DIST_ROOT, ".vite", "manifest.json");
const SOURCE_ENTRY = path.join(ROOT, "src", "main.tsx");
const FILE_ENGINE_MANIFEST = path.join(ROOT, "public", "file-engine", "manifest.json");
const DEFAULT_OUTPUT = path.join(
  ROOT,
  "openspec/changes/systematic-performance-and-code-optimization/evidence/build-graph.json",
);
const BASELINE_BUDGET = {
  // Reviewed after the first window/plugin split: 267,015 / 81,805 bytes.
  // The headroom covers deterministic bundler drift without restoring the eager graph.
  initialEntryBytes: 285_000,
  initialEntryGzipBytes: 90_000,
  largestChunkBytes: 820_000,
};
const FORBIDDEN_EAGER_PATTERNS = [
  "src/plugins/screenshot/capture/CaptureApp",
  "src/plugins/screenshot/capture/PinApp",
  "src/plugins/quickLauncher/QuickLauncherApp",
  "src/plugins/bingWallpaper/PaperApp",
  "src/plugins/file/engine/FileEngineApp",
  "pdfjs-dist",
  "docx",
  "docx-preview",
];

export function analyzeBuildGraph(manifest, readAsset) {
  const entries = Object.entries(manifest)
    .filter(([, chunk]) => chunk.isEntry)
    .map(([key, chunk]) => analyzeEntry(key, chunk, manifest, readAsset));
  const chunks = Object.entries(manifest)
    .filter(([, chunk]) => typeof chunk.file === "string" && chunk.file.endsWith(".js"))
    .map(([key, chunk]) => assetRecord(key, chunk.file, readAsset))
    .sort((left, right) => right.bytes - left.bytes);
  return {
    entries,
    chunks,
    largestChunk: chunks[0] ?? null,
  };
}

export function budgetFindings(report, budget = BASELINE_BUDGET) {
  const findings = [];
  for (const entry of report.entries) {
    if (entry.staticBytes > budget.initialEntryBytes) {
      findings.push(`${entry.key} static bytes ${entry.staticBytes} > ${budget.initialEntryBytes}`);
    }
    if (entry.staticGzipBytes > budget.initialEntryGzipBytes) {
      findings.push(`${entry.key} static gzip bytes ${entry.staticGzipBytes} > ${budget.initialEntryGzipBytes}`);
    }
  }
  if (report.largestChunk && report.largestChunk.bytes > budget.largestChunkBytes) {
    findings.push(
      `${report.largestChunk.key} bytes ${report.largestChunk.bytes} > ${budget.largestChunkBytes}`,
    );
  }
  return findings;
}

export function collectSourceGraph(entryPath, readSource, resolveLocal) {
  const modules = new Set();
  const externalImports = new Set();
  const dynamicImports = new Set();
  const visit = (modulePath) => {
    if (modules.has(modulePath)) return;
    modules.add(modulePath);
    const source = readSource(modulePath);
    for (const specifier of dynamicImportSpecifiers(source)) {
      dynamicImports.add(specifier);
    }
    for (const specifier of staticImportSpecifiers(source)) {
      if (!specifier.startsWith(".")) {
        externalImports.add(specifier);
        continue;
      }
      const resolved = resolveLocal(modulePath, specifier);
      if (resolved) visit(resolved);
    }
  };
  visit(entryPath);
  return {
    modules: [...modules].sort(),
    externalImports: [...externalImports].sort(),
    dynamicImports: [...dynamicImports].sort(),
  };
}

export function forbiddenEagerFindings(sourceGraph, patterns = FORBIDDEN_EAGER_PATTERNS) {
  const candidates = [...sourceGraph.modules, ...sourceGraph.externalImports];
  return patterns.filter((pattern) =>
    candidates.some((candidate) => candidate.includes(pattern)),
  );
}

function analyzeEntry(key, chunk, manifest, readAsset) {
  const staticKeys = collectStaticImports(key, manifest);
  const dynamicKeys = [
    ...new Set(
      staticKeys.flatMap((itemKey) => manifest[itemKey]?.dynamicImports ?? []),
    ),
  ];
  const staticAssets = staticKeys.map((itemKey) => {
    const item = manifest[itemKey];
    return assetRecord(itemKey, item.file, readAsset);
  });
  return {
    key,
    file: chunk.file,
    staticAssets,
    dynamicImports: dynamicKeys,
    staticBytes: staticAssets.reduce((sum, asset) => sum + asset.bytes, 0),
    staticGzipBytes: staticAssets.reduce((sum, asset) => sum + asset.gzipBytes, 0),
  };
}

function collectStaticImports(rootKey, manifest) {
  const visited = new Set();
  const visit = (key) => {
    if (visited.has(key) || !manifest[key]) return;
    visited.add(key);
    for (const imported of manifest[key].imports ?? []) visit(imported);
  };
  visit(rootKey);
  return [...visited];
}

function assetRecord(key, file, readAsset) {
  const bytes = readAsset(file);
  return {
    key,
    file,
    bytes: bytes.length,
    gzipBytes: zlib.gzipSync(bytes).length,
  };
}

function staticImportSpecifiers(source) {
  return [
    ...source.matchAll(
      /(?:import|export)\s+(?:type\s+)?(?:[^"']*?\s+from\s*)?["']([^"']+)["']/g,
    ),
  ].map((match) => match[1]);
}

function dynamicImportSpecifiers(source) {
  return [...source.matchAll(/import\(\s*["']([^"']+)["']\s*\)/g)].map(
    (match) => match[1],
  );
}

function resolveSourceImport(importer, specifier) {
  const base = path.resolve(path.dirname(importer), specifier);
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    `${base}.js`,
    `${base}.jsx`,
    path.join(base, "index.ts"),
    path.join(base, "index.tsx"),
  ];
  return candidates.find((candidate) => fs.existsSync(candidate)) ?? null;
}

function main() {
  if (!fs.existsSync(MANIFEST_PATH)) {
    throw new Error(`Missing ${path.relative(ROOT, MANIFEST_PATH)}; run pnpm build first`);
  }
  const manifest = JSON.parse(fs.readFileSync(MANIFEST_PATH, "utf8"));
  const report = analyzeBuildGraph(manifest, (file) =>
    fs.readFileSync(path.join(DIST_ROOT, file)),
  );
  const sourceGraph = collectSourceGraph(
    SOURCE_ENTRY,
    (file) => fs.readFileSync(file, "utf8"),
    resolveSourceImport,
  );
  const forbiddenEagerImports = forbiddenEagerFindings({
    ...sourceGraph,
    modules: sourceGraph.modules.map((file) => path.relative(ROOT, file).split(path.sep).join("/")),
  });
  const findings = [
    ...budgetFindings(report),
    ...forbiddenEagerImports.map((pattern) => `initial source graph eagerly reaches ${pattern}`),
  ];
  const fileEngineAssets = fs.existsSync(FILE_ENGINE_MANIFEST)
    ? JSON.parse(fs.readFileSync(FILE_ENGINE_MANIFEST, "utf8")).measurements
    : null;
  const result = {
    schemaVersion: 1,
    recordedAt: new Date().toISOString(),
    budget: BASELINE_BUDGET,
    findings,
    forbiddenEagerImports,
    fileEngineAssets,
    sourceGraph: {
      modules: sourceGraph.modules.map((file) => path.relative(ROOT, file).split(path.sep).join("/")),
      externalImports: sourceGraph.externalImports,
      dynamicImports: sourceGraph.dynamicImports,
    },
    ...report,
  };
  const outputIndex = process.argv.indexOf("--output");
  const output = outputIndex >= 0 ? process.argv[outputIndex + 1] : DEFAULT_OUTPUT;
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, `${JSON.stringify(result, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  if (process.argv.includes("--check") && findings.length > 0) process.exitCode = 1;
}

if (import.meta.url === `file://${process.argv[1]}`) main();
