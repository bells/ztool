import assert from "node:assert/strict";
import test from "node:test";
import {
  STATUS_BAR_COMMANDS,
  createStatusBarService,
} from "/private/tmp/zero-tests/services/statusBarService.js";

test("defines explicit Rust command names for status bar actions", () => {
  assert.deepEqual(STATUS_BAR_COMMANDS, {
    getSettings: "get_status_bar_settings",
    updateSettings: "update_status_bar_settings",
    getItems: "get_status_bar_items",
    runItemAction: "run_status_bar_item_action",
  });
});

test("invokes status bar commands through the injected bridge", async () => {
  const calls = [];
  const service = createStatusBarService(async (command, payload) => {
    calls.push({ command, payload });
    if (command === STATUS_BAR_COMMANDS.getItems) {
      return [];
    }
    if (command === STATUS_BAR_COMMANDS.runItemAction) {
      return undefined;
    }
    return {
      enabled: true,
      showPluginItemsOnLaunch: true,
      pluginItemsCollapsed: false,
      visiblePluginItems: {},
    };
  });

  await service.getSettings();
  await service.updateSettings({ enabled: false });
  await service.getItems();
  await service.runItemAction({ itemId: "zero.awake.status" });

  assert.deepEqual(calls, [
    {
      command: "get_status_bar_settings",
      payload: undefined,
    },
    {
      command: "update_status_bar_settings",
      payload: { input: { enabled: false } },
    },
    {
      command: "get_status_bar_items",
      payload: undefined,
    },
    {
      command: "run_status_bar_item_action",
      payload: { input: { itemId: "zero.awake.status" } },
    },
  ]);
});
