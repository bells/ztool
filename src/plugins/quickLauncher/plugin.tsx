import { FIRST_PARTY_PLUGIN_IDS } from "../../brand/identity";
import type { BundledPluginModule } from "../../core/pluginHost/pluginModule";
import {
  createQuickLauncherTranslator,
  quickLauncherMessages,
} from "./i18n";
import QuickLauncherApp from "./QuickLauncherApp";
import { QuickLauncherPanel } from "./QuickLauncherPanel";

export const quickLauncherPlugin: BundledPluginModule = {
  kind: "quick-launcher",
  accentClass: "accent-quick-launcher",
  manifest: {
    name: FIRST_PARTY_PLUGIN_IDS.launch,
    id: FIRST_PARTY_PLUGIN_IDS.launch,
    version: "1.0.0",
    author: "bells",
    main: "plugins/quickLauncher",
    permissions: [
      "system.apps.read",
      "system.apps.execute",
      "system.window.focus",
      "system.settings.open",
    ],
    displayName: "Zero Launch",
    description: "Search, launch, and switch local apps and system settings",
    platforms: ["macos", "windows"],
    runtime: "webview",
    contributes: {
      views: [{ id: "zero.launch.main", title: "Zero Launch", surface: "main" }],
      commands: [
        { id: "zero.launch.show", title: "Show Zero Launch" },
        { id: "zero.launch.refresh", title: "Refresh application index" },
      ],
      statusBarItems: [
        {
          id: "zero.launch.status",
          title: "Zero Launch",
          icon: "launch",
          action: { type: "open-plugin" },
          order: 40,
          visibleByDefault: true,
        },
      ],
    },
  },
  presentation: {
    "zh-CN": {
      title: quickLauncherMessages["zh-CN"]["plugin.title"],
      subtitle: quickLauncherMessages["zh-CN"]["plugin.subtitle"],
    },
    "en-US": {
      title: quickLauncherMessages["en-US"]["plugin.title"],
      subtitle: quickLauncherMessages["en-US"]["plugin.subtitle"],
    },
  },
  renderPanel(language) {
    return <QuickLauncherPanel t={createQuickLauncherTranslator(language)} />;
  },
  surfaces: { launcher: QuickLauncherApp },
};
