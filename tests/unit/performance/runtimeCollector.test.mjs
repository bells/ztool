import assert from "node:assert/strict";
import test from "node:test";
import {
  parsePerformanceEvents,
  classifyRuntimeCoverage,
  summarizePerformanceEvents,
  summarizePerformanceMeasurements,
} from "../../../scripts/performance/runtime-collector.mjs";

test("runtime collector parses only structured performance lines", () => {
  const events = parsePerformanceEvents([
    "ordinary log",
    'ZERO_PERF {"sequence":1,"phase":"migration","outcome":"ok","startedAtUs":0,"durationUs":1000}',
    'ZERO_PERF {"sequence":2,"phase":"migration","outcome":"ok","startedAtUs":2,"durationUs":3000}',
  ].join("\n"));
  assert.equal(events.length, 2);
  assert.deepEqual(summarizePerformanceEvents(events).migration.raw, [1, 3]);
  assert.equal(summarizePerformanceEvents(events).migration.median, 1);
});

test("runtime collector separates media bytes and reports required coverage", () => {
  const events = [
    { phase: "first_frontend_ready:tray", durationUs: 10_000 },
    { phase: "surface_reveal:tray", durationUs: 2_000 },
    { phase: "plugin_activation:first:tray:zero.snap", durationUs: 3_000 },
    { phase: "plugin_activation:repeat:tray:zero.snap", durationUs: 1_000 },
    { phase: "media_transfer:screenshot_read", durationUs: 0, value: 100, unit: "bytes" },
    { phase: "media_transfer:screenshot_read", durationUs: 0, value: 50, unit: "bytes" },
  ];
  assert.deepEqual(
    summarizePerformanceMeasurements(events)["media_transfer:screenshot_read:bytes"],
    { count: 2, min: 50, median: 50, p95: 100, max: 100, raw: [100, 50], total: 150 },
  );
  assert.deepEqual(classifyRuntimeCoverage(events, [{ rssBytes: 1, cpuPercent: 0 }]), {
    coldStartup: true,
    warmReveal: true,
    firstPluginActivation: true,
    repeatPluginActivation: true,
    mediaTransferBytes: true,
    processTreeCpuRss: true,
  });
});
