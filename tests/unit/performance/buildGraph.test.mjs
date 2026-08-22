import assert from "node:assert/strict";
import test from "node:test";
import {
  analyzeBuildGraph,
  budgetFindings,
  collectSourceGraph,
  forbiddenEagerFindings,
} from "../../../scripts/performance/build-graph.mjs";

test("build graph follows static imports and reports dynamic imports separately", () => {
  const manifest = {
    "src/main.tsx": {
      file: "assets/main.js",
      isEntry: true,
      imports: ["vendor.js"],
      dynamicImports: ["engine.tsx"],
    },
    "vendor.js": { file: "assets/vendor.js" },
    "engine.tsx": { file: "assets/engine.js" },
  };
  const sizes = new Map([
    ["assets/main.js", Buffer.alloc(10)],
    ["assets/vendor.js", Buffer.alloc(20)],
    ["assets/engine.js", Buffer.alloc(30)],
  ]);
  const report = analyzeBuildGraph(manifest, (file) => sizes.get(file));
  assert.equal(report.entries[0].staticBytes, 30);
  assert.deepEqual(report.entries[0].dynamicImports, ["engine.tsx"]);
  assert.deepEqual(budgetFindings(report, {
    initialEntryBytes: 29,
    initialEntryGzipBytes: Number.MAX_SAFE_INTEGER,
    largestChunkBytes: Number.MAX_SAFE_INTEGER,
  }), ["src/main.tsx static bytes 30 > 29"]);
});

test("source graph keeps dynamic surfaces out of eager dependency findings", () => {
  const sources = new Map([
    ["src/main.tsx", 'import "./shell"; import("./engine");'],
    ["src/shell.tsx", 'import React from "react";'],
    ["src/engine.tsx", 'import "pdfjs-dist";'],
  ]);
  const graph = collectSourceGraph(
    "src/main.tsx",
    (file) => sources.get(file),
    (importer, specifier) => {
      if (importer === "src/main.tsx" && specifier === "./shell") return "src/shell.tsx";
      return null;
    },
  );
  assert.deepEqual(graph.modules, ["src/main.tsx", "src/shell.tsx"]);
  assert.deepEqual(graph.dynamicImports, ["./engine"]);
  assert.deepEqual(forbiddenEagerFindings(graph, ["pdfjs-dist", "engine"]), []);
});
