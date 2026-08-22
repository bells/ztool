import type {
  BingWallpaperActionInput,
  BingWallpaperActionResult,
  BingWallpaperPreview,
  BingWallpaperPreviewResourceInput,
  BingWallpaperSnapshot,
} from "./contracts.js";

export const BING_WALLPAPER_COMMANDS = {
  snapshot: "get_bing_wallpaper_snapshot",
  refresh: "refresh_bing_wallpapers",
  preview: "get_bing_wallpaper_preview",
  readPreview: "read_bing_wallpaper_preview",
  releasePreview: "release_bing_wallpaper_preview",
  save: "save_bing_wallpaper_to_downloads",
  apply: "apply_bing_wallpaper",
} as const;

export type BingWallpaperCommand =
  (typeof BING_WALLPAPER_COMMANDS)[keyof typeof BING_WALLPAPER_COMMANDS];
export type BingWallpaperInvokeArgs = {
  input: BingWallpaperActionInput | BingWallpaperPreviewResourceInput;
};
export type BingWallpaperInvokeBridge = <T>(
  command: BingWallpaperCommand,
  args?: BingWallpaperInvokeArgs,
) => Promise<T>;

export function createBingWallpaperService(invokeCommand: BingWallpaperInvokeBridge) {
  return {
    snapshot: () =>
      invokeCommand<BingWallpaperSnapshot>(BING_WALLPAPER_COMMANDS.snapshot),
    refresh: () =>
      invokeCommand<BingWallpaperSnapshot>(BING_WALLPAPER_COMMANDS.refresh),
    preview: (input: BingWallpaperActionInput) =>
      invokeCommand<BingWallpaperPreview>(BING_WALLPAPER_COMMANDS.preview, { input }),
    readPreview: (input: BingWallpaperPreviewResourceInput) =>
      invokeCommand<ArrayBuffer>(BING_WALLPAPER_COMMANDS.readPreview, { input }),
    releasePreview: (input: BingWallpaperPreviewResourceInput) =>
      invokeCommand<void>(BING_WALLPAPER_COMMANDS.releasePreview, { input }),
    save: (input: BingWallpaperActionInput) =>
      invokeCommand<BingWallpaperActionResult>(BING_WALLPAPER_COMMANDS.save, { input }),
    apply: (input: BingWallpaperActionInput) =>
      invokeCommand<BingWallpaperActionResult>(BING_WALLPAPER_COMMANDS.apply, { input }),
  };
}

export type BingWallpaperService = ReturnType<typeof createBingWallpaperService>;
