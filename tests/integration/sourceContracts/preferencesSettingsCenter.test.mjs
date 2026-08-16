import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const app = fs.readFileSync("src/App.tsx", "utf8");
const center = fs.readFileSync("src/core/preferences/PreferencesSettingsCenter.tsx", "utf8");
const pages = fs.readFileSync("src/core/preferences/PreferencesPages.tsx", "utf8");
const extensions = fs.readFileSync("src/core/pluginHost/PluginManagerPanel.tsx", "utf8");
const css = fs.readFileSync("src/App.css", "utf8");

test("preferences route uses one settings center instead of stacked legacy panels", () => {
  const preferencesApp = app.slice(app.indexOf("export function PreferencesWindowApp"), app.indexOf("export function AboutWindowApp"));
  assert.match(preferencesApp, /<PreferencesSettingsCenter/);
  assert.doesNotMatch(preferencesApp, /<PreferencesPanel|<PluginManagerPanel/);
});

test("settings center exposes search, selected destination, narrow back flow, and focus handoff", () => {
  assert.match(center, /type="search"/);
  assert.match(center, /aria-current=/);
  assert.match(center, /prefs\.back/);
  assert.match(center, /scrollIntoView/);
  assert.match(center, /\.focus\(/);
  assert.match(css, /@media \(max-width: 680px\)/);
  assert.match(css, /\.preferences-center\.navigation-open \.preferences-content/);
  assert.match(css, /prefers-reduced-motion/);
});

test("settings pages keep controls immediate and shortcuts read-only", () => {
  assert.doesNotMatch(pages, />\s*(Save|Cancel|Apply|OK)\s*</i);
  assert.doesNotMatch(pages, /type="text"[^>]*shortcut|shortcut[^>]*type="text"/i);
  assert.match(pages, /role="switch"/);
  assert.match(pages, /aria-live=/);
});

test("extension manager owns localized focused sections and preserves permission approval", () => {
  for (const settingId of [
    "extensions.market",
    "extensions.local",
    "extensions.installed",
    "extensions.restore",
    "extensions.diagnostics",
  ]) {
    assert.match(extensions, new RegExp(`preferencesSettingFocusTargetId\\(\"${settingId.replaceAll(".", "\\.")}\"\\)`));
  }
  assert.match(extensions, /approvedPermissions: pendingInstall\.permissions/);
  assert.match(extensions, /pluginHost\.reload\(\)/);
  assert.doesNotMatch(extensions, /Market refreshed\.|Git-based market|Validate local package|Installed plugins/);
});
