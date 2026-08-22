import assert from "node:assert/strict";
import test from "node:test";
import {
  classifyPdfComplexity,
  DEFAULT_PDF_COMPLEXITY_POLICY,
} from "/private/tmp/zero-tests/plugins/file/engine/pdfComplexity.js";

function page(overrides = {}) {
  return {
    width: 612,
    height: 792,
    vectorOperationCount: 0,
    imageCount: 0,
    text: Array.from({ length: 12 }, (_, index) => ({
      text: `Stable paragraph ${index}`,
      x: 54,
      y: 70 + index * 40,
      width: 180,
      height: 12,
      rotationDegrees: 0,
    })),
    ...overrides,
  };
}

test("simple ordered text selects editable reconstruction", () => {
  assert.deepEqual(classifyPdfComplexity([page()]), {
    profile: "editableReconstruction",
    signals: [],
  });
});

test("scans, rotations, columns, overlaps, and dense vector tables select layout preservation", () => {
  const scan = page({ text: [], imageCount: 1 });
  const rotated = page({
    text: page().text.map((item) => ({ ...item, rotationDegrees: 90 })),
  });
  const columns = page({
    text: page().text.map((item, index) => ({ ...item, x: index % 2 ? 450 : 20 })),
  });
  const overlapping = page({
    text: page().text.map((item) => ({ ...item, x: 50, y: 50 })),
  });
  const vectors = page({
    vectorOperationCount: DEFAULT_PDF_COMPLEXITY_POLICY.maximumVectorOperationsPerPage + 1,
  });
  for (const candidate of [scan, rotated, columns, overlapping, vectors]) {
    assert.equal(classifyPdfComplexity([candidate]).profile, "layoutPreserving");
  }
});

test("the pinned vector threshold rejects the corpus table layout without rejecting plain text", () => {
  assert.equal(DEFAULT_PDF_COMPLEXITY_POLICY.maximumVectorOperationsPerPage, 16);
  assert.equal(classifyPdfComplexity([page({ vectorOperationCount: 20 })]).profile, "layoutPreserving");
  assert.equal(classifyPdfComplexity([page({ vectorOperationCount: 0 })]).profile, "editableReconstruction");
});
