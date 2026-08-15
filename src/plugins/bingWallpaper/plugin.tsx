import { FIRST_PARTY_PLUGIN_IDS } from "../../brand/identity";
import type { BundledPluginModule } from "../../core/pluginHost/pluginModule";
import { BingWallpaperPanel } from "./BingWallpaperPanel";
import {
  bingWallpaperMessages,
  createBingWallpaperTranslator,
} from "./i18n";
import PaperApp from "./PaperApp";

export const bingWallpaperPlugin: BundledPluginModule = {
  kind: "bing-wallpaper",
  accentClass: "accent-bing-wallpaper",
  manifest: {
    name: FIRST_PARTY_PLUGIN_IDS.paper,
    id: FIRST_PARTY_PLUGIN_IDS.paper,
    version: "1.0.0",
    author: "bells",
    main: "plugins/bingWallpaper",
    permissions: ["network", "storage.plugin", "system.wallpaper"],
    displayName: "Zero Paper",
    description: "Browse, download, and apply Bing daily wallpapers",
    platforms: ["macos", "windows", "linux"],
    runtime: "webview",
    contributes: {
      views: [{ id: "zero.paper.main", title: "Zero Paper", surface: "main" }],
      commands: [
        { id: "zero.paper.refresh", title: "Refresh Bing wallpapers" },
        { id: "zero.paper.apply", title: "Apply Bing wallpaper" },
        { id: "zero.paper.download", title: "Download Bing wallpaper" },
      ],
      statusBarItems: [
        {
          id: "zero.paper.status",
          title: "Zero Paper",
          icon: "paper",
          action: { type: "open-plugin" },
          order: 30,
          visibleByDefault: true,
        },
      ],
    },
  },
  presentation: {
    "zh-CN": {
      title: bingWallpaperMessages["zh-CN"]["plugin.title"],
      subtitle: bingWallpaperMessages["zh-CN"]["plugin.subtitle"],
    },
    "en-US": {
      title: bingWallpaperMessages["en-US"]["plugin.title"],
      subtitle: bingWallpaperMessages["en-US"]["plugin.subtitle"],
    },
  },
  renderPanel(language) {
    return <BingWallpaperPanel t={createBingWallpaperTranslator(language)} />;
  },
  surfaces: { paper: PaperApp },
};
