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
import CaptureApp from "./plugins/screenshot/capture/CaptureApp";
import PinApp from "./plugins/screenshot/capture/PinApp";
import QuickLauncherApp from "./plugins/quickLauncher/QuickLauncherApp";
import PaperApp from "./plugins/bingWallpaper/PaperApp";

const label = getCurrentWindow().label;
const surface = resolveAppSurface(label);
const RoutedApp =
  surface === "capture"
    ? CaptureApp
    : surface === "launcher"
      ? QuickLauncherApp
      : surface === "paper"
        ? PaperApp
    : surface === "pin"
      ? PinApp
      : surface === "main"
        ? MainWindowApp
        : surface === "preferences"
          ? PreferencesWindowApp
          : surface === "about"
            ? AboutWindowApp
            : TrayPanelApp;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RoutedApp />
  </React.StrictMode>,
);
