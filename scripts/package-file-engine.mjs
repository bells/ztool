import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import JSZip from "jszip";
import { build } from "vite";

import { prepareFileEngine } from "./prepare-file-engine.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const BUILD_ROOT = path.join(ROOT, "build/file-engine-package");
const ENGINE_ROOT = path.join(BUILD_ROOT, "engine");
const OUTPUT = path.join(ROOT, "build/zero-file-1.0.0.zplugin");
const ENGINE_VERSION = "1.0.0";
const PINNED_PUBLIC_KEY = Buffer.from(
  "IEUnZngZj5k5vPRKkumGQ60Qs5hfQT8WAFGAD8V/ZGI=",
  "base64",
);
const PKCS8_ED25519_SEED_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
const SPKI_ED25519_PUBLIC_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

export async function packageFileEngine({ signingKeyBase64 = process.env.ZERO_FILE_ENGINE_SIGNING_KEY } = {}) {
  if (!signingKeyBase64) {
    throw new Error(
      "ZERO_FILE_ENGINE_SIGNING_KEY must contain the release Ed25519 32-byte seed in base64.",
    );
  }
  const signingSeed = Buffer.from(signingKeyBase64, "base64");
  if (signingSeed.length !== 32) {
    throw new Error("ZERO_FILE_ENGINE_SIGNING_KEY must decode to exactly 32 bytes.");
  }
  const privateKey = crypto.createPrivateKey({
    key: Buffer.concat([PKCS8_ED25519_SEED_PREFIX, signingSeed]),
    format: "der",
    type: "pkcs8",
  });
  const derivedPublicKey = crypto
    .createPublicKey(privateKey)
    .export({ format: "der", type: "spki" })
    .subarray(-32);
  if (!crypto.timingSafeEqual(derivedPublicKey, PINNED_PUBLIC_KEY)) {
    throw new Error("The supplied signing key does not match Zero's pinned File release key.");
  }

  prepareFileEngine();
  await build({ configFile: path.join(ROOT, "vite.file-engine.config.ts") });
  fs.cpSync(path.join(ROOT, "public/file-engine"), path.join(ENGINE_ROOT, "file-engine"), {
    recursive: true,
  });
  const noticesRoot = path.join(ENGINE_ROOT, "licenses");
  fs.mkdirSync(noticesRoot, { recursive: true });
  for (const [source, destination] of [
    ["public/file-engine/THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"],
    ["public/file-engine/licenses/pdfjs-dist-LICENSE", "pdfjs-dist-LICENSE"],
    ["public/file-engine/licenses/docx-LICENSE", "docx-LICENSE"],
    ["public/file-engine/licenses/docx-preview-LICENSE", "docx-preview-LICENSE"],
  ]) {
    fs.copyFileSync(path.join(ROOT, source), path.join(noticesRoot, destination));
  }

  const assets = listFiles(ENGINE_ROOT).map((absolutePath) => {
    const bytes = fs.readFileSync(absolutePath);
    const relative = path.relative(BUILD_ROOT, absolutePath).split(path.sep).join("/");
    return {
      path: relative,
      sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
      bytes: bytes.length,
      mediaType: mediaTypeFor(relative),
    };
  });
  const notices = assets
    .map((asset) => asset.path)
    .filter((assetPath) => assetPath.startsWith("engine/licenses/"));
  const firstPartyEngine = {
    protocolVersion: 1,
    packageVersion: ENGINE_VERSION,
    hostApiRange: ">=0.1.0",
    directions: ["pdfToDocx", "docxToPdf"],
    platformMinimums: [
      { platform: "macos", version: "11.0" },
      { platform: "windows", version: "10.0.17763" },
    ],
    assets,
    notices,
    signature: "",
  };
  const manifest = {
    name: "zero.file",
    version: ENGINE_VERSION,
    author: "bells",
    main: "engine/index.html",
    permissions: ["document.convert"],
    id: "zero.file",
    displayName: "Zero File",
    description: "Offline PDF and Word conversion",
    engines: { zero: ">=0.1.0", api: "1" },
    platforms: ["macos", "windows"],
    runtime: "webview",
    firstPartyEngine,
  };
  const payload = signaturePayload(manifest);
  firstPartyEngine.signature = crypto.sign(null, Buffer.from(payload), privateKey).toString("base64");
  const publicKey = crypto.createPublicKey({
    key: Buffer.concat([SPKI_ED25519_PUBLIC_PREFIX, PINNED_PUBLIC_KEY]),
    format: "der",
    type: "spki",
  });
  if (!crypto.verify(null, Buffer.from(payload), publicKey, Buffer.from(firstPartyEngine.signature, "base64"))) {
    throw new Error("The generated Zero File manifest signature could not be verified.");
  }
  fs.writeFileSync(path.join(BUILD_ROOT, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  const zip = new JSZip();
  for (const absolutePath of listFiles(BUILD_ROOT)) {
    const relative = path.relative(BUILD_ROOT, absolutePath).split(path.sep).join("/");
    zip.file(relative, fs.readFileSync(absolutePath), {
      binary: true,
      unixPermissions: 0o100644,
    });
  }
  const archive = await zip.generateAsync({
    type: "nodebuffer",
    compression: "DEFLATE",
    compressionOptions: { level: 9 },
    platform: "UNIX",
  });
  fs.mkdirSync(path.dirname(OUTPUT), { recursive: true });
  fs.writeFileSync(OUTPUT, archive);
  return {
    output: OUTPUT,
    sha256: crypto.createHash("sha256").update(archive).digest("hex"),
    compressedBytes: archive.length,
    installedBytes: assets.reduce((total, asset) => total + asset.bytes, 0),
    assetCount: assets.length,
  };
}

function signaturePayload(manifest) {
  return canonicalJson({
    schema: "zero-file-engine-signature-v1",
    pluginName: manifest.name,
    pluginVersion: manifest.version,
    protocolVersion: manifest.firstPartyEngine.protocolVersion,
    packageVersion: manifest.firstPartyEngine.packageVersion,
    hostApiRange: manifest.firstPartyEngine.hostApiRange,
    directions: manifest.firstPartyEngine.directions,
    platformMinimums: manifest.firstPartyEngine.platformMinimums,
    assets: manifest.firstPartyEngine.assets,
    notices: manifest.firstPartyEngine.notices,
  });
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function listFiles(root) {
  return fs
    .readdirSync(root, { withFileTypes: true })
    .flatMap((entry) => {
      const target = path.join(root, entry.name);
      if (entry.isDirectory()) return listFiles(target);
      if (!entry.isFile() || entry.isSymbolicLink()) {
        throw new Error(`Unsupported File engine package entry: ${target}`);
      }
      return [target];
    })
    .sort();
}

function mediaTypeFor(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  const mediaTypes = {
    ".html": "text/html",
    ".mjs": "text/javascript",
    ".js": "text/javascript",
    ".css": "text/css",
    ".json": "application/json",
    ".wasm": "application/wasm",
    ".bcmap": "application/octet-stream",
    ".pfb": "application/octet-stream",
    ".ttf": "font/ttf",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".svg": "image/svg+xml",
    ".md": "text/markdown",
  };
  return mediaTypes[extension] ?? "text/plain";
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const report = await packageFileEngine();
  console.log(JSON.stringify(report, null, 2));
}
