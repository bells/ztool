import type { ComponentType, ReactNode } from "react";
import type { ResolvedLanguage } from "../preferences/i18n";
import type { PluginManifest } from "./contracts";

export type BundledPluginSurface = "capture" | "pin" | "launcher" | "paper";

export type PluginTranslator = (key: string) => string;

export interface PluginPresentation {
  title: string;
  subtitle: string;
}

export interface BundledPluginModule {
  kind: string;
  accentClass: string;
  manifest: PluginManifest;
  presentation: Readonly<Record<ResolvedLanguage, PluginPresentation>>;
  renderPanel(language: ResolvedLanguage): ReactNode;
  surfaces?: Partial<Record<BundledPluginSurface, ComponentType>>;
}

export interface RuntimeExtensionModule {
  trust: "sandboxed-runtime";
  manifest: PluginManifest;
}

export type RegisteredPluginModule =
  | ({ trust: "trusted-bundled" } & BundledPluginModule)
  | RuntimeExtensionModule;
