import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_PREFERENCES_DESTINATION,
  createPreferencesDestinations,
  createPreferencesSettingIndex,
  createRenderedPreferencesSettingIds,
  filterPreferencesSettings,
  pluginIdFromPreferencesDestination,
  preferencesSettingFocusTargetId,
  resolvePreferencesDestination,
  shouldClearPreferencesSearch,
  toolPreferencesDestinationId,
} from "/private/tmp/zero-tests/core/preferences/preferencesNavigation.js";

const messages = {
  "prefs.nav.general": "常规",
  "prefs.nav.statusBar": "状态栏",
  "prefs.nav.shortcuts": "键盘快捷键",
  "prefs.nav.tools": "工具",
  "prefs.nav.extensions": "扩展",
  "prefs.general.description": "Zero 的基础行为",
  "prefs.statusBar.description": "菜单栏中的 Zero 与工具图标",
  "prefs.shortcuts.description": "查看已注册的全局快捷键",
  "prefs.tools.description": "工具的启用与显示位置",
  "prefs.tool.description": "工具设置",
  "prefs.extensions.description": "安装与管理扩展",
  "prefs.launchAtLogin.title": "登录时打开",
  "prefs.launchAtLogin.description": "登录后自动启动 Zero",
  "prefs.language.title": "语言",
  "prefs.language.description": "Zero 界面使用的语言",
  "statusBar.enabled.title": "显示工具图标",
  "statusBar.enabled.description": "在菜单栏显示工具",
  "statusBar.launch.title": "启动时显示",
  "statusBar.launch.description": "恢复菜单栏工具",
  "statusBar.collapsed.title": "折叠工具图标",
  "statusBar.collapsed.description": "只显示 Zero 主图标",
  "statusBar.items.title": "状态栏工具",
  "statusBar.items.description": "选择在菜单栏显示的工具",
  "prefs.shortcuts.snap": "Zero Snap",
  "prefs.shortcuts.launch": "Zero Launch",
  "prefs.shortcuts.readOnly": "快捷键由 Zero 注册，只读显示",
  "prefs.tools.overview": "工具概览",
  "extensions.market.title": "扩展市场",
  "extensions.market.description": "刷新可安装扩展",
  "extensions.local.title": "本地扩展包",
  "extensions.local.description": "验证本地 zplugin",
  "extensions.installed.title": "已安装扩展",
  "extensions.installed.description": "启用、禁用或卸载",
  "extensions.restore.title": "恢复内置工具",
  "extensions.restore.description": "恢复 Zero 自带工具",
  "extensions.diagnostics.title": "诊断",
  "extensions.diagnostics.description": "查看扩展错误",
  "prefs.tool.enabled.title": "启用工具",
  "prefs.tool.enabled.description": "控制工具是否运行",
  "prefs.tool.navigation.title": "在 Zero 中显示",
  "prefs.tool.navigation.description": "控制主界面与托盘导航",
  "prefs.tool.statusBar.title": "在状态栏显示",
  "prefs.tool.statusBar.description": "控制原生菜单栏图标",
  "prefs.tool.shortcut.title": "键盘快捷键",
  "prefs.tool.shortcut.description": "查看工具快捷键",
};

const t = (key) => messages[key] ?? key;
const tools = [
  { id: "zero.snap", title: "Zero Snap", subtitle: "截图、复制与保存" },
  { id: "zero.awake", title: "Zero Awake", subtitle: "保持系统唤醒" },
];

test("creates stable static and dynamic tool destinations", () => {
  const destinations = createPreferencesDestinations(tools, t);

  assert.equal(destinations[0].id, DEFAULT_PREFERENCES_DESTINATION);
  assert.deepEqual(
    destinations.map((destination) => destination.id),
    [
      "general",
      "status-bar",
      "shortcuts",
      "tools",
      "tool:zero.snap",
      "tool:zero.awake",
      "extensions",
    ],
  );
  assert.equal(toolPreferencesDestinationId("zero.snap"), "tool:zero.snap");
  assert.equal(pluginIdFromPreferencesDestination("tool:zero.snap"), "zero.snap");
  assert.equal(pluginIdFromPreferencesDestination("tools"), null);
});

test("resolves missing destinations to General and preserves available destinations", () => {
  const destinations = createPreferencesDestinations(tools, t);

  assert.equal(resolvePreferencesDestination("tool:missing", destinations), "general");
  assert.equal(resolvePreferencesDestination("status-bar", destinations), "status-bar");
});

test("searches localized titles, descriptions, categories, and tool names", () => {
  const destinations = createPreferencesDestinations(tools, t);
  const index = createPreferencesSettingIndex({ destinations, tools, t });

  assert.equal(filterPreferencesSettings(index, "登录")[0].id, "general.open-at-login");
  assert.ok(
    filterPreferencesSettings(index, "Zero Awake").some(
      (result) => result.destinationId === "tool:zero.awake",
    ),
  );
  assert.ok(
    filterPreferencesSettings(index, "菜单栏").some(
      (result) => result.destinationId === "status-bar",
    ),
  );
  assert.deepEqual(filterPreferencesSettings(index, "does-not-exist"), []);
  assert.deepEqual(filterPreferencesSettings(index, ""), []);
});

test("creates stable focus targets and recognizes Escape clearing", () => {
  assert.equal(
    preferencesSettingFocusTargetId("tool.zero.snap.status-bar"),
    "preference-setting-tool-zero-snap-status-bar",
  );
  assert.equal(shouldClearPreferencesSearch("Escape"), true);
  assert.equal(shouldClearPreferencesSearch("Enter"), false);
});

test("every searchable setting resolves to a rendered target in both languages", () => {
  for (const translate of [t, (key) => `English ${key}`]) {
    const destinations = createPreferencesDestinations(tools, translate);
    const index = createPreferencesSettingIndex({ destinations, tools, t: translate });
    const renderedIds = new Set(createRenderedPreferencesSettingIds(tools));

    assert.deepEqual(new Set(index.map((setting) => setting.id)), renderedIds);
    for (const setting of index) {
      assert.ok(destinations.some((destination) => destination.id === setting.destinationId));
      assert.equal(
        setting.focusTargetId,
        preferencesSettingFocusTargetId(setting.id),
      );
    }
  }
});
