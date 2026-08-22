import assert from "node:assert/strict";
import test from "node:test";
import {
  createLatestQueryScheduler,
  acceptsPresentationCompletion,
  mergeBoundedIconBatch,
  resolveActivationAfterFlush,
} from "/private/tmp/zero-tests/plugins/quickLauncher/quickLauncherScheduling.js";

function fakeTimeouts() {
  let nextId = 1;
  const callbacks = new Map();
  return {
    scheduler: {
      setTimeout(callback) {
        const id = nextId++;
        callbacks.set(id, callback);
        return id;
      },
      clearTimeout(id) {
        callbacks.delete(id);
      },
    },
    fire() {
      const pending = [...callbacks.values()];
      callbacks.clear();
      for (const callback of pending) callback();
    },
    activeCount: () => callbacks.size,
  };
}

test("fast typing coalesces to the latest query and records request metrics", async () => {
  const fake = fakeTimeouts();
  const queries = [];
  const scheduler = createLatestQueryScheduler(
    async (query) => queries.push(query),
    fake.scheduler,
    40,
  );
  scheduler.schedule("c");
  scheduler.schedule("co");
  scheduler.schedule("code");
  assert.equal(fake.activeCount(), 1);
  fake.fire();
  await scheduler.flush();

  assert.deepEqual(queries, ["code"]);
  assert.deepEqual(scheduler.metrics(), {
    scheduled: 3,
    executed: 1,
    superseded: 2,
    maxConcurrent: 1,
  });
});

test("Enter flushes the latest pending query before activation", async () => {
  const fake = fakeTimeouts();
  const queries = [];
  const scheduler = createLatestQueryScheduler(
    async (query) => queries.push(query),
    fake.scheduler,
    40,
  );
  scheduler.schedule("微");
  scheduler.schedule("微信");
  await scheduler.flush();
  assert.deepEqual(queries, ["微信"]);
  assert.equal(fake.activeCount(), 0);
});

test("a hidden surface cancels pending presentation work", async () => {
  const fake = fakeTimeouts();
  const queries = [];
  const scheduler = createLatestQueryScheduler(
    async (query) => queries.push(query),
    fake.scheduler,
    40,
  );
  scheduler.schedule("stale");
  scheduler.cancelPending();
  fake.fire();
  await scheduler.flush();
  assert.deepEqual(queries, []);

  scheduler.schedule("revealed");
  await scheduler.flush();
  assert.deepEqual(queries, ["revealed"]);
});

test("slow searches remain single-flight and preserve the newest successor", async () => {
  const fake = fakeTimeouts();
  const queries = [];
  let releaseFirst;
  const scheduler = createLatestQueryScheduler(async (query) => {
    queries.push(query);
    if (query === "first") {
      await new Promise((resolve) => {
        releaseFirst = resolve;
      });
    }
  }, fake.scheduler, 40);

  scheduler.schedule("first");
  fake.fire();
  scheduler.schedule("second");
  scheduler.schedule("latest");
  releaseFirst();
  await scheduler.flush();

  assert.deepEqual(queries, ["first", "latest"]);
  assert.equal(scheduler.metrics().maxConcurrent, 1);
});

test("icon batches merge in one bounded deterministic state transition", () => {
  const current = { old1: "one", old2: "two" };
  const next = mergeBoundedIconBatch(current, [
    { itemId: "new1", dataUrl: "three" },
    { itemId: "new2" },
  ], 3);
  assert.deepEqual(next, { old2: "two", new1: "three", new2: null });
  assert.deepEqual(current, { old1: "one", old2: "two" });
});

test("superseded and hidden icon completions are ignored", () => {
  assert.equal(acceptsPresentationCompletion(3, 3, "active"), true);
  assert.equal(acceptsPresentationCompletion(2, 3, "active"), false);
  assert.equal(acceptsPresentationCompletion(3, 3, "hidden"), false);
});

test("activation uses only the flushed result revision and stable item id", () => {
  const result = { revision: 9, items: [{ id: "latest" }] };
  assert.deepEqual(resolveActivationAfterFlush(undefined, "latest", result), {
    itemId: "latest",
    revision: 9,
  });
  assert.equal(resolveActivationAfterFlush("stale", "latest", result), null);
  assert.equal(resolveActivationAfterFlush(undefined, null, result), null);
});
