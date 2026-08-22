import { execFileSync } from "node:child_process";
import os from "node:os";

export const PERFORMANCE_SCHEMA_VERSION = 1;

export function createRunMetadata({
  kind,
  buildMode,
  command,
  sampleCount,
  warmupCount,
  settleMs,
  displaySetup = "not-recorded",
  notes = [],
} = {}) {
  const metadata = {
    schemaVersion: PERFORMANCE_SCHEMA_VERSION,
    recordedAt: new Date().toISOString(),
    kind,
    repository: readGitContext(),
    environment: readEnvironment(displaySetup),
    protocol: {
      buildMode,
      command,
      sampleCount,
      warmupCount,
      settleMs,
    },
    notes,
  };
  validateRunMetadata(metadata);
  return metadata;
}

export function validateRunMetadata(metadata) {
  const requiredStrings = [
    ["kind", metadata.kind],
    ["recordedAt", metadata.recordedAt],
    ["repository.commit", metadata.repository?.commit],
    ["environment.os", metadata.environment?.os],
    ["environment.architecture", metadata.environment?.architecture],
    ["environment.hardware", metadata.environment?.hardware],
    ["environment.memory", metadata.environment?.memory],
    ["environment.powerMode", metadata.environment?.powerMode],
    ["environment.displaySetup", metadata.environment?.displaySetup],
    ["protocol.buildMode", metadata.protocol?.buildMode],
    ["protocol.command", metadata.protocol?.command],
  ];
  const missing = requiredStrings
    .filter(([, value]) => typeof value !== "string" || value.length === 0)
    .map(([name]) => name);
  for (const [name, value] of [
    ["protocol.sampleCount", metadata.protocol?.sampleCount],
    ["protocol.warmupCount", metadata.protocol?.warmupCount],
    ["protocol.settleMs", metadata.protocol?.settleMs],
  ]) {
    if (!Number.isInteger(value) || value < 0) missing.push(name);
  }
  if (missing.length > 0) {
    throw new Error(`Incomplete performance metadata: ${missing.join(", ")}`);
  }
}

export function percentile(samples, percentileValue) {
  if (samples.length === 0) return null;
  const sorted = [...samples].sort((left, right) => left - right);
  const index = Math.max(
    0,
    Math.min(sorted.length - 1, Math.ceil(percentileValue * sorted.length) - 1),
  );
  return sorted[index];
}

export function summarizeSamples(samples) {
  return {
    count: samples.length,
    min: samples.length === 0 ? null : Math.min(...samples),
    median: percentile(samples, 0.5),
    p95: percentile(samples, 0.95),
    max: samples.length === 0 ? null : Math.max(...samples),
    raw: [...samples],
  };
}

function readGitContext() {
  return {
    commit: runOptional("git", ["rev-parse", "HEAD"]) ?? "unknown",
    dirty: (runOptional("git", ["status", "--porcelain"]) ?? "").length > 0,
  };
}

function readEnvironment(displaySetup) {
  const macosVersion = process.platform === "darwin"
    ? runOptional("sw_vers", ["-productVersion"])
    : null;
  const macosBuild = process.platform === "darwin"
    ? runOptional("sw_vers", ["-buildVersion"])
    : null;
  return {
    os: macosVersion
      ? `macOS ${macosVersion}${macosBuild ? ` (${macosBuild})` : ""}`
      : `${os.platform()} ${os.release()}`,
    architecture: os.arch(),
    hardware: `${os.cpus()[0]?.model ?? "unknown CPU"}; ${os.cpus().length} logical CPUs`,
    memory: `${Math.round(os.totalmem() / 1024 / 1024)} MiB`,
    powerMode: readPowerMode(),
    displaySetup,
  };
}

function readPowerMode() {
  if (process.platform !== "darwin") return "not-recorded";
  const power = runOptional("pmset", ["-g", "batt"]);
  if (!power) return "not-recorded";
  const firstLine = power.split("\n")[0]?.trim();
  return firstLine || "not-recorded";
}

function runOptional(command, args) {
  try {
    return execFileSync(command, args, {
      cwd: process.cwd(),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}
