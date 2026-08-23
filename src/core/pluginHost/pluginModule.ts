import type { ComponentType } from "react";
import type { ResolvedLanguage } from "../preferences/i18n";
import type { PluginManifest } from "./contracts";

export type BundledPluginSurface = "capture" | "pin" | "launcher" | "paper" | "snap-menu";

export type PluginTranslator = (key: string) => string;

export interface PluginPresentation {
  title: string;
  subtitle: string;
}

export interface BundledPluginPanelProps {
  language: ResolvedLanguage;
}

export type BundledPluginLoader<Props = Record<string, never>> = () => Promise<{
  default: ComponentType<Props>;
}>;

export interface BundledPluginModule {
  kind: string;
  accentClass: string;
  manifest: PluginManifest;
  presentation: Readonly<Record<ResolvedLanguage, PluginPresentation>>;
  loadPanel: BundledPluginLoader<BundledPluginPanelProps>;
  surfaces?: Partial<Record<BundledPluginSurface, BundledPluginLoader>>;
}

export interface RuntimeExtensionModule {
  trust: "sandboxed-runtime";
  manifest: PluginManifest;
}

export type RegisteredPluginModule =
  | ({ trust: "trusted-bundled" } & BundledPluginModule)
  | RuntimeExtensionModule;
