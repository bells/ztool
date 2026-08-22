import assert from "node:assert/strict";
import test from "node:test";
import {
  SUPPORTED_PLUGIN_PERMISSIONS,
  validatePluginManifest,
  validatePluginMarketIndex,
} from "/private/tmp/zero-tests/core/pluginHost/validation.js";

const validManifest = {
  name: "clipboard-helper",
  version: "0.1.0",
  author: "watson",
  main: "dist/index.html",
  permissions: ["clipboard.read", "network"],
};

const validMarket = {
  schemaVersion: 1,
  updatedAt: "2026-06-21T00:00:00Z",
  plugins: [
    {
      name: "clipboard-helper",
      version: "0.1.0",
      author: "watson",
      description: "Clipboard helper plugin",
      repository: "https://github.com/watson/clipboard-helper",
      releaseUrl: "https://github.com/watson/clipboard-helper/releases/tag/v0.1.0",
      downloadUrl:
        "https://github.com/watson/clipboard-helper/releases/download/v0.1.0/clipboard-helper.zplugin",
      sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      permissions: ["clipboard.read"],
    },
  ],
};

function assertIssue(report, code, path) {
  assert.equal(report.valid, false);
  assert.ok(
    report.issues.some((issue) => issue.code === code && issue.path === path),
    `expected issue ${code} at ${path}, got ${JSON.stringify(report.issues)}`,
  );
}

test("accepts a valid MVP plugin manifest", () => {
  const report = validatePluginManifest(validManifest);

  assert.equal(report.valid, true);
  assert.deepEqual(report.manifest, validManifest);
  assert.deepEqual(SUPPORTED_PLUGIN_PERMISSIONS, [
    "clipboard.read",
    "clipboard.write",
    "network",
    "storage.plugin",
    "ui.message",
    "process.execute",
    "system.wallpaper",
    "system.apps.read",
    "system.apps.execute",
    "system.window.focus",
    "system.settings.open",
    "document.convert",
  ]);
});

test("document.convert remains reserved for signed Zero File metadata", () => {
  const report = validatePluginManifest({
    ...validManifest,
    name: "impostor",
    id: "zero.file",
    permissions: ["document.convert"],
  });
  assertIssue(report, "manifest.documentConvert.firstPartyOnly", "permissions");
});

test("accepts the system wallpaper permission and rejects near matches", () => {
  const accepted = validatePluginManifest({
    ...validManifest,
    permissions: ["network", "storage.plugin", "system.wallpaper"],
  });
  const rejected = validatePluginManifest({
    ...validManifest,
    permissions: ["system.wallpapers"],
  });

  assert.equal(accepted.valid, true);
  assertIssue(rejected, "manifest.permission.unsupported", "permissions[0]");
});

test("accepts every launcher permission and rejects near matches", () => {
  const permissions = [
    "system.apps.read",
    "system.apps.execute",
    "system.window.focus",
    "system.settings.open",
  ];
  assert.equal(validatePluginManifest({ ...validManifest, permissions }).valid, true);
  assertIssue(
    validatePluginManifest({ ...validManifest, permissions: ["system.app.read"] }),
    "manifest.permission.unsupported",
    "permissions[0]",
  );
});

test("rejects manifest missing required fields", () => {
  const report = validatePluginManifest({
    ...validManifest,
    main: undefined,
  });

  assertIssue(report, "manifest.required", "main");
});

test("rejects unsafe manifest main paths", () => {
  assertIssue(
    validatePluginManifest({
      ...validManifest,
      main: "../dist/index.html",
    }),
    "manifest.main.unsafe",
    "main",
  );

  assertIssue(
    validatePluginManifest({
      ...validManifest,
      main: "/Applications/Calculator.app",
    }),
    "manifest.main.unsafe",
    "main",
  );
});

test("rejects unsupported manifest permissions", () => {
  const report = validatePluginManifest({
    ...validManifest,
    permissions: ["clipboard.read", "filesystem.all"],
  });

  assertIssue(report, "manifest.permission.unsupported", "permissions[1]");
});

test("rejects incompatible manifest host and API versions", () => {
  const report = validatePluginManifest({
    ...validManifest,
    engines: {
      api: "2",
      zero: "999.0.0",
    },
  });

  assertIssue(report, "manifest.api.incompatible", "engines.api");
  assertIssue(report, "manifest.zero.incompatible", "engines.zero");
});

test("accepts and canonicalizes the legacy host key", () => {
  const report = validatePluginManifest({
    ...validManifest,
    engines: {
      api: "1",
      ztool: "0.1.0",
    },
  });

  assert.equal(report.valid, true);
  assert.deepEqual(report.manifest.engines, {
    api: "1",
    zero: "0.1.0",
  });
});

test("canonical host key is authoritative when both host keys exist", () => {
  const accepted = validatePluginManifest({
    ...validManifest,
    engines: {
      zero: "0.1.0",
      ztool: "999.0.0",
    },
  });
  const rejected = validatePluginManifest({
    ...validManifest,
    engines: {
      zero: "999.0.0",
      ztool: "0.1.0",
    },
  });

  assert.equal(accepted.valid, true);
  assert.deepEqual(accepted.manifest.engines, { zero: "0.1.0" });
  assertIssue(rejected, "manifest.zero.incompatible", "engines.zero");
});

test("accepts a valid Git-based market index", () => {
  const report = validatePluginMarketIndex(validMarket);

  assert.equal(report.valid, true);
  assert.deepEqual(report.market, validMarket);
});

test("rejects invalid market entries", () => {
  assertIssue(
    validatePluginMarketIndex({
      ...validMarket,
      plugins: [
        {
          ...validMarket.plugins[0],
          downloadUrl: undefined,
        },
      ],
    }),
    "market.downloadUrl.required",
    "plugins[0].downloadUrl",
  );

  assertIssue(
    validatePluginMarketIndex({
      ...validMarket,
      plugins: [
        {
          ...validMarket.plugins[0],
          downloadUrl:
            "https://github.com/watson/clipboard-helper/releases/download/v0.1.0/clipboard-helper.zip",
        },
      ],
    }),
    "market.downloadUrl.extension",
    "plugins[0].downloadUrl",
  );
});
