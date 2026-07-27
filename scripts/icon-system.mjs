#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
export const PROJECT_ROOT = path.resolve(SCRIPT_DIR, "..");
export const ICON_DIR = path.join(PROJECT_ROOT, "src", "assets", "icons");

export const STATUS_ICON_SPECS = Object.freeze([
  { id: "zero", file: "zero.svg", label: "Zero" },
  { id: "launch", file: "zero-launch.svg", label: "Zero Launch" },
  { id: "screenshot", file: "zero-snap.svg", label: "Zero Snap" },
  { id: "caffeine-empty", file: "zero-awake.svg", label: "Zero Awake" },
  {
    id: "caffeine-full",
    file: "zero-awake-active.svg",
    label: "Zero Awake Active",
  },
  { id: "paper", file: "zero-paper.svg", label: "Zero Paper" },
]);

export const TRAY_ICON_SPECS = Object.freeze([
  ...STATUS_ICON_SPECS,
  { id: "extension", file: "extension.svg", label: "Extension" },
]);

export const CANONICAL_IDENTITY_FILES = Object.freeze([
  "zero.svg",
  "zero-launch.svg",
  "zero-snap.svg",
  "zero-awake.svg",
  "zero-paper.svg",
]);

const APP_ICON_FILE = "zero-app-icon.svg";
export const TRAY_OUTPUT_FILES = Object.freeze({
  zero: "zero.png",
  launch: "zero-launch.png",
  screenshot: "zero-snap.png",
  "caffeine-empty": "zero-awake.png",
  "caffeine-full": "zero-awake-active.png",
  paper: "zero-paper.png",
  extension: "extension.png",
});
const ALLOWED_DRAWING_COLORS = new Set(["none", "currentColor"]);
const PROHIBITED_ELEMENTS = /<(?:text|image|filter|foreignObject|script|style)\b/i;
const PROHIBITED_REFERENCES = /\b(?:href|xlink:href)\s*=|\burl\s*\(|\bdata:/i;

export function readAttribute(source, attribute) {
  const root = source.match(/<svg\b([^>]*)>/i);
  if (!root) {
    return undefined;
  }
  return root[1].match(new RegExp(`\\b${attribute}="([^"]*)"`))?.[1];
}

export function validateXmlStructure(source) {
  const issues = [];
  const stack = [];
  const tokens = source.match(/<[^>]+>/g) ?? [];

  for (const token of tokens) {
    if (/^<\?/.test(token) || /^<!/.test(token)) {
      continue;
    }
    const closing = token.match(/^<\/([A-Za-z][\w:.-]*)\s*>$/);
    if (closing) {
      const expected = stack.pop();
      if (expected !== closing[1]) {
        issues.push(`mismatched closing tag: expected ${expected ?? "none"}, received ${closing[1]}`);
      }
      continue;
    }
    const opening = token.match(/^<([A-Za-z][\w:.-]*)\b/);
    if (!opening) {
      issues.push(`invalid XML tag: ${token}`);
      continue;
    }
    if (!/\/>$/.test(token)) {
      stack.push(opening[1]);
    }
  }

  if (stack.length > 0) {
    issues.push(`unclosed XML tag: ${stack.at(-1)}`);
  }
  return issues;
}

export function validateStatusSvg(source, file) {
  const issues = validateXmlStructure(source).map((issue) => `${file}: ${issue}`);
  const requiredRootAttributes = {
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    "stroke-width": "2",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
  };

  for (const [attribute, expected] of Object.entries(requiredRootAttributes)) {
    const actual = readAttribute(source, attribute);
    if (actual !== expected) {
      issues.push(`${file}: expected ${attribute}="${expected}", received ${actual ?? "missing"}`);
    }
  }

  if (PROHIBITED_ELEMENTS.test(source)) {
    issues.push(`${file}: contains a prohibited SVG element`);
  }
  if (PROHIBITED_REFERENCES.test(source)) {
    issues.push(`${file}: contains an external or embedded resource reference`);
  }
  if (/\bfont(?:-family|-face)?\b/i.test(source)) {
    issues.push(`${file}: contains a font reference`);
  }

  for (const match of source.matchAll(/\b(?:fill|stroke)="([^"]+)"/g)) {
    if (!ALLOWED_DRAWING_COLORS.has(match[1])) {
      issues.push(`${file}: uses unsupported drawing color ${match[1]}`);
    }
  }

  return issues;
}

export function validateIconSystem(root = PROJECT_ROOT) {
  const iconDir = path.join(root, "src", "assets", "icons");
  const issues = [];

  for (const file of [...CANONICAL_IDENTITY_FILES, "zero-awake-active.svg"]) {
    const filePath = path.join(iconDir, file);
    if (!fs.existsSync(filePath)) {
      issues.push(`${file}: missing canonical status SVG`);
      continue;
    }
    issues.push(...validateStatusSvg(fs.readFileSync(filePath, "utf8"), file));
  }

  const extension = readRequiredFile(iconDir, "extension.svg", issues);
  if (extension) {
    issues.push(...validateStatusSvg(extension, "extension.svg"));
  }

  const zero = readRequiredFile(iconDir, "zero.svg", issues);
  if (zero && !zero.includes('<path d="M5.5 18.5 18.5 5.5"/>')) {
    issues.push("zero.svg: missing the canonical 45 degree slash");
  }

  const awake = readRequiredFile(iconDir, "zero-awake.svg", issues);
  const awakeActive = readRequiredFile(iconDir, "zero-awake-active.svg", issues);
  if (awake && awakeActive) {
    const withoutStateMark = awakeActive.replace(/\s*<path d="M6\.5 14h7"\/>/, "");
    if (normalizeSvg(withoutStateMark) !== normalizeSvg(awake)) {
      issues.push("zero-awake-active.svg: base geometry differs from zero-awake.svg");
    }
  }

  const appIcon = readRequiredFile(iconDir, APP_ICON_FILE, issues);
  if (appIcon) {
    issues.push(...validateXmlStructure(appIcon).map((issue) => `${APP_ICON_FILE}: ${issue}`));
    if (readAttribute(appIcon, "viewBox") !== "0 0 512 512") {
      issues.push(`${APP_ICON_FILE}: expected viewBox="0 0 512 512"`);
    }
    for (const expected of [
      '<rect x="32" y="32" width="448" height="448" rx="104" fill="#111318"/>',
      '<circle cx="256" cy="256" r="144"/>',
      '<path d="M142 370 370 142"/>',
    ]) {
      if (!appIcon.includes(expected)) {
        issues.push(`${APP_ICON_FILE}: missing canonical geometry ${expected}`);
      }
    }
  }

  issues.push(...validateGeneratedAssets(root));
  return issues;
}

export function readPngMetadata(bytes) {
  const signature = "89504e470d0a1a0a";
  if (bytes.length < 26 || bytes.subarray(0, 8).toString("hex") !== signature) {
    return undefined;
  }
  if (bytes.subarray(12, 16).toString("ascii") !== "IHDR") {
    return undefined;
  }
  const colorType = bytes[25];
  return {
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
    bitDepth: bytes[24],
    colorType,
    hasAlpha: colorType === 4 || colorType === 6,
  };
}

export function validateGeneratedAssets(root = PROJECT_ROOT) {
  const issues = [];
  const trayDir = path.join(root, "src-tauri", "icons", "tray");
  for (const [id, file] of Object.entries(TRAY_OUTPUT_FILES)) {
    const filePath = path.join(trayDir, file);
    const metadata = readPngFile(filePath, issues, `tray ${id}`);
    if (
      metadata &&
      (metadata.width !== 18 ||
        metadata.height !== 18 ||
        metadata.bitDepth !== 8 ||
        !metadata.hasAlpha)
    ) {
      issues.push(
        `${file}: expected an 18x18 8-bit PNG with alpha, received ${JSON.stringify(metadata)}`,
      );
    }
  }

  const tauriRoot = path.join(root, "src-tauri");
  const configPath = path.join(tauriRoot, "tauri.conf.json");
  if (!fs.existsSync(configPath)) {
    issues.push("src-tauri/tauri.conf.json: missing Tauri configuration");
    return issues;
  }

  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
  for (const relativeFile of config.bundle?.icon ?? []) {
    const filePath = path.join(tauriRoot, relativeFile);
    if (!fs.existsSync(filePath)) {
      issues.push(`${relativeFile}: Tauri bundle icon is missing`);
      continue;
    }
    if (relativeFile.endsWith(".png")) {
      readPngFile(filePath, issues, relativeFile);
    } else if (relativeFile.endsWith(".icns")) {
      const bytes = fs.readFileSync(filePath);
      if (bytes.subarray(0, 4).toString("ascii") !== "icns") {
        issues.push(`${relativeFile}: invalid ICNS signature`);
      }
    } else if (relativeFile.endsWith(".ico")) {
      const bytes = fs.readFileSync(filePath);
      if (bytes.length < 4 || bytes.readUInt32LE(0) !== 0x00010000) {
        issues.push(`${relativeFile}: invalid ICO signature`);
      }
    }
  }

  const appIcon = readPngFile(
    path.join(tauriRoot, "icons", "app-icon.png"),
    issues,
    "app-icon.png",
  );
  if (
    appIcon &&
    (appIcon.width !== 512 || appIcon.height !== 512 || !appIcon.hasAlpha)
  ) {
    issues.push(
      `app-icon.png: expected a 512x512 PNG with alpha, received ${JSON.stringify(appIcon)}`,
    );
  }

  return issues;
}

export function normalizeSvg(source) {
  return source.replace(/>\s+</g, "><").trim();
}

export function buildContactSheetSvg(root = PROJECT_ROOT) {
  const iconDir = path.join(root, "src", "assets", "icons");
  const sizes = [16, 18, 22, 24];
  const columns = STATUS_ICON_SPECS.length;
  const cellWidth = 132;
  const rowHeight = 92;
  const margin = 32;
  const headerHeight = 76;
  const footerHeight = 214;
  const width = margin * 2 + columns * cellWidth;
  const height = headerHeight + sizes.length * rowHeight * 2 + footerHeight;
  const iconSources = STATUS_ICON_SPECS.map((spec) => ({
    ...spec,
    source: fs.readFileSync(path.join(iconDir, spec.file), "utf8"),
  }));
  const appSource = fs.readFileSync(path.join(iconDir, APP_ICON_FILE), "utf8");

  const rows = [];
  for (const [themeIndex, theme] of [
    { name: "Light", background: "#F4F4F1", foreground: "#111318", secondary: "#5D6168" },
    { name: "Dark", background: "#111318", foreground: "#F5F5F2", secondary: "#A7ABB2" },
  ].entries()) {
    const themeTop = headerHeight + themeIndex * sizes.length * rowHeight;
    rows.push(
      `<rect x="0" y="${themeTop}" width="${width}" height="${sizes.length * rowHeight}" fill="${theme.background}"/>`,
      `<text x="16" y="${themeTop + 24}" fill="${theme.secondary}" font-size="13" font-family="-apple-system, BlinkMacSystemFont, sans-serif">${theme.name}</text>`,
    );
    for (const [sizeIndex, size] of sizes.entries()) {
      const rowTop = themeTop + sizeIndex * rowHeight;
      rows.push(
        `<text x="16" y="${rowTop + 58}" fill="${theme.secondary}" font-size="12" font-family="ui-monospace, SFMono-Regular, monospace">${size}px</text>`,
      );
      for (const [iconIndex, icon] of iconSources.entries()) {
        const centerX = margin + iconIndex * cellWidth + cellWidth / 2;
        const iconX = centerX - size / 2;
        const iconY = rowTop + 32;
        rows.push(
          nestSvg(icon.source, iconX, iconY, size, size, theme.foreground),
        );
      }
    }
  }

  const appTop = headerHeight + sizes.length * rowHeight * 2 + 34;
  const app128X = margin + 124;
  const app512PreviewSize = 176;
  const app512X = margin + 380;
  rows.push(
    `<rect x="0" y="${headerHeight + sizes.length * rowHeight * 2}" width="${width}" height="${footerHeight}" fill="#E7E7E3"/>`,
    `<text x="16" y="${appTop - 10}" fill="#5D6168" font-size="13" font-family="-apple-system, BlinkMacSystemFont, sans-serif">Application master previews</text>`,
    nestSvg(appSource, app128X, appTop, 128, 128),
    `<text x="${app128X + 48}" y="${appTop + 154}" fill="#5D6168" font-size="12" font-family="ui-monospace, SFMono-Regular, monospace">128px</text>`,
    nestSvg(appSource, app512X, appTop - 24, app512PreviewSize, app512PreviewSize),
    `<text x="${app512X + 66}" y="${appTop + 174}" fill="#5D6168" font-size="12" font-family="ui-monospace, SFMono-Regular, monospace">512px master, scaled for sheet</text>`,
  );

  const labels = iconSources.map((icon, index) => {
    const centerX = margin + index * cellWidth + cellWidth / 2;
    return `<text x="${centerX}" y="48" text-anchor="middle" fill="#3E4248" font-size="11" font-family="-apple-system, BlinkMacSystemFont, sans-serif">${escapeXml(icon.label)}</text>`;
  });

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">`,
    `<rect width="${width}" height="${height}" fill="#FFFFFF"/>`,
    `<text x="16" y="24" fill="#111318" font-size="16" font-weight="600" font-family="-apple-system, BlinkMacSystemFont, sans-serif">Zero Icon System</text>`,
    ...labels,
    ...rows,
    "</svg>",
    "",
  ].join("\n");
}

function nestSvg(source, x, y, width, height, color) {
  return source.replace(
    /<svg\b/,
    `<svg x="${x}" y="${y}" width="${width}" height="${height}"${color ? ` color="${color}"` : ""}`,
  );
}

function readRequiredFile(iconDir, file, issues) {
  const filePath = path.join(iconDir, file);
  if (!fs.existsSync(filePath)) {
    issues.push(`${file}: missing required file`);
    return undefined;
  }
  return fs.readFileSync(filePath, "utf8");
}

function readPngFile(filePath, issues, label) {
  if (!fs.existsSync(filePath)) {
    issues.push(`${label}: PNG file is missing`);
    return undefined;
  }
  const metadata = readPngMetadata(fs.readFileSync(filePath));
  if (!metadata) {
    issues.push(`${label}: invalid PNG structure`);
  }
  return metadata;
}

function escapeXml(value) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

function writeContactSheet(outputPath) {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, buildContactSheetSvg(), "utf8");
  console.log(`Wrote ${path.relative(PROJECT_ROOT, outputPath)}`);
}

function generateTrayIcons() {
  const outputDir = path.join(PROJECT_ROOT, "src-tauri", "icons", "tray");
  const temporaryRoot = fs.mkdtempSync(path.join(os.tmpdir(), "zero-tray-icons-"));
  fs.mkdirSync(outputDir, { recursive: true });

  try {
    for (const spec of TRAY_ICON_SPECS) {
      const temporaryOutput = path.join(temporaryRoot, spec.id);
      fs.mkdirSync(temporaryOutput, { recursive: true });
      runTauriIcon(path.join(ICON_DIR, spec.file), temporaryOutput, ["18"]);
      const generated = path.join(temporaryOutput, "18x18.png");
      const destination = path.join(outputDir, TRAY_OUTPUT_FILES[spec.id]);
      fs.copyFileSync(generated, destination);
      console.log(
        `Wrote ${path.relative(PROJECT_ROOT, destination)} from ${spec.file}`,
      );
    }
  } finally {
    fs.rmSync(temporaryRoot, { recursive: true, force: true });
  }
}

function generateApplicationIcons() {
  const outputDir = path.join(PROJECT_ROOT, "src-tauri", "icons");
  runTauriIcon(path.join(ICON_DIR, APP_ICON_FILE), outputDir);
  const generatedMaster = path.join(outputDir, "icon.png");
  const reviewMaster = path.join(outputDir, "app-icon.png");
  fs.copyFileSync(generatedMaster, reviewMaster);
  console.log(`Wrote ${path.relative(PROJECT_ROOT, reviewMaster)}`);
}

function runTauriIcon(source, outputDir, pngSizes = []) {
  const args = [
    "exec",
    "tauri",
    "icon",
    source,
    "--output",
    outputDir,
  ];
  for (const size of pngSizes) {
    args.push("--png", size);
  }

  const result = spawnSync("pnpm", args, {
    cwd: PROJECT_ROOT,
    stdio: "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Tauri icon generation failed with exit code ${result.status}`);
  }
}

function runValidation() {
  const issues = validateIconSystem();
  if (issues.length > 0) {
    console.error("Zero icon validation failed:");
    for (const issue of issues) {
      console.error(`- ${issue}`);
    }
    process.exitCode = 1;
    return;
  }
  console.log("Zero icon validation passed.");
}

const invokedDirectly = process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  const command = process.argv[2] ?? "validate";
  if (command === "validate") {
    runValidation();
  } else if (command === "contact-sheet") {
    writeContactSheet(
      path.resolve(
        PROJECT_ROOT,
        process.argv[3] ?? "docs/assets/zero-icon-contact-sheet.svg",
      ),
    );
  } else if (command === "generate-tray") {
    generateTrayIcons();
  } else if (command === "generate-app") {
    generateApplicationIcons();
  } else {
    console.error(`Unknown command: ${command}`);
    process.exitCode = 2;
  }
}
