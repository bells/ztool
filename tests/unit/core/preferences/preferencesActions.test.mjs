import test from "node:test";
import assert from "node:assert/strict";
import { updateLaunchAtLoginPreference } from "/private/tmp/zero-tests/core/preferences/preferencesActions.js";

const preferences = {
  launchAtLogin: false,
  language: "system",
  visibleTools: { "zero.snap": true },
};

test("autostart updates only after the native owner confirms success", async () => {
  const calls = [];
  const next = await updateLaunchAtLoginPreference(preferences, true, {
    enable: async () => calls.push("enable"),
    disable: async () => calls.push("disable"),
  });

  assert.deepEqual(calls, ["enable"]);
  assert.equal(next.launchAtLogin, true);
  assert.equal(preferences.launchAtLogin, false);
});

test("autostart failure leaves the previous preference unchanged", async () => {
  await assert.rejects(
    updateLaunchAtLoginPreference(preferences, true, {
      enable: async () => {
        throw new Error("permission denied");
      },
      disable: async () => undefined,
    }),
    /permission denied/,
  );
  assert.equal(preferences.launchAtLogin, false);
});
