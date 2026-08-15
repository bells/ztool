import {
  createMessageTranslator,
  type LocalizedMessages,
  type ResolvedLanguage,
} from "../../core/preferences/i18n.js";

export const bingWallpaperMessages = {
  "zh-CN": {
    "plugin.title": "Zero Paper",
    "plugin.subtitle": "浏览、下载并应用每日壁纸",
    "wallpaper.title": "Zero Paper",
    "wallpaper.actions": "壁纸操作",
    "wallpaper.download": "保存到下载目录",
    "wallpaper.apply": "设为桌面壁纸",
    "wallpaper.applyPreview": "将当前图片设为桌面壁纸",
    "wallpaper.older": "查看更早的壁纸",
    "wallpaper.newer": "查看更新的壁纸",
    "wallpaper.loading": "正在读取壁纸缓存",
    "wallpaper.refreshing": "更新中",
    "wallpaper.stale": "离线缓存",
    "wallpaper.previewLoading": "正在载入图片",
    "wallpaper.previewUnavailable": "图片暂不可用",
    "wallpaper.applying": "正在应用",
    "wallpaper.empty": "暂时没有可用壁纸",
    "wallpaper.retry": "重试",
    "wallpaper.fallbackTitle": "Bing 每日壁纸",
    "wallpaper.attributionFallback": "Bing 每日壁纸",
    "wallpaper.applied": "已设为桌面壁纸",
    "wallpaper.saved": "已保存到下载目录",
    "wallpaper.platformUnsupported": "当前桌面环境不支持自动设置壁纸，但仍可浏览和下载。",
  },
  "en-US": {
    "plugin.title": "Zero Paper",
    "plugin.subtitle": "Browse, download, and apply daily wallpapers",
    "wallpaper.title": "Zero Paper",
    "wallpaper.actions": "Wallpaper actions",
    "wallpaper.download": "Save to Downloads",
    "wallpaper.apply": "Set as desktop wallpaper",
    "wallpaper.applyPreview": "Set this image as desktop wallpaper",
    "wallpaper.older": "View an older wallpaper",
    "wallpaper.newer": "View a newer wallpaper",
    "wallpaper.loading": "Loading wallpaper cache",
    "wallpaper.refreshing": "Refreshing",
    "wallpaper.stale": "Offline cache",
    "wallpaper.previewLoading": "Loading image",
    "wallpaper.previewUnavailable": "Image unavailable",
    "wallpaper.applying": "Applying",
    "wallpaper.empty": "No wallpaper is available yet",
    "wallpaper.retry": "Retry",
    "wallpaper.fallbackTitle": "Bing daily wallpaper",
    "wallpaper.attributionFallback": "Bing daily wallpaper",
    "wallpaper.applied": "Desktop wallpaper updated",
    "wallpaper.saved": "Saved to Downloads",
    "wallpaper.platformUnsupported": "This desktop environment cannot set wallpaper automatically, but browsing and downloads remain available.",
  },
} as const satisfies LocalizedMessages;

export type TranslationKey = keyof (typeof bingWallpaperMessages)["zh-CN"];

export function createBingWallpaperTranslator(language: ResolvedLanguage) {
  return createMessageTranslator(language, bingWallpaperMessages);
}
