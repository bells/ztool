import assert from "node:assert/strict";
import test from "node:test";
import {
  applyStatusBarSettingsUpdate,
  createStatusBarUiState,
  statusBarPluginVisibilityInput,
} from "/private/tmp/zero-tests/services/statusBarController.js";

function pluginRecord(name, enabled = true, contributes = undefined, health = undefined) {
  return {
    name,
    version: "0.1.0",
    author: "watson",
    source: "bundled",
    enabled,
    health: health ?? (enabled ? "ready" : "disabled"),
    manifest: {
      name,
      version: "0.1.0",
      author: "watson",
      main: `plugins/${name}`,
      permissions: ["ui.message"],
      displayName: name === "zero.snap" ? "Screenshot" : "Caffeine",
      description: "Plugin description",
      contributes,
    },
    approvedPermissions: ["ui.message"],
  };
}

const screenshotStatusItem = {
  id: "zero.snap.status",
  title: "Zero Snap",
  icon: "screenshot",
  action: { type: "start-screenshot" },
  order: 20,
  visibleByDefault: true,
};

const caffeineStatusItem = {
  id: "zero.awake.status",
  title: "Zero Awake",
  icon: "caffeine-empty",
  activeIcon: "caffeine-full",
  action: { type: "toggle-caffeine" },
  order: 10,
  visibleByDefault: true,
};

test("applies status bar setting updates without dropping existing item visibility", () => {
  const settings = {
    enabled: true,
    showPluginItemsOnLaunch: true,
    pluginItemsCollapsed: false,
    visiblePluginItems: {
      "zero.snap": true,
      "zero.awake": true,
    },
  };

  assert.deepEqual(
    applyStatusBarSettingsUpdate(settings, {
      enabled: false,
      pluginItemsCollapsed: true,
      visiblePluginItems: {
        "zero.awake": false,
      },
    }),
    {
      enabled: false,
      showPluginItemsOnLaunch: true,
      pluginItemsCollapsed: true,
      visiblePluginItems: {
        "zero.snap": true,
        "zero.awake": false,
      },
    },
  );
  assert.deepEqual(statusBarPluginVisibilityInput("zero.snap", false), {
    visiblePluginItems: {
      "zero.snap": false,
    },
  });
});

test("creates preference, preview, fallback, and error state for the status bar UI", () => {
  const records = [
    pluginRecord("zero.snap", true, { statusBarItems: [screenshotStatusItem] }),
    pluginRecord("zero.awake", true, { statusBarItems: [caffeineStatusItem] }),
  ];
  const settings = {
    enabled: true,
    showPluginItemsOnLaunch: true,
    pluginItemsCollapsed: false,
    visiblePluginItems: {
      "zero.snap": false,
      "zero.awake": true,
    },
  };
  const items = [
    {
      id: "zero.primary",
      pluginName: null,
      title: "Zero",
      icon: "zero",
      baseIcon: "zero",
      action: { type: "toggle-tray" },
      order: 0,
      nativeVisible: true,
    },
    {
      id: "zero.awake.status",
      pluginName: "zero.awake",
      title: "Zero Awake",
      icon: "caffeine-empty",
      baseIcon: "caffeine-empty",
      activeIcon: "caffeine-full",
      action: { type: "toggle-caffeine" },
      order: 10,
      nativeVisible: false,
    },
  ];

  const uiState = createStatusBarUiState({
    records,
    settings,
    items,
    isLoading: false,
    isBusy: false,
    error: "Cannot save status bar settings",
  });

  assert.deepEqual(uiState.previewItems.map((item) => item.id), [
    "zero.primary",
    "zero.awake.status",
  ]);
  assert.deepEqual(uiState.preferenceItems.map((item) => ({
    id: item.id,
    visible: item.visible,
  })), [
    {
      id: "zero.awake.status",
      visible: true,
    },
    {
      id: "zero.snap.status",
      visible: false,
    },
  ]);
  assert.deepEqual(uiState.fallbackItems.map((item) => item.id), [
    "zero.awake.status",
  ]);
  assert.equal(uiState.messageKey, "statusBar.message.error");
  assert.equal(uiState.messageDetail, "Cannot save status bar settings");
});

test("collapsed status bar settings keep only the primary item in the preview", () => {
  const records = [
    pluginRecord("zero.snap", true, { statusBarItems: [screenshotStatusItem] }),
  ];
  const items = [
    {
      id: "zero.primary",
      pluginName: null,
      title: "Zero",
      icon: "zero",
      baseIcon: "zero",
      action: { type: "toggle-tray" },
      order: 0,
      nativeVisible: true,
    },
    {
      id: "zero.snap.status",
      pluginName: "zero.snap",
      title: "Zero Snap",
      icon: "screenshot",
      baseIcon: "screenshot",
      action: { type: "start-screenshot" },
      order: 20,
      nativeVisible: true,
    },
  ];
  const uiState = createStatusBarUiState({
    records,
    settings: {
      enabled: true,
      showPluginItemsOnLaunch: true,
      pluginItemsCollapsed: true,
      visiblePluginItems: { "zero.snap": true },
    },
    items,
    isLoading: false,
    isBusy: false,
    error: null,
  });

  assert.deepEqual(uiState.previewItems.map((item) => item.id), ["zero.primary"]);
  assert.deepEqual(uiState.preferenceItems.map((item) => item.id), ["zero.snap.status"]);
});
