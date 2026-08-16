import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const ROOT = process.cwd();
const PLUGINS_ROOT = path.join(ROOT, "src/plugins");
const CORE_ROOT = path.join(ROOT, "src/core");
const COMPOSITION_FILE = path.join(ROOT, "src/appShell/bundledPluginModules.ts");
const EXPECTED_PLUGINS = ["bingWallpaper", "caffeine", "file", "quickLauncher", "screenshot"];

test("plugins directory contains only self-contained registered bundled plugins", () => {
  const pluginDirectories = fs.readdirSync(PLUGINS_ROOT, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  assert.deepEqual(pluginDirectories, EXPECTED_PLUGINS);

  const composition = fs.readFileSync(COMPOSITION_FILE, "utf8");
  for (const plugin of EXPECTED_PLUGINS) {
    assert.ok(fs.existsSync(path.join(PLUGINS_ROOT, plugin, "plugin.tsx")));
    assert.match(composition, new RegExp(`plugins/${plugin}/plugin`));
  }
});

test("host core never imports a concrete plugin", () => {
  for (const file of sourceFiles(CORE_ROOT)) {
    const source = fs.readFileSync(file, "utf8");
    assert.doesNotMatch(source, /(?:from|import\()\s*["'][^"']*plugins\//, file);
  }
});

test("concrete frontend plugins never import sibling plugins", () => {
  for (const plugin of EXPECTED_PLUGINS) {
    const pluginRoot = path.join(PLUGINS_ROOT, plugin);
    for (const file of sourceFiles(pluginRoot)) {
      const source = fs.readFileSync(file, "utf8");
      for (const specifier of importSpecifiers(source)) {
        if (!specifier.startsWith(".")) continue;
        const resolved = path.resolve(path.dirname(file), specifier);
        if (!resolved.startsWith(`${PLUGINS_ROOT}${path.sep}`)) continue;
        assert.ok(
          resolved === pluginRoot || resolved.startsWith(`${pluginRoot}${path.sep}`),
          `${path.relative(ROOT, file)} imports sibling plugin through ${specifier}`,
        );
      }
    }
  }
});

function sourceFiles(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return /\.(?:ts|tsx)$/.test(entry.name) ? [target] : [];
  });
}

function importSpecifiers(source) {
  return [...source.matchAll(/(?:from\s*|import\()\s*["']([^"']+)["']/g)]
    .map((match) => match[1]);
}
