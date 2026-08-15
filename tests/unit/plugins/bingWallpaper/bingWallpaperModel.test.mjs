import assert from "node:assert/strict";
import test from "node:test";
import {
  bingWallpaperDisplayTitle,
  createBingWallpaperNavigation,
  resolveBingWallpaperSelection,
  selectNewerBingWallpaper,
  selectOlderBingWallpaper,
  sortBingWallpapers,
} from "/private/tmp/zero-tests/plugins/bingWallpaper/bingWallpaperModel.js";

function wallpaper(id, startDate, overrides = {}) {
  return {
    id,
    startDate,
    title: `Title ${id}`,
    attribution: `Attribution ${id}`,
    remoteUrl: `https://www.bing.com/th?id=${id}`,
    cacheFileName: `${startDate}-${id}.jpg`,
    cached: true,
    ...overrides,
  };
}

const items = [
  wallpaper("older", "20260713"),
  wallpaper("newest", "20260715"),
  wallpaper("middle", "20260714"),
];

test("sorts newest first and selects newest by default", () => {
  assert.deepEqual(sortBingWallpapers(items).map((item) => item.id), [
    "newest",
    "middle",
    "older",
  ]);
  assert.equal(resolveBingWallpaperSelection(items, null), "newest");
  assert.equal(createBingWallpaperNavigation(items, null).selected?.id, "newest");
});

test("navigates older and newer with stable boundaries", () => {
  assert.equal(selectOlderBingWallpaper(items, "newest"), "middle");
  assert.equal(selectOlderBingWallpaper(items, "middle"), "older");
  assert.equal(selectOlderBingWallpaper(items, "older"), "older");
  assert.equal(selectNewerBingWallpaper(items, "older"), "middle");
  assert.equal(selectNewerBingWallpaper(items, "middle"), "newest");
  assert.equal(selectNewerBingWallpaper(items, "newest"), "newest");
});

test("preserves stable IDs after refresh and falls back when selection disappears", () => {
  const reordered = [wallpaper("middle", "20260716"), wallpaper("newest", "20260715")];
  assert.equal(resolveBingWallpaperSelection(reordered, "newest"), "newest");
  assert.equal(resolveBingWallpaperSelection(reordered, "older"), "middle");
  assert.equal(resolveBingWallpaperSelection([], "newest"), null);
  assert.equal(createBingWallpaperNavigation([], null).selected, null);
});

test("handles single items and conservative display title fallback", () => {
  const single = [wallpaper("only", "20260715")];
  const navigation = createBingWallpaperNavigation(single, "only");
  assert.equal(navigation.canSelectOlder, false);
  assert.equal(navigation.canSelectNewer, false);
  assert.equal(
    bingWallpaperDisplayTitle(
      wallpaper("fallback", "20260715", {
        title: " ",
        attribution: "Mountain lake (© Photographer)",
      }),
      "Wallpaper",
    ),
    "Mountain lake",
  );
  assert.equal(
    bingWallpaperDisplayTitle(
      wallpaper("empty", "20260715", { title: "", attribution: "" }),
      "Wallpaper",
    ),
    "Wallpaper",
  );
});
