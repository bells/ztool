import assert from "node:assert/strict";
import test from "node:test";
import {
  CAFFEINE_DURATION_OPTIONS,
  formatCompactDuration,
  formatDurationClock,
  getRemainingMs,
} from "/private/tmp/zero-tests/plugins/caffeine/caffeineDuration.js";

test("defines no-limit and supported finite caffeine duration options", () => {
  assert.deepEqual(
    CAFFEINE_DURATION_OPTIONS.map((option) => option.minutes),
    [null, 5, 10, 15, 30, 60, 120, 300],
  );
});

test("formats compact duration labels for the tray panel", () => {
  assert.equal(formatCompactDuration(null), "∞");
  assert.equal(formatCompactDuration(5), "5m");
  assert.equal(formatCompactDuration(30), "30m");
  assert.equal(formatCompactDuration(60), "1h");
  assert.equal(formatCompactDuration(120), "2h");
  assert.equal(formatCompactDuration(300), "5h");
});

test("formats elapsed and remaining durations as compact clocks", () => {
  assert.equal(formatDurationClock(0), "00:00");
  assert.equal(formatDurationClock(65_000), "01:05");
  assert.equal(formatDurationClock(3_661_000), "01:01:01");
});

test("clamps remaining time at zero", () => {
  assert.equal(getRemainingMs(10_000, 7_500), 2_500);
  assert.equal(getRemainingMs(10_000, 12_000), 0);
  assert.equal(getRemainingMs(null, 12_000), null);
});
