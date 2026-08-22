import assert from "node:assert/strict";
import test from "node:test";
import {
  createActionGate,
  createRequestGate,
  errorMessage,
  loadBingWallpaperCacheFirst,
  nextBingWallpaperReloadVersion,
  previewBytesMatchDescriptor,
  shouldStartBingWallpaperPresentation,
} from "/private/tmp/zero-tests/plugins/bingWallpaper/bingWallpaperController.js";

function snapshot(stale = false, error) {
  return {
    items: [],
    market: "zh-CN",
    stale,
    platform: { platform: "macos", supported: true },
    ...(error ? { error } : {}),
  };
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

test("publishes cached snapshot before refreshed snapshot", async () => {
  const events = [];
  const cached = snapshot(false);
  const refreshed = snapshot(false);
  await loadBingWallpaperCacheFirst(
    {
      snapshot: async () => {
        events.push("snapshot-call");
        return cached;
      },
      refresh: async () => {
        events.push("refresh-call");
        return refreshed;
      },
    },
    createRequestGate(),
    (value) => events.push(value === cached ? "cached" : "refreshed"),
    (message) => events.push(`error:${message}`),
  );

  assert.deepEqual(events, ["snapshot-call", "cached", "refresh-call", "refreshed"]);
});

test("preserves stale refresh data and reports its structured error", async () => {
  const events = [];
  const stale = snapshot(true, {
    code: "network.timeout",
    message: "Bing timed out",
    retryable: true,
  });
  await loadBingWallpaperCacheFirst(
    { snapshot: async () => snapshot(), refresh: async () => stale },
    createRequestGate(),
    (value) => events.push(value),
    (message) => events.push(message),
  );

  assert.equal(events[1], stale);
  assert.equal(events[2], "Bing timed out");
});

test("disposed gates prevent stale async completions from updating state", async () => {
  const pending = deferred();
  const gate = createRequestGate();
  const snapshots = [];
  let refreshCalls = 0;
  const loading = loadBingWallpaperCacheFirst(
    {
      snapshot: () => pending.promise,
      refresh: async () => {
        refreshCalls += 1;
        return snapshot();
      },
    },
    gate,
    (value) => snapshots.push(value),
    () => assert.fail("disposed request must not report an error"),
  );

  gate.dispose();
  pending.resolve(snapshot());
  await loading;
  assert.deepEqual(snapshots, []);
  assert.equal(refreshCalls, 0);
});

test("normalizes Error objects structured errors and primitive failures", () => {
  assert.equal(errorMessage(new Error("native error")), "native error");
  assert.equal(errorMessage({ message: "structured error" }), "structured error");
  assert.equal(errorMessage("plain error"), "plain error");
});

test("action gate prevents duplicate and overlapping apply/save operations", () => {
  const gate = createActionGate();

  assert.equal(gate.tryStart("apply"), true);
  assert.equal(gate.current(), "apply");
  assert.equal(gate.tryStart("apply"), false);
  assert.equal(gate.tryStart("save"), false);
  gate.finish("save");
  assert.equal(gate.current(), "apply");
  gate.finish("apply");
  assert.equal(gate.current(), null);
  assert.equal(gate.tryStart("save"), true);
});

test("retry versions advance monotonically", () => {
  assert.equal(nextBingWallpaperReloadVersion(0), 1);
  assert.equal(nextBingWallpaperReloadVersion(7), 8);
});

test("hidden Paper surfaces do not start presentation and preview bytes must match descriptors", () => {
  assert.equal(shouldStartBingWallpaperPresentation("active"), true);
  assert.equal(shouldStartBingWallpaperPresentation("hidden"), false);
  assert.equal(shouldStartBingWallpaperPresentation("disposed"), false);
  assert.equal(previewBytesMatchDescriptor(1024, 1024), true);
  assert.equal(previewBytesMatchDescriptor(0, 0), false);
  assert.equal(previewBytesMatchDescriptor(1023, 1024), false);
});

test("selection replacement accepts only the newest preview completion", async () => {
  const olderPreview = deferred();
  const newerPreview = deferred();
  const olderGate = createRequestGate();
  const newerGate = createRequestGate();
  const published = [];

  const olderRequest = olderPreview.promise.then((value) => {
    if (olderGate.isCurrent()) published.push(value);
  });
  olderGate.dispose();
  const newerRequest = newerPreview.promise.then((value) => {
    if (newerGate.isCurrent()) published.push(value);
  });

  newerPreview.resolve("newer");
  olderPreview.resolve("older");
  await Promise.all([olderRequest, newerRequest]);
  assert.deepEqual(published, ["newer"]);
});
