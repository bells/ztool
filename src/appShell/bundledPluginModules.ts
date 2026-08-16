import type { ComponentType } from "react";
import { createExtensionLauncherHostApis } from "../core/pluginHost/extensionLauncherHost";
import type {
  BundledPluginModule,
  BundledPluginSurface,
} from "../core/pluginHost/pluginModule";
import { createBundledPluginRegistry } from "../core/pluginHost/pluginModuleRegistry";
import type { ResolvedLanguage } from "../core/preferences/i18n";
import { bingWallpaperPlugin } from "../plugins/bingWallpaper/plugin";
import { caffeinePlugin } from "../plugins/caffeine/plugin";
import { filePlugin } from "../plugins/file/plugin";
import { quickLauncherPlugin } from "../plugins/quickLauncher/plugin";
import { quickLauncherService } from "../plugins/quickLauncher/quickLauncherService";
import { screenshotPlugin } from "../plugins/screenshot/plugin";

export const BUNDLED_PLUGIN_MODULES = [
  screenshotPlugin,
  caffeinePlugin,
  bingWallpaperPlugin,
  quickLauncherPlugin,
  filePlugin,
] as const satisfies readonly BundledPluginModule[];

export const bundledPluginRegistry = createBundledPluginRegistry(
  BUNDLED_PLUGIN_MODULES,
);

export const BUNDLED_PLUGIN_MANIFESTS = bundledPluginRegistry.manifests;

export const launcherExtensionHostApis = createExtensionLauncherHostApis(
  quickLauncherService,
);

export function bundledPluginModule(pluginId: string) {
  return bundledPluginRegistry.find(pluginId);
}

export function bundledPluginKind(pluginId: string) {
  return bundledPluginModule(pluginId)?.kind ?? null;
}

export function pluginAccentClass(pluginId: string) {
  return bundledPluginModule(pluginId)?.accentClass ?? "accent-extension";
}

export function bundledPluginPresentation(
  pluginId: string,
  language: ResolvedLanguage,
) {
  return bundledPluginModule(pluginId)?.presentation[language];
}

export function renderBundledPluginPanel(
  pluginId: string,
  language: ResolvedLanguage,
) {
  return bundledPluginModule(pluginId)?.renderPanel(language);
}

export function bundledPluginSurface(
  surface: BundledPluginSurface,
): ComponentType | undefined {
  return BUNDLED_PLUGIN_MODULES.find((module) => module.surfaces?.[surface])
    ?.surfaces?.[surface];
}
