import fs from "node:fs";
import path from "node:path";
import { createRunMetadata, summarizeSamples } from "./contracts.mjs";

export function parsePerformanceEvents(text) {
  return text
    .split(/\r?\n/)
    .filter((line) => line.startsWith("ZERO_PERF "))
    .map((line) => JSON.parse(line.slice("ZERO_PERF ".length)));
}

export function summarizePerformanceEvents(events) {
  const phases = {};
  for (const event of events.filter((event) => event.value === undefined)) {
    (phases[event.phase] ??= []).push(event.durationUs / 1000);
  }
  return Object.fromEntries(
    Object.entries(phases).map(([phase, samples]) => [phase, summarizeSamples(samples)]),
  );
}

export function summarizePerformanceMeasurements(events) {
  const measurements = {};
  for (const event of events.filter((event) => Number.isFinite(event.value))) {
    const key = `${event.phase}:${event.unit ?? "value"}`;
    (measurements[key] ??= []).push(event.value);
  }
  return Object.fromEntries(
    Object.entries(measurements).map(([key, samples]) => [
      key,
      { ...summarizeSamples(samples), total: samples.reduce((total, value) => total + value, 0) },
    ]),
  );
}

export function classifyRuntimeCoverage(events, processSamples) {
  const phases = events.map((event) => event.phase);
  return {
    coldStartup: phases.some((phase) => phase.startsWith("first_frontend_ready:")),
    warmReveal: phases.some((phase) => phase.startsWith("surface_reveal:")),
    firstPluginActivation: phases.some((phase) => phase.startsWith("plugin_activation:first:")),
    repeatPluginActivation: phases.some((phase) => phase.startsWith("plugin_activation:repeat:")),
    mediaTransferBytes: events.some(
      (event) => event.phase.startsWith("media_transfer:") && event.unit === "bytes",
    ),
    processTreeCpuRss: processSamples.length > 0 && processSamples.every(
      (sample) => Number.isFinite(sample.rssBytes) && Number.isFinite(sample.cpuPercent),
    ),
  };
}

function main() {
  const options = parseArguments(process.argv.slice(2));
  const input = fs.readFileSync(options.input, "utf8");
  const events = parsePerformanceEvents(input);
  if (events.length === 0) {
    throw new Error(`No ZERO_PERF events found in ${options.input}`);
  }
  const processSamples = options.processSamples
    ? JSON.parse(fs.readFileSync(options.processSamples, "utf8"))
    : [];
  const result = {
    metadata: createRunMetadata({
      kind: options.kind,
      buildMode: options.buildMode,
      command: options.command,
      sampleCount: events.length,
      warmupCount: options.warmupCount,
      settleMs: options.settleMs,
      displaySetup: options.display,
    }),
    phases: summarizePerformanceEvents(events),
    measurements: summarizePerformanceMeasurements(events),
    events,
    processTree: {
      rssBytes: summarizeSamples(processSamples.map((sample) => sample.rssBytes)),
      cpuPercent: summarizeSamples(processSamples.map((sample) => sample.cpuPercent)),
      raw: processSamples,
    },
    coverage: classifyRuntimeCoverage(events, processSamples),
  };
  const serialized = `${JSON.stringify(result, null, 2)}\n`;
  fs.mkdirSync(path.dirname(options.output), { recursive: true });
  fs.writeFileSync(options.output, serialized);
  process.stdout.write(serialized);
}

function parseArguments(args) {
  args = args.filter((argument) => argument !== "--");
  const values = new Map();
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      throw new Error(`Expected --key value arguments, received ${key ?? "<none>"}`);
    }
    values.set(key.slice(2), value);
  }
  const input = values.get("input");
  const output = values.get("output");
  if (!input || !output) throw new Error("--input and --output are required");
  return {
    input,
    output,
    processSamples: values.get("process-samples"),
    kind: values.get("kind") ?? "runtime",
    buildMode: values.get("build-mode") ?? "release",
    command: values.get("command") ?? "not-recorded",
    warmupCount: Number.parseInt(values.get("warmup-count") ?? "0", 10),
    settleMs: Number.parseInt(values.get("settle-ms") ?? "60000", 10),
    display: values.get("display") ?? "not-recorded",
  };
}

if (import.meta.url === `file://${process.argv[1]}`) main();
