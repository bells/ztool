import assert from "node:assert/strict";
import test from "node:test";
import {
  FIRST_PARTY_PLUGIN_IDS,
  PRODUCT_NAME,
  canonicalFirstPartyContributionId,
  canonicalFirstPartyPluginId,
} from "/private/tmp/zero-tests/brand/identity.js";

test("defines the canonical Zero product and first-party plugin ids", () => {
  assert.equal(PRODUCT_NAME, "Zero");
  assert.deepEqual(FIRST_PARTY_PLUGIN_IDS, {
    launch: "zero.launch",
    snap: "zero.snap",
    awake: "zero.awake",
    paper: "zero.paper",
    file: "zero.file",
  });
});

test("normalizes only known legacy first-party ids and contributions", () => {
  assert.equal(canonicalFirstPartyPluginId("ztool.quick-launcher"), "zero.launch");
  assert.equal(canonicalFirstPartyPluginId("ztool.screenshot"), "zero.snap");
  assert.equal(canonicalFirstPartyPluginId("ztool.caffeine"), "zero.awake");
  assert.equal(canonicalFirstPartyPluginId("ztool.bing-wallpaper"), "zero.paper");
  assert.equal(canonicalFirstPartyPluginId("ztool.third-party"), "ztool.third-party");
  assert.equal(
    canonicalFirstPartyContributionId("ztool.screenshot.capture"),
    "zero.snap.capture",
  );
  assert.equal(
    canonicalFirstPartyContributionId("ztool.third-party.capture"),
    "ztool.third-party.capture",
  );
});
