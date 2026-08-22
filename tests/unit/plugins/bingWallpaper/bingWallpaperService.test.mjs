import assert from "node:assert/strict";
import test from "node:test";
import {
  BING_WALLPAPER_COMMANDS,
  createBingWallpaperService,
} from "/private/tmp/zero-tests/plugins/bingWallpaper/bingWallpaperServiceCore.js";

test("uses stable command names and camelCase wallpaperId payloads", async () => {
  const calls = [];
  const service = createBingWallpaperService(async (command, args) => {
    calls.push([command, args]);
    return { command, args };
  });

  await service.snapshot();
  await service.refresh();
  await service.preview({ wallpaperId: "wallpaper-1" });
  await service.readPreview({ token: "preview-token" });
  await service.releasePreview({ token: "preview-token" });
  await service.save({ wallpaperId: "wallpaper-1" });
  await service.apply({ wallpaperId: "wallpaper-1" });

  assert.deepEqual(BING_WALLPAPER_COMMANDS, {
    snapshot: "get_bing_wallpaper_snapshot",
    refresh: "refresh_bing_wallpapers",
    preview: "get_bing_wallpaper_preview",
    readPreview: "read_bing_wallpaper_preview",
    releasePreview: "release_bing_wallpaper_preview",
    save: "save_bing_wallpaper_to_downloads",
    apply: "apply_bing_wallpaper",
  });
  assert.deepEqual(calls, [
    [BING_WALLPAPER_COMMANDS.snapshot, undefined],
    [BING_WALLPAPER_COMMANDS.refresh, undefined],
    [BING_WALLPAPER_COMMANDS.preview, { input: { wallpaperId: "wallpaper-1" } }],
    [BING_WALLPAPER_COMMANDS.readPreview, { input: { token: "preview-token" } }],
    [BING_WALLPAPER_COMMANDS.releasePreview, { input: { token: "preview-token" } }],
    [BING_WALLPAPER_COMMANDS.save, { input: { wallpaperId: "wallpaper-1" } }],
    [BING_WALLPAPER_COMMANDS.apply, { input: { wallpaperId: "wallpaper-1" } }],
  ]);
});
