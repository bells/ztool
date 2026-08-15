import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const selection = process.argv[2] ?? "all";
const roots =
  selection === "unit"
    ? ["tests/unit"]
    : selection === "integration"
      ? ["tests/integration"]
      : selection === "all"
        ? ["tests/unit", "tests/integration"]
        : [];

if (roots.length === 0) {
  throw new Error(`Unknown test selection: ${selection}`);
}

const files = roots.flatMap(testFiles).sort();
if (files.length === 0) {
  throw new Error(`No tests found for ${selection}`);
}

const result = spawnSync(process.execPath, ["--test", ...files], {
  cwd: process.cwd(),
  stdio: "inherit",
});
process.exit(result.status ?? 1);

function testFiles(root) {
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) return testFiles(target);
    return entry.name.endsWith(".test.mjs") ? [target] : [];
  });
}
