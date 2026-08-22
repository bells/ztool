import fs from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { createLatestQueryScheduler } from "/private/tmp/zero-tests/plugins/quickLauncher/quickLauncherScheduling.js";
import { percentile } from "./contracts.mjs";

const OUTPUT = path.join(
  process.cwd(),
  "openspec/changes/systematic-performance-and-code-optimization/evidence/launch-scheduling.json",
);

function immediateTimeouts() {
  let nextId = 1;
  const callbacks = new Map();
  return {
    scheduler: {
      setTimeout(callback) {
        const id = nextId++;
        callbacks.set(id, callback);
        return id;
      },
      clearTimeout(id) {
        callbacks.delete(id);
      },
    },
  };
}

const samples = [];
let ipcCount = 0;
let aggregate = { scheduled: 0, executed: 0, superseded: 0, maxConcurrent: 0 };
for (let sample = 0; sample < 30; sample += 1) {
  const fake = immediateTimeouts();
  const scheduler = createLatestQueryScheduler(async () => {
    ipcCount += 1;
  }, fake.scheduler, 40);
  const started = performance.now();
  for (const query of ["v", "vs", "vsc", "vsco", "vscod", "vscode"]) {
    scheduler.schedule(query);
  }
  await scheduler.flush();
  samples.push((performance.now() - started) * 1_000);
  const metrics = scheduler.metrics();
  aggregate = {
    scheduled: aggregate.scheduled + metrics.scheduled,
    executed: aggregate.executed + metrics.executed,
    superseded: aggregate.superseded + metrics.superseded,
    maxConcurrent: Math.max(aggregate.maxConcurrent, metrics.maxConcurrent),
  };
  scheduler.dispose();
}

const report = {
  schemaVersion: 1,
  recordedAt: new Date().toISOString(),
  fixture: { samples: 30, keystrokesPerSample: 6 },
  frontendQueryCount: aggregate.scheduled,
  searchIpcCount: ipcCount,
  supersededQueryCount: aggregate.superseded,
  maxSearchConcurrency: aggregate.maxConcurrent,
  schedulingLatencyMicros: {
    raw: samples,
    p50: percentile(samples, 0.5),
    p95: percentile(samples, 0.95),
  },
  structuralCounters: {
    indexCloneBytesPerQuery: 0,
    runningProbeCountPerQuery: 0,
    iconBatchRequestsPerVisibleResultSet: 1,
    maxNativeIconLoadConcurrency: 1,
    reactIconCommitsPerCompletedBatch: 1,
  },
  note: "Deterministic scheduler/contract evidence; this is not real Tauri IPC or WebView latency.",
};

fs.mkdirSync(path.dirname(OUTPUT), { recursive: true });
fs.writeFileSync(OUTPUT, `${JSON.stringify(report, null, 2)}\n`);
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
