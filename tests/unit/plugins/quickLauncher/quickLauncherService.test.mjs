import assert from "node:assert/strict";
import test from "node:test";
import {
  buildActivateInput,
  buildIconInput,
  buildSearchInput,
  createQuickLauncherService,
  QUICK_LAUNCHER_COMMANDS,
} from "/private/tmp/zero-tests/plugins/quickLauncher/quickLauncherServiceCore.js";

test("quick launcher payload builders expose only documented fields", () => {
  assert.deepEqual(buildSearchInput("wx", 24), { query: "wx", limit: 24 });
  assert.deepEqual(buildIconInput("app:macos:1"), { itemId: "app:macos:1" });
  assert.deepEqual(buildIconInput("app:macos:1", "icon-1"), {
    itemId: "app:macos:1",
    iconKey: "icon-1",
  });
  assert.deepEqual(buildActivateInput("setting:macos:1", 3), {
    itemId: "setting:macos:1",
    revision: 3,
  });
});

test("quick launcher service invokes stable command names and camelCase inputs", async () => {
  const calls = [];
  const service = createQuickLauncherService(async (command, args) => {
    calls.push([command, args]);
    return {};
  });
  await service.getSnapshot();
  await service.refresh();
  await service.search("ps", 12);
  await service.getIcon("app:windows:2", "key");
  await service.getIcons([{ itemId: "app:windows:2", iconKey: "key" }]);
  await service.refreshRunning();
  await service.activate("app:windows:2", 9);
  await service.showWindow();
  await service.hideWindow();

  assert.deepEqual(calls, [
    [QUICK_LAUNCHER_COMMANDS.snapshot, undefined],
    [QUICK_LAUNCHER_COMMANDS.refresh, undefined],
    [QUICK_LAUNCHER_COMMANDS.search, { input: { query: "ps", limit: 12 } }],
    [QUICK_LAUNCHER_COMMANDS.icon, { input: { itemId: "app:windows:2", iconKey: "key" } }],
    [QUICK_LAUNCHER_COMMANDS.icons, {
      input: { items: [{ itemId: "app:windows:2", iconKey: "key" }] },
    }],
    [QUICK_LAUNCHER_COMMANDS.refreshRunning, undefined],
    [QUICK_LAUNCHER_COMMANDS.activate, { input: { itemId: "app:windows:2", revision: 9 } }],
    [QUICK_LAUNCHER_COMMANDS.showWindow, undefined],
    [QUICK_LAUNCHER_COMMANDS.hideWindow, undefined],
  ]);
});
