import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_PREFERENCES,
  getVisiblePluginIds,
  normalizePreferences,
  setToolVisibility,
  setLanguagePreference,
} from "/private/tmp/zero-tests/core/preferences/preferencesModel.js";

test("normalizes missing visibility to all tools visible", () => {
  const preferences = normalizePreferences({}, ["screenshot", "caffeine"]);

  assert.equal(preferences.language, "system");
  assert.deepEqual(preferences.visibleTools, {
    screenshot: true,
    caffeine: true,
  });
});

test("filters visible plugin ids from preferences", () => {
  const preferences = normalizePreferences(
    {
      visibleTools: {
        screenshot: false,
        caffeine: true,
      },
    },
    ["screenshot", "caffeine"],
  );

  assert.deepEqual(getVisiblePluginIds(["screenshot", "caffeine"], preferences), ["caffeine"]);
});

test("keeps the final visible tool enabled", () => {
  const preferences = normalizePreferences(
    {
      ...DEFAULT_PREFERENCES,
      visibleTools: {
        screenshot: true,
        caffeine: false,
      },
    },
    ["screenshot", "caffeine"],
  );

  const next = setToolVisibility(preferences, "screenshot", false, ["screenshot", "caffeine"]);

  assert.equal(next.visibleTools.screenshot, true);
  assert.equal(next.visibleTools.caffeine, false);
});

test("normalizes invalid language to system and accepts explicit language", () => {
  assert.equal(
    normalizePreferences({ language: "pirate" }, ["screenshot", "caffeine"]).language,
    "system",
  );
  assert.equal(
    normalizePreferences({ language: "en-US" }, ["screenshot", "caffeine"]).language,
    "en-US",
  );
});

test("sets language preference without changing tool visibility", () => {
  const preferences = normalizePreferences({}, ["screenshot", "caffeine"]);
  const next = setLanguagePreference(preferences, "zh-CN");

  assert.equal(next.language, "zh-CN");
  assert.deepEqual(next.visibleTools, preferences.visibleTools);
});

test("supports registry plugin ids and migrates legacy bundled visibility keys", () => {
  const preferences = normalizePreferences(
    {
      visibleTools: {
        "ztool.screenshot": false,
        "ztool.caffeine": true,
        "ztool.third-party": false,
      },
    },
    ["zero.snap", "zero.awake", "zero.paper", "ztool.third-party"],
  );

  assert.deepEqual(preferences.visibleTools, {
    "zero.snap": false,
    "zero.awake": true,
    "zero.paper": true,
    "ztool.third-party": false,
  });
});
