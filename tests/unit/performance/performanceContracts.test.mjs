import assert from "node:assert/strict";
import test from "node:test";
import {
  percentile,
  summarizeSamples,
  validateRunMetadata,
} from "../../../scripts/performance/contracts.mjs";

test("performance samples retain raw values and deterministic percentiles", () => {
  const samples = [9, 1, 5, 3, 7];
  assert.equal(percentile(samples, 0.5), 5);
  assert.equal(percentile(samples, 0.95), 9);
  assert.deepEqual(summarizeSamples(samples), {
    count: 5,
    min: 1,
    median: 5,
    p95: 9,
    max: 9,
    raw: samples,
  });
});

test("performance metadata rejects missing protocol fields", () => {
  assert.throws(
    () => validateRunMetadata({ repository: {}, environment: {}, protocol: {} }),
    /Incomplete performance metadata/,
  );
});
