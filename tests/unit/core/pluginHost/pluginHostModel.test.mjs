import assert from "node:assert/strict";
import test from "node:test";
import {
  createPluginHostState,
  pluginHostActionFailed,
  selectActivePlugin,
  summarizePluginRecords,
  toPluginNavigationItems,
} from "/private/tmp/zero-tests/core/pluginHost/pluginHostModel.js";

function pluginRecord(name, enabled = true, source = "bundled") {
  return {
    name,
    version: "0.1.0",
    author: "watson",
    source,
    enabled,
    health: enabled ? "ready" : "disabled",
    manifest: {
      name,
      version: "0.1.0",
      author: "watson",
      main: "dist/index.html",
      permissions: ["ui.message"],
      displayName: name === "zero.snap" ? "Zero Snap" : undefined,
      description: name === "market-tool" ? "Market tool" : undefined,
    },
    approvedPermissions: ["ui.message"],
  };
}

test("selects requested enabled plugin or falls back to first enabled plugin", () => {
  const records = [
    pluginRecord("zero.snap", false),
    pluginRecord("zero.awake", true),
  ];

  assert.equal(selectActivePlugin(records, "zero.awake")?.name, "zero.awake");
  assert.equal(selectActivePlugin(records, "zero.snap")?.name, "zero.awake");
  assert.equal(selectActivePlugin(records, "missing")?.name, "zero.awake");
  assert.equal(
    selectActivePlugin([pluginRecord("zero.snap")], "ztool.screenshot")?.name,
    "zero.snap",
  );
});

test("empty plugin records produce no active plugin or navigation items", () => {
  const state = createPluginHostState([]);

  assert.equal(state.activePlugin, undefined);
  assert.deepEqual(state.navigationItems, []);
  assert.equal(state.summary.total, 0);
});

test("navigation items include enabled plugins and keep disabled diagnostics", () => {
  const records = [
    pluginRecord("zero.snap", true),
    pluginRecord("market-tool", false, "market"),
  ];

  assert.deepEqual(toPluginNavigationItems(records), [
    {
      id: "zero.snap",
      title: "Zero Snap",
      subtitle: "bundled · 0.1.0",
      health: "ready",
      enabled: true,
      source: "bundled",
    },
  ]);
});

test("host state derives active plugin and keeps action errors structured", () => {
  const state = createPluginHostState([
    pluginRecord("zero.snap", true),
    pluginRecord("market-tool", true, "market"),
  ], "market-tool");
  const failed = pluginHostActionFailed(state, "checksum mismatch");

  assert.equal(state.activePlugin?.name, "market-tool");
  assert.equal(failed.error, "checksum mismatch");
  assert.equal(failed.isBusy, false);
});

test("summarizes plugin records for about and diagnostics surfaces", () => {
  const summary = summarizePluginRecords([
    pluginRecord("zero.snap", true),
    pluginRecord("zero.awake", false),
    pluginRecord("market-tool", true, "market"),
    {
      ...pluginRecord("broken-tool", true, "local"),
      health: "error",
    },
  ]);

  assert.deepEqual(summary, {
    total: 4,
    bundled: 2,
    market: 1,
    local: 1,
    development: 0,
    enabled: 3,
    disabled: 1,
    failed: 1,
    incompatible: 0,
  });
});
