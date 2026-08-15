import assert from "node:assert/strict";
import test from "node:test";
import {
  INITIAL_PLUGIN_MARKET_STATE,
  marketRefreshFailed,
  marketRefreshStarted,
  marketRefreshSucceeded,
  toPluginInstallCards,
} from "/private/tmp/zero-tests/core/pluginHost/pluginMarketModel.js";

const marketEntry = {
  name: "clipboard-helper",
  version: "0.1.0",
  author: "watson",
  repository: "https://github.com/watson/clipboard-helper",
  releaseUrl: "https://github.com/watson/clipboard-helper/releases/tag/v0.1.0",
  downloadUrl: "https://github.com/watson/clipboard-helper/releases/download/v0.1.0/clipboard-helper.zplugin",
  permissions: ["clipboard.read"],
  description: "Clipboard helper",
};

test("maps market entries to install cards with install and checksum state", () => {
  const cards = toPluginInstallCards([
    {
      ...marketEntry,
      installedVersion: "0.1.0",
      sha256: "0".repeat(64),
    },
  ]);

  assert.deepEqual(cards, [
    {
      name: "clipboard-helper",
      title: "clipboard-helper",
      version: "0.1.0",
      author: "watson",
      description: "Clipboard helper",
      permissions: ["clipboard.read"],
      installedVersion: "0.1.0",
      isInstalled: true,
      checksumStatus: "verified",
      releaseUrl: "https://github.com/watson/clipboard-helper/releases/tag/v0.1.0",
    },
  ]);
});

test("refresh success clears errors and preserves stale flag from snapshot", () => {
  const loading = marketRefreshStarted(INITIAL_PLUGIN_MARKET_STATE);
  const next = marketRefreshSucceeded({
    sourceUrl: "https://github.com/bells/zero/market.json",
    schemaVersion: 1,
    updatedAt: "2026-06-21T00:00:00Z",
    entries: [marketEntry],
    stale: false,
  });

  assert.equal(loading.isLoading, true);
  assert.equal(next.isLoading, false);
  assert.equal(next.error, null);
  assert.equal(next.stale, false);
  assert.equal(next.installCards.length, 1);
});

test("refresh failure keeps cached cards and marks market stale", () => {
  const current = marketRefreshSucceeded({
    sourceUrl: "https://github.com/bells/zero/market.json",
    schemaVersion: 1,
    entries: [marketEntry],
    stale: false,
  });

  const failed = marketRefreshFailed(current, new Error("network unavailable"));

  assert.equal(failed.isLoading, false);
  assert.equal(failed.stale, true);
  assert.equal(failed.error, "network unavailable");
  assert.deepEqual(failed.installCards, current.installCards);
});
