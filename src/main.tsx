import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";
import {
  AboutWindowApp,
  MainWindowApp,
  PreferencesWindowApp,
  TrayPanelApp,
} from "./App";
import { resolveAppSurface } from "./appShell/appSurface";
import { bundledPluginSurface } from "./appShell/bundledPluginModules";
import type { BundledPluginSurface } from "./core/pluginHost/pluginModule";

const label = getCurrentWindow().label;
const surface = resolveAppSurface(label);
const pluginSurface = isBundledPluginSurface(surface)
  ? bundledPluginSurface(surface)
  : undefined;
const RoutedApp =
  pluginSurface ??
  (surface === "main"
        ? MainWindowApp
        : surface === "preferences"
          ? PreferencesWindowApp
          : surface === "about"
            ? AboutWindowApp
            : TrayPanelApp);

function isBundledPluginSurface(
  surface: string,
): surface is BundledPluginSurface {
  return ["capture", "pin", "launcher", "paper"].includes(surface);
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RoutedApp />
  </React.StrictMode>,
);
