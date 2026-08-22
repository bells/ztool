import fs from "node:fs";
import path from "node:path";
import { createRunMetadata } from "./contracts.mjs";

const options = parseArguments(process.argv.slice(2));
const result = createRunMetadata(options);
const serialized = `${JSON.stringify(result, null, 2)}\n`;

if (options.output) {
  fs.mkdirSync(path.dirname(options.output), { recursive: true });
  fs.writeFileSync(options.output, serialized);
}
process.stdout.write(serialized);

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
  return {
    kind: values.get("kind") ?? "environment",
    buildMode: values.get("build-mode") ?? "release",
    command: values.get("command") ?? "not-recorded",
    sampleCount: integer(values.get("sample-count"), 0),
    warmupCount: integer(values.get("warmup-count"), 0),
    settleMs: integer(values.get("settle-ms"), 0),
    displaySetup: values.get("display") ?? "not-recorded",
    output: values.get("output"),
  };
}

function integer(value, fallback) {
  if (value === undefined) return fallback;
  const parsed = Number.parseInt(value, 10);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new Error(`Expected a non-negative integer, received ${value}`);
  }
  return parsed;
}
