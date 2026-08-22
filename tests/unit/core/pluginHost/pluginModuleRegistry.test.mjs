import assert from "node:assert/strict";
import test from "node:test";
import { createBundledPluginRegistry } from "/private/tmp/zero-tests/core/pluginHost/pluginModuleRegistry.js";

function pluginModule(id, contributionId = `${id}.main`) {
  return {
    kind: id,
    accentClass: `accent-${id}`,
    manifest: {
      name: id,
      id,
      version: "1.0.0",
      author: "tests",
      main: `plugins/${id}`,
      permissions: ["ui.message"],
      contributes: {
        views: [{ id: contributionId, title: id, surface: "main" }],
      },
    },
    presentation: {
      "zh-CN": { title: id, subtitle: id },
      "en-US": { title: id, subtitle: id },
    },
    loadPanel: async () => ({ default: () => null }),
  };
}

test("builds a lookup registry from complete unique bundled modules", () => {
  const modules = [pluginModule("tool.one"), pluginModule("tool.two")];
  const registry = createBundledPluginRegistry(modules);

  assert.deepEqual(registry.modules, modules);
  assert.deepEqual(registry.manifests, modules.map((module) => module.manifest));
  assert.equal(registry.find("tool.two"), modules[1]);
  assert.equal(registry.find("missing"), undefined);
});

test("rejects duplicate canonical plugin identities", () => {
  assert.throws(
    () => createBundledPluginRegistry([
      pluginModule("zero.snap"),
      pluginModule("ztool.screenshot", "legacy.snap.main"),
    ]),
    /Duplicate bundled plugin id: zero\.snap/,
  );
});

test("rejects conflicting contribution identifiers", () => {
  assert.throws(
    () => createBundledPluginRegistry([
      pluginModule("tool.one", "shared.main"),
      pluginModule("tool.two", "shared.main"),
    ]),
    /Duplicate bundled contribution id shared\.main/,
  );
});

test("rejects invalid manifests and incomplete presentation metadata", () => {
  assert.throws(
    () => createBundledPluginRegistry([pluginModule("BAD ID")]),
    /Invalid bundled plugin/,
  );
  assert.throws(
    () => createBundledPluginRegistry([{ ...pluginModule("tool.one"), kind: "" }]),
    /missing presentation metadata/,
  );
  assert.throws(
    () => createBundledPluginRegistry([{ ...pluginModule("tool.one"), loadPanel: null }]),
    /missing presentation metadata/,
  );
});
