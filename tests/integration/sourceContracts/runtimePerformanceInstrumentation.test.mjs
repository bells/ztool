import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const [mainSource, lazyPanelSource, surfaceSource, performanceSource, collectorSource] =
  await Promise.all([
    readFile(new URL("../../../src/main.tsx", import.meta.url), "utf8"),
    readFile(new URL("../../../src/appShell/LazyPluginPanel.tsx", import.meta.url), "utf8"),
    readFile(new URL("../../../src-tauri/src/services/surface_activity.rs", import.meta.url), "utf8"),
    readFile(new URL("../../../src-tauri/src/services/performance.rs", import.meta.url), "utf8"),
    readFile(new URL("../../../scripts/performance/runtime-collector.mjs", import.meta.url), "utf8"),
  ]);

test("pairs native reveal starts with next-frame frontend acknowledgements", () => {
  assert.match(surfaceSource, /begin_pending\(phase\.clone\(\)\)/);
  assert.match(surfaceSource, /surface_reveal:/);
  assert.match(mainSource, /SURFACE_ACTIVITY_EVENT/);
  assert.match(mainSource, /requestAnimationFrame\(\(\) => \{/);
  assert.match(mainSource, /invoke\("mark_surface_ready"\)/);
  assert.match(mainSource, /invoke\("mark_frontend_ready"\)/);
});

test("records first and repeat plugin activation only after lazy panels mount", () => {
  assert.match(lazyPanelSource, /activatedPluginIds = new Set<string>/);
  assert.match(lazyPanelSource, /<PluginActivationMarker pluginId=\{pluginId\}/);
  assert.match(lazyPanelSource, /invoke\("record_plugin_activation"/);
  assert.match(lazyPanelSource, /durationUs:/);
});

test("runtime events carry media bytes and collector reports full coverage independently", () => {
  assert.match(performanceSource, /pub value: Option<u64>/);
  assert.match(performanceSource, /pub unit: Option<String>/);
  assert.match(performanceSource, /media_transfer:/);
  assert.match(collectorSource, /measurements: summarizePerformanceMeasurements\(events\)/);
  assert.match(collectorSource, /rssBytes/);
  assert.match(collectorSource, /cpuPercent/);
  assert.match(collectorSource, /firstPluginActivation/);
  assert.match(collectorSource, /repeatPluginActivation/);
});
