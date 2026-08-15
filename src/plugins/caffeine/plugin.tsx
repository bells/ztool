import type { BundledPluginModule } from "../../core/pluginHost/pluginModule";
import { FIRST_PARTY_PLUGIN_IDS } from "../../brand/identity";
import { CaffeinePanel } from "./CaffeinePanel";
import { caffeineMessages, createCaffeineTranslator } from "./i18n";

export const caffeinePlugin: BundledPluginModule = {
  kind: "caffeine",
  accentClass: "accent-caffeine",
  manifest: {
    name: FIRST_PARTY_PLUGIN_IDS.awake,
    id: FIRST_PARTY_PLUGIN_IDS.awake,
    version: "0.1.0",
    author: "watson",
    main: "plugins/caffeine",
    permissions: ["ui.message"],
    displayName: "Zero Awake",
    description: "Keep display and system awake",
    platforms: ["macos", "windows", "linux"],
    runtime: "webview",
    contributes: {
      views: [{ id: "zero.awake.main", title: "Zero Awake", surface: "main" }],
      commands: [{ id: "zero.awake.toggle", title: "Toggle Zero Awake" }],
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
          action: { type: "toggle-caffeine", commandId: "zero.awake.toggle" },
          order: 10,
          visibleByDefault: true,
        },
      ],
    },
  },
  presentation: {
    "zh-CN": {
      title: caffeineMessages["zh-CN"]["plugin.title"],
      subtitle: caffeineMessages["zh-CN"]["plugin.subtitle"],
    },
    "en-US": {
      title: caffeineMessages["en-US"]["plugin.title"],
      subtitle: caffeineMessages["en-US"]["plugin.subtitle"],
    },
  },
  renderPanel(language) {
    return <CaffeinePanel t={createCaffeineTranslator(language)} />;
  },
};
