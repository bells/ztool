import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_STATUS_BAR_SETTINGS,
  createStatusBarPreview,
  getStatusBarFallbackItems,
  normalizeStatusBarSettings,
  resolveStatusBarPreferenceItems,
  resolveStatusBarItems,
} from "/private/tmp/zero-status-bar-test/services/statusBarModel.js";

function pluginRecord(name, enabled = true, contributes = undefined, health = undefined) {
  return {
    name,
    version: "0.1.0",
    author: "watson",
    source: name.startsWith("zero.") ? "bundled" : "market",
    enabled,
    health: health ?? (enabled ? "ready" : "disabled"),
    manifest: {
      name,
      version: "0.1.0",
      author: "watson",
      main: `plugins/${name}`,
      permissions: ["ui.message"],
      displayName: name === "zero.snap"
        ? "Screenshot"
        : name === "zero.awake"
          ? "Caffeine"
          : "Market Tool",
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

const marketStatusItem = {
  id: "market-tool.status",
  title: "Market Tool",
  icon: "extension",
  action: { type: "open-plugin" },
  order: 100,
  visibleByDefault: true,
};

test("normalizes missing status bar settings to native startup defaults", () => {
  assert.deepEqual(
    normalizeStatusBarSettings(undefined, [
      pluginRecord("zero.snap"),
      pluginRecord("zero.awake"),
    ]),
    {
      enabled: true,
      showPluginItemsOnLaunch: true,
      pluginItemsCollapsed: false,
      visiblePluginItems: {
        "zero.snap": true,
        "zero.awake": true,
      },
    },
  );

  assert.equal(DEFAULT_STATUS_BAR_SETTINGS.enabled, true);
  assert.equal(DEFAULT_STATUS_BAR_SETTINGS.pluginItemsCollapsed, false);
});

test("normalizes and preserves persisted tool-item collapse state", () => {
  const records = [pluginRecord("zero.snap")];

  assert.equal(
    normalizeStatusBarSettings({ pluginItemsCollapsed: true }, records)
      .pluginItemsCollapsed,
    true,
  );
  assert.equal(
    normalizeStatusBarSettings(
      {
        visiblePluginItems: { "zero.snap": false },
      },
      records,
    ).pluginItemsCollapsed,
    false,
  );
});

test("normalizes legacy first-party visibility without changing third-party ids", () => {
  const records = [
    pluginRecord("zero.snap"),
    pluginRecord("zero.awake"),
    pluginRecord("ztool.third-party"),
  ];
  const settings = normalizeStatusBarSettings(
    {
      visiblePluginItems: {
        "zero.snap": false,
        "ztool.screenshot": true,
        "ztool.caffeine": false,
        "ztool.third-party": false,
      },
    },
    records,
  );

  assert.deepEqual(settings.visiblePluginItems, {
    "zero.snap": false,
    "zero.awake": false,
    "ztool.third-party": false,
  });
});

test("resolves primary item plus enabled visible plugin items in deterministic order", () => {
  const records = [
    pluginRecord("zero.snap", true, { statusBarItems: [screenshotStatusItem] }),
    pluginRecord("zero.awake", true, { statusBarItems: [caffeineStatusItem] }),
    pluginRecord("market-tool", true, { statusBarItems: [marketStatusItem] }),
  ];
  const settings = normalizeStatusBarSettings(
    {
      visiblePluginItems: {
        "market-tool": false,
      },
    },
    records,
  );

  assert.deepEqual(
    resolveStatusBarItems({
      records,
      settings,
      caffeineEnabled: true,
      platformSupportsNativeMultiItem: true,
    }).map((item) => ({
      id: item.id,
      pluginName: item.pluginName,
      icon: item.icon,
      actionType: item.action.type,
      nativeVisible: item.nativeVisible,
    })),
    [
      {
        id: "zero.primary",
        pluginName: null,
        icon: "zero",
        actionType: "toggle-tray",
        nativeVisible: true,
      },
      {
        id: "zero.awake.status",
        pluginName: "zero.awake",
        icon: "caffeine-full",
        actionType: "toggle-caffeine",
        nativeVisible: true,
      },
      {
        id: "zero.snap.status",
        pluginName: "zero.snap",
        icon: "screenshot",
        actionType: "start-screenshot",
        nativeVisible: true,
      },
    ],
  );
});

test("omits disabled plugins and keeps primary item recoverable", () => {
  const records = [
    pluginRecord("zero.snap", false, { statusBarItems: [screenshotStatusItem] }),
    pluginRecord("zero.awake", true, { statusBarItems: [caffeineStatusItem] }),
  ];
  const settings = normalizeStatusBarSettings(
    {
      enabled: false,
      visiblePluginItems: {
        "zero.awake": false,
      },
    },
    records,
  );

  assert.deepEqual(
    resolveStatusBarItems({
      records,
      settings,
      caffeineEnabled: false,
      platformSupportsNativeMultiItem: true,
    }).map((item) => item.id),
    ["zero.primary"],
  );
});

test("preview and fallback action row share the same filtered plugin items", () => {
  const records = [
    pluginRecord("zero.snap", true, { statusBarItems: [screenshotStatusItem] }),
    pluginRecord("zero.awake", true, { statusBarItems: [caffeineStatusItem] }),
  ];
  const settings = normalizeStatusBarSettings(
    {
      visiblePluginItems: {
        "zero.snap": false,
      },
    },
    records,
  );
  const items = resolveStatusBarItems({
    records,
    settings,
    caffeineEnabled: false,
    platformSupportsNativeMultiItem: false,
  });

  assert.deepEqual(createStatusBarPreview(items).map((item) => item.id), [
    "zero.primary",
    "zero.awake.status",
  ]);
  assert.deepEqual(getStatusBarFallbackItems(items).map((item) => item.id), [
    "zero.awake.status",
  ]);
  assert.equal(items.find((item) => item.id === "zero.awake.status").nativeVisible, false);
});

test("fallback action row ignores macOS tool-item collapse state", () => {
  const records = [
    pluginRecord("zero.snap", true, { statusBarItems: [screenshotStatusItem] }),
    pluginRecord("zero.awake", true, { statusBarItems: [caffeineStatusItem] }),
  ];
  const resolveFallbackIds = (pluginItemsCollapsed) => {
    const settings = normalizeStatusBarSettings(
      { pluginItemsCollapsed },
      records,
    );
    return getStatusBarFallbackItems(
      resolveStatusBarItems({
        records,
        settings,
        caffeineEnabled: false,
        platformSupportsNativeMultiItem: false,
      }),
    ).map((item) => item.id);
  };

  assert.deepEqual(resolveFallbackIds(false), [
    "zero.awake.status",
    "zero.snap.status",
  ]);
  assert.deepEqual(resolveFallbackIds(true), [
    "zero.awake.status",
    "zero.snap.status",
  ]);
});

test("preference rows include hidden enabled plugin items so users can restore them", () => {
  const records = [
    pluginRecord("zero.snap", true, { statusBarItems: [screenshotStatusItem] }),
    pluginRecord("zero.awake", true, { statusBarItems: [caffeineStatusItem] }),
  ];
  const settings = normalizeStatusBarSettings(
    {
      visiblePluginItems: {
        "zero.awake": false,
      },
    },
    records,
  );

  assert.deepEqual(
    resolveStatusBarPreferenceItems({ records, settings }).map((item) => ({
      id: item.id,
      pluginName: item.pluginName,
      icon: item.icon,
      visible: item.visible,
      disabled: item.disabled,
    })),
    [
      {
        id: "zero.awake.status",
        pluginName: "zero.awake",
        icon: "caffeine-empty",
        visible: false,
        disabled: false,
      },
      {
        id: "zero.snap.status",
        pluginName: "zero.snap",
        icon: "screenshot",
        visible: true,
        disabled: false,
      },
    ],
  );
});

test("fallback action row omits plugin items already visible as native status items", () => {
  const nativeItems = [
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

  assert.deepEqual(getStatusBarFallbackItems(nativeItems), []);
});
