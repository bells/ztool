import { FIRST_PARTY_PLUGIN_IDS } from "../../brand/identity";
import type { BundledPluginModule } from "../../core/pluginHost/pluginModule";
import { createScreenshotTranslator, screenshotMessages } from "./i18n";
import { ScreenshotPanel } from "./ScreenshotPanel";
import CaptureApp from "./capture/CaptureApp";
import PinApp from "./capture/PinApp";

export const screenshotPlugin: BundledPluginModule = {
  kind: "screenshot",
  accentClass: "accent-screenshot",
  manifest: {
    name: FIRST_PARTY_PLUGIN_IDS.snap,
    id: FIRST_PARTY_PLUGIN_IDS.snap,
    version: "0.1.0",
    author: "watson",
    main: "plugins/screenshot",
    permissions: ["ui.message"],
    displayName: "Zero Snap",
    description: "Shortcut, copy, save",
    platforms: ["macos", "windows", "linux"],
    runtime: "webview",
    contributes: {
      views: [{ id: "zero.snap.main", title: "Zero Snap", surface: "main" }],
      commands: [
        { id: "zero.snap.capture", title: "Capture Zero Snap" },
        { id: "zero.snap.copy", title: "Capture and Copy" },
        { id: "zero.snap.save", title: "Capture and Save" },
      ],
      statusBarItems: [
        {
          id: "zero.snap.status",
          title: "Zero Snap",
          icon: "screenshot",
          action: { type: "start-screenshot", commandId: "zero.snap.capture" },
          order: 20,
          visibleByDefault: true,
        },
      ],
    },
  },
  presentation: {
    "zh-CN": {
      title: screenshotMessages["zh-CN"]["plugin.title"],
      subtitle: screenshotMessages["zh-CN"]["plugin.subtitle"],
    },
    "en-US": {
      title: screenshotMessages["en-US"]["plugin.title"],
      subtitle: screenshotMessages["en-US"]["plugin.subtitle"],
    },
  },
  renderPanel(language) {
    return <ScreenshotPanel t={createScreenshotTranslator(language)} />;
  },
  surfaces: { capture: CaptureApp, pin: PinApp },
};
