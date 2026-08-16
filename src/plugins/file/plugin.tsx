import { FIRST_PARTY_PLUGIN_IDS } from "../../brand/identity";
import type { BundledPluginModule } from "../../core/pluginHost/pluginModule";
import { FilePanel } from "./FilePanel";
import { createFileTranslator, fileMessages } from "./i18n";

export const filePlugin: BundledPluginModule = {
  kind: "file",
  accentClass: "accent-file",
  manifest: {
    name: FIRST_PARTY_PLUGIN_IDS.file,
    id: FIRST_PARTY_PLUGIN_IDS.file,
    version: "1.0.0",
    author: "bells",
    main: "plugins/file",
    permissions: [],
    displayName: "Zero File",
    description: "Convert PDF and Word files with detected local providers",
    platforms: ["macos", "windows"],
    runtime: "webview",
    contributes: {
      views: [{ id: "zero.file.main", title: "Zero File", surface: "main" }],
    },
  },
  presentation: {
    "zh-CN": {
      title: fileMessages["zh-CN"]["plugin.title"],
      subtitle: fileMessages["zh-CN"]["plugin.subtitle"],
    },
    "en-US": {
      title: fileMessages["en-US"]["plugin.title"],
      subtitle: fileMessages["en-US"]["plugin.subtitle"],
    },
  },
  renderPanel(language) {
    return <FilePanel t={createFileTranslator(language)} />;
  },
};
