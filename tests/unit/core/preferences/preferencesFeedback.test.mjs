import test from "node:test";
import assert from "node:assert/strict";
import {
  IDLE_PREFERENCE_FEEDBACK,
  createPreferenceOperationGate,
  isPreferenceOperationPending,
  preferenceFeedbackFor,
  updatePreferenceFeedback,
} from "/private/tmp/zero-tests/core/preferences/preferencesFeedback.js";

test("preference feedback is keyed and defaults to idle", () => {
  assert.deepEqual(preferenceFeedbackFor({}, "general.language"), IDLE_PREFERENCE_FEEDBACK);

  const pending = updatePreferenceFeedback({}, "general.language", "pending");
  assert.equal(isPreferenceOperationPending(pending, "general.language"), true);
  assert.equal(preferenceFeedbackFor(pending, "general.autostart").status, "idle");
});

test("preference operation gate rejects duplicate work until completion", () => {
  const gate = createPreferenceOperationGate();

  assert.equal(gate.tryStart("general.autostart"), true);
  assert.equal(gate.tryStart("general.autostart"), false);
  assert.equal(gate.isActive("general.autostart"), true);
  gate.finish("general.autostart");
  assert.equal(gate.tryStart("general.autostart"), true);
});

test("preference feedback retains actionable error text", () => {
  const failed = updatePreferenceFeedback(
    {},
    "general.autostart",
    "error",
    "permission denied",
  );

  assert.deepEqual(preferenceFeedbackFor(failed, "general.autostart"), {
    status: "error",
    message: "permission denied",
  });
});
