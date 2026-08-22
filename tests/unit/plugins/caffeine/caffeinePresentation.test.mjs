import assert from "node:assert/strict";
import test from "node:test";
import {
  shouldRefreshExpiredCaffeine,
  shouldRunCaffeinePresentationClock,
  startCaffeinePresentationClock,
} from "/private/tmp/zero-tests/plugins/caffeine/caffeinePresentation.js";

function fakeScheduler() {
  let now = 1_000;
  let nextId = 1;
  const callbacks = new Map();
  return {
    scheduler: {
      now: () => now,
      setInterval(callback) {
        const id = nextId++;
        callbacks.set(id, callback);
        return id;
      },
      clearInterval(id) {
        callbacks.delete(id);
      },
    },
    advance(ms) {
      now += ms;
      for (const callback of callbacks.values()) callback();
    },
    activeCount: () => callbacks.size,
  };
}

const activeReady = {
  enabled: true,
  surfaceActivity: "active",
  authoritativeSnapshotReady: true,
};

test("inactive, hidden, and unsynchronized Awake surfaces schedule no clock", () => {
  assert.equal(shouldRunCaffeinePresentationClock({ ...activeReady, enabled: false }), false);
  assert.equal(shouldRunCaffeinePresentationClock({ ...activeReady, surfaceActivity: "hidden" }), false);
  assert.equal(shouldRunCaffeinePresentationClock({ ...activeReady, authoritativeSnapshotReady: false }), false);

  const fake = fakeScheduler();
  startCaffeinePresentationClock({ ...activeReady, surfaceActivity: "hidden" }, fake.scheduler, () => undefined);
  assert.equal(fake.activeCount(), 0);
});

test("visible enabled Awake ticks once per second and cleans up on hide", () => {
  const fake = fakeScheduler();
  const ticks = [];
  const stop = startCaffeinePresentationClock(activeReady, fake.scheduler, (now) => ticks.push(now));

  assert.deepEqual(ticks, [1_000]);
  assert.equal(fake.activeCount(), 1);
  fake.advance(1_000);
  assert.deepEqual(ticks, [1_000, 2_000]);
  stop();
  assert.equal(fake.activeCount(), 0);
});

test("reveal waits for an authoritative snapshot before restarting", () => {
  const fake = fakeScheduler();
  startCaffeinePresentationClock(
    { ...activeReady, authoritativeSnapshotReady: false },
    fake.scheduler,
    () => undefined,
  );
  assert.equal(fake.activeCount(), 0);

  const stop = startCaffeinePresentationClock(activeReady, fake.scheduler, () => undefined);
  assert.equal(fake.activeCount(), 1);
  stop();
});

test("expiry while hidden and stale disabled expiry never trigger frontend refresh", () => {
  assert.equal(
    shouldRefreshExpiredCaffeine(
      { ...activeReady, surfaceActivity: "hidden" },
      1_000,
      0,
    ),
    false,
  );
  assert.equal(
    shouldRefreshExpiredCaffeine({ ...activeReady, enabled: false }, 1_000, 0),
    false,
  );
  assert.equal(shouldRefreshExpiredCaffeine(activeReady, 1_000, 0), true);
});

test("multiple visible Awake surfaces own independent clocks", () => {
  const fake = fakeScheduler();
  const stopTray = startCaffeinePresentationClock(activeReady, fake.scheduler, () => undefined);
  const stopMain = startCaffeinePresentationClock(activeReady, fake.scheduler, () => undefined);
  assert.equal(fake.activeCount(), 2);
  stopTray();
  assert.equal(fake.activeCount(), 1);
  stopMain();
  assert.equal(fake.activeCount(), 0);
});
