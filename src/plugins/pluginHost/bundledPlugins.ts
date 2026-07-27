import type { PluginManifest } from "./contracts";
import {
  FIRST_PARTY_PLUGIN_IDS,
  canonicalFirstPartyPluginId,
} from "../../brand/identity.js";

export type BuiltinPluginKind = "screenshot" | "caffeine" | "bing-wallpaper" | "quick-launcher";

export const BUNDLED_PLUGIN_MANIFESTS: PluginManifest[] = [
  {
    name: FIRST_PARTY_PLUGIN_IDS.snap,
    version: "0.1.0",
    author: "watson",
    main: "plugins/screenshot",
    permissions: ["ui.message"],
    id: FIRST_PARTY_PLUGIN_IDS.snap,
    displayName: "Zero Snap",
    description: "Shortcut, copy, save",
    platforms: ["macos", "windows", "linux"],
    runtime: "webview",
    contributes: {
      views: [
        {
          id: "zero.snap.main",
          title: "Zero Snap",
          surface: "main",
        },
      ],
      commands: [
        {
          id: "zero.snap.capture",
          title: "Capture Zero Snap",
        },
        {
          id: "zero.snap.copy",
          title: "Capture and Copy",
        },
        {
          id: "zero.snap.save",
          title: "Capture and Save",
        },
      ],
      statusBarItems: [
        {
          id: "zero.snap.status",
          title: "Zero Snap",
          icon: "screenshot",
          action: {
            type: "start-screenshot",
            commandId: "zero.snap.capture",
          },
          order: 20,
          visibleByDefault: true,
        },
      ],
    },
  },
  {
    name: FIRST_PARTY_PLUGIN_IDS.awake,
    version: "0.1.0",
    author: "watson",
    main: "plugins/caffeine",
    permissions: ["ui.message"],
    id: FIRST_PARTY_PLUGIN_IDS.awake,
    displayName: "Zero Awake",
    description: "Keep display and system awake",
    platforms: ["macos", "windows", "linux"],
    runtime: "webview",
    contributes: {
      views: [
        {
          id: "zero.awake.main",
          title: "Zero Awake",
          surface: "main",
        },
      ],
      commands: [
        {
          id: "zero.awake.toggle",
          title: "Toggle Zero Awake",
        },
      ],
      settings: [
        {
          key: "durationMinutes",
          type: "number",
          default: 0,
          label: "Duration minutes",
        },
      ],
      statusBarItems: [
        {
          id: "zero.awake.status",
          title: "Zero Awake",
          icon: "caffeine-empty",
          activeIcon: "caffeine-full",
          action: {
            type: "toggle-caffeine",
            commandId: "zero.awake.toggle",
          },
          order: 10,
          visibleByDefault: true,
        },
      ],
    },
  },
  {
    name: FIRST_PARTY_PLUGIN_IDS.paper,
    version: "1.0.0",
    author: "bells",
    main: "plugins/bingWallpaper",
    permissions: ["network", "storage.plugin", "system.wallpaper"],
    id: FIRST_PARTY_PLUGIN_IDS.paper,
    displayName: "Zero Paper",
    description: "Browse, download, and apply Bing daily wallpapers",
    platforms: ["macos", "windows", "linux"],
    runtime: "webview",
    contributes: {
      views: [
        {
          id: "zero.paper.main",
          title: "Zero Paper",
          surface: "main",
        },
      ],
      commands: [
        {
          id: "zero.paper.refresh",
          title: "Refresh Bing wallpapers",
        },
        {
          id: "zero.paper.apply",
          title: "Apply Bing wallpaper",
        },
        {
          id: "zero.paper.download",
          title: "Download Bing wallpaper",
        },
      ],
      statusBarItems: [
        {
          id: "zero.paper.status",
          title: "Zero Paper",
          icon: "paper",
          action: {
            type: "open-plugin",
          },
          order: 30,
          visibleByDefault: true,
        },
      ],
    },
  },
  {
    name: FIRST_PARTY_PLUGIN_IDS.launch,
    version: "1.0.0",
    author: "bells",
    main: "plugins/quickLauncher",
    permissions: [
      "system.apps.read",
      "system.apps.execute",
      "system.window.focus",
      "system.settings.open",
    ],
    id: FIRST_PARTY_PLUGIN_IDS.launch,
    displayName: "Zero Launch",
    description: "Search, launch, and switch local apps and system settings",
    platforms: ["macos", "windows"],
    runtime: "webview",
    contributes: {
      views: [
        {
          id: "zero.launch.main",
          title: "Zero Launch",
          surface: "main",
        },
      ],
      commands: [
        {
          id: "zero.launch.show",
          title: "Show Zero Launch",
        },
        {
          id: "zero.launch.refresh",
          title: "Refresh application index",
        },
      ],
      statusBarItems: [
        {
          id: "zero.launch.status",
          title: "Zero Launch",
          icon: "launch",
          action: {
            type: "open-plugin",
          },
          order: 40,
          visibleByDefault: true,
        },
      ],
    },
  },
];

export function bundledPluginKind(pluginId: string): BuiltinPluginKind | null {
  const canonicalId = canonicalFirstPartyPluginId(pluginId);

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.snap || pluginId === "screenshot") {
    return "screenshot";
  }

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.awake || pluginId === "caffeine") {
    return "caffeine";
  }

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.paper || pluginId === "bing-wallpaper") {
    return "bing-wallpaper";
  }

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.launch || pluginId === "quick-launcher") {
    return "quick-launcher";
  }

  return null;
}

export function pluginAccentClass(pluginId: string) {
  const builtin = bundledPluginKind(pluginId);
  return builtin ? `accent-${builtin}` : "accent-extension";
}
