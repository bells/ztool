import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const panel = fs.readFileSync("src/plugins/file/FilePanel.tsx", "utf8");
const css = fs.readFileSync("src/App.css", "utf8");

test("File drag and picker intake enqueue reviewed candidates without auto-start", () => {
  assert.match(panel, /onDragDropEvent/);
  assert.match(panel, /controller\.inspect\(paths\)/);
  assert.match(panel, /controller\.choose\(\)/);
  assert.match(panel, /controller\.enqueue\(intake\.enqueueItems\)/);
  assert.equal((panel.match(/controller\.start\(\)/g) ?? []).length, 1);
  assert.match(panel, /onClick=\{\(\) => void controller\.start\(\)\}/);
});

test("File exposes truthful progress and job-scoped actions", () => {
  assert.match(panel, /role="progressbar"/);
  assert.match(panel, /aria-valuenow=\{summary\.percent\}/);
  assert.match(panel, /file\.indeterminate/);
  for (const action of ["cancel", "remove", "retry", "open", "reveal"]) {
    assert.match(panel, new RegExp(`controller\\.${action}\\(job\\.id\\)`));
  }
  assert.match(panel, /controller\.actionError\?\.owner === job\.id/);
});

test("File keeps one responsive queue region with accessible and reduced-motion structure", () => {
  assert.equal((panel.match(/className="file-queue-region"/g) ?? []).length, 1);
  assert.match(panel, /aria-live="polite"/);
  assert.match(panel, /role="alert"/);
  assert.match(css, /\.file-queue-region[\s\S]*overflow: auto/);
  assert.match(css, /\.main-window-shell \.file-job-row/);
  assert.match(css, /@media \(max-width: 820px\)[\s\S]*grid-template-columns: 1fr/);
  assert.match(css, /@media \(prefers-reduced-motion: reduce\)/);
  assert.match(css, /\.main-window-shell \.file-job-actions button,[\s\S]*min-height: 44px/);
});

test("File built-in readiness, fidelity warnings, and keyboard flow use native accessible controls", () => {
  assert.match(panel, /guidance\.providerOrigin === "builtIn"/);
  assert.match(panel, /t\("provider\.offlineReady"\)/);
  assert.match(panel, /qualityTranslationKey\(job\.state\.result\.qualityProfile\)/);
  for (const profile of [
    "editableReconstruction",
    "layoutPreserving",
    "webRenderedPdf",
    "compatibilityProvider",
  ]) {
    assert.match(panel, new RegExp(`${profile}: "quality\\.`));
  }
  assert.equal((panel.match(/<button\b/g) ?? []).length > 0, true);
  assert.equal((panel.match(/<button\b/g) ?? []).length, (panel.match(/type="button"/g) ?? []).length);
  assert.match(panel, /disabled=\{!queueActions\.canStart/);
  assert.match(panel, /role="alert"/);
  assert.match(panel, /aria-live="polite"/);
});
