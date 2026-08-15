import type { LanguagePreference } from "./preferencesModel";

export type ResolvedLanguage = "zh-CN" | "en-US";
export type MessageBundle = Readonly<Record<string, string>>;
export type LocalizedMessages = Readonly<Record<ResolvedLanguage, MessageBundle>>;
export type Translator = (key: string) => string;

const zh = {
  "app.tagline": "托盘优先的插件工具箱",
  "app.pluginCount": "个工具",
  "nav.preferences": "偏好",
  "nav.about": "关于",
  "nav.quit": "退出",
  "shell.openZero": "打开 Zero",
  "shell.more": "更多",
  "shell.plugins": "工具插件",
  "shell.systemActions": "系统操作",
  "shell.aboutZero": "关于 Zero",
  "shell.exitStatusBar": "退出状态栏",
  "shell.actionError": "操作失败",
  "shell.mainTitle": "Zero 主界面",
  "shell.mainSubtitle": "管理工具、状态和偏好",
  "shell.pluginWorkspace": "插件工作区",
  "shell.preferencesSubtitle": "启动、语言和工具展示",
  "shell.aboutSubtitle": "版本、运行方式和插件信息",
  "prefs.eyebrow": "偏好",
  "prefs.title": "偏好设置",
  "prefs.saved": "已保存",
  "prefs.launchAtLogin.title": "登录时打开",
  "prefs.launchAtLogin.description": "开机登录后自动启动 Zero",
  "prefs.language.title": "语言",
  "prefs.language.description": "默认跟随系统语言",
  "prefs.language.system": "跟随系统",
  "prefs.language.zh": "中文",
  "prefs.language.en": "English",
  "prefs.tools.title": "工具展示",
  "prefs.message.ready": "偏好设置已准备好",
  "prefs.message.autostartReadError": "读取登录启动状态失败",
  "prefs.message.autostartOn": "已设置为登录时打开",
  "prefs.message.autostartOff": "已关闭登录时打开",
  "prefs.message.autostartWriteError": "设置登录启动失败",
  "prefs.message.toolsSaved": "工具展示偏好已保存",
  "prefs.message.languageSaved": "语言偏好已保存",
  "statusBar.title": "状态栏",
  "statusBar.enabled.title": "显示工具子图标",
  "statusBar.enabled.description": "主 Zero 图标保留，后面展示已启用工具",
  "statusBar.launch.title": "启动时显示工具图标",
  "statusBar.launch.description": "启动后立即恢复工具插件状态栏图标",
  "statusBar.preview.title": "排列预览",
  "statusBar.preview.description": "主 Logo 后依次展示可见工具",
  "statusBar.items.title": "状态栏工具",
  "statusBar.items.empty": "没有可展示的工具插件",
  "statusBar.fallback.title": "状态栏快捷工具",
  "statusBar.fallback.description": "当前平台在面板内提供同样的操作入口",
  "statusBar.message.ready": "状态栏设置已准备好",
  "statusBar.message.loading": "正在读取状态栏设置",
  "statusBar.message.saving": "正在保存状态栏设置",
  "statusBar.message.error": "状态栏设置失败",
  "about.eyebrow": "关于",
  "about.title": "关于 Zero",
  "about.descriptionTitle": "一个托盘优先的工具集合应用",
  "about.description": "每个工具都是独立插件，通过统一模块注册并由宿主安全协调。",
  "about.pluginCount": "插件数量",
  "about.runtime": "运行方式",
} as const;

const en: Record<keyof typeof zh, string> = {
  "app.tagline": "Tray-first plugin toolbox",
  "app.pluginCount": "tools",
  "nav.preferences": "Prefs",
  "nav.about": "About",
  "nav.quit": "Quit",
  "shell.openZero": "Open Zero",
  "shell.more": "More",
  "shell.plugins": "Tool plugins",
  "shell.systemActions": "System actions",
  "shell.aboutZero": "About Zero",
  "shell.exitStatusBar": "Exit status bar",
  "shell.actionError": "Action failed",
  "shell.mainTitle": "Zero Home",
  "shell.mainSubtitle": "Manage tools, status, and preferences",
  "shell.pluginWorkspace": "Plugin workspace",
  "shell.preferencesSubtitle": "Startup, language, and visible tools",
  "shell.aboutSubtitle": "Version, runtime, and plugin information",
  "prefs.eyebrow": "Preferences",
  "prefs.title": "Preferences",
  "prefs.saved": "Saved",
  "prefs.launchAtLogin.title": "Open at login",
  "prefs.launchAtLogin.description": "Start Zero after system login",
  "prefs.language.title": "Language",
  "prefs.language.description": "Defaults to the system language",
  "prefs.language.system": "System",
  "prefs.language.zh": "Chinese",
  "prefs.language.en": "English",
  "prefs.tools.title": "Visible tools",
  "prefs.message.ready": "Preferences are ready",
  "prefs.message.autostartReadError": "Failed to read login startup state",
  "prefs.message.autostartOn": "Open at login is enabled",
  "prefs.message.autostartOff": "Open at login is disabled",
  "prefs.message.autostartWriteError": "Failed to update login startup",
  "prefs.message.toolsSaved": "Tool visibility saved",
  "prefs.message.languageSaved": "Language preference saved",
  "statusBar.title": "Status bar",
  "statusBar.enabled.title": "Show tool icons",
  "statusBar.enabled.description": "Keep the main Zero icon, then show enabled tools",
  "statusBar.launch.title": "Show tool icons at launch",
  "statusBar.launch.description": "Restore plugin status bar icons immediately after startup",
  "statusBar.preview.title": "Arrangement preview",
  "statusBar.preview.description": "Visible tools appear after the main logo",
  "statusBar.items.title": "Status bar tools",
  "statusBar.items.empty": "No tool plugin can be shown",
  "statusBar.fallback.title": "Status bar shortcuts",
  "statusBar.fallback.description": "This platform exposes the same actions in the panel",
  "statusBar.message.ready": "Status bar settings are ready",
  "statusBar.message.loading": "Loading status bar settings",
  "statusBar.message.saving": "Saving status bar settings",
  "statusBar.message.error": "Status bar settings failed",
  "about.eyebrow": "About",
  "about.title": "About Zero",
  "about.descriptionTitle": "A tray-first utility collection",
  "about.description": "Every tool is an independent plugin registered through one host-controlled module boundary.",
  "about.pluginCount": "Plugins",
  "about.runtime": "Runtime",
};

const dictionaries: LocalizedMessages = { "zh-CN": zh, "en-US": en };

export type TranslationKey = keyof typeof zh;

export function resolveLanguage(
  preference: LanguagePreference,
  systemLanguage: string,
): ResolvedLanguage {
  if (preference === "zh-CN" || preference === "en-US") {
    return preference;
  }
  return systemLanguage.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

export function createMessageTranslator(
  language: ResolvedLanguage,
  messages: LocalizedMessages,
): Translator {
  return (key) => messages[language][key] ?? key;
}

export function createTranslator(language: ResolvedLanguage): Translator {
  return createMessageTranslator(language, dictionaries);
}
