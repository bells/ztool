import assert from "node:assert/strict";
import test from "node:test";
import {
  GLOBAL_SHORTCUT_COMMANDS,
  createGlobalShortcutService,
  shortcutStatusMessageKey,
} from "/private/tmp/zero-tests/services/globalShortcutServiceCore.js";

test("defines and invokes the read-only global shortcut command", async () => {
  const calls = [];
  const snapshots = [
    {
      id: "snapCapture",
      pluginName: "zero.snap",
      accelerator: "CommandOrControl+Shift+A",
      enabled: true,
      registered: true,
      platformSupported: true,
      registrationState: "active",
    },
  ];
  const service = createGlobalShortcutService(async (command) => {
    calls.push(command);
    return snapshots;
  });

  assert.deepEqual(await service.getSnapshots(), snapshots);
  assert.deepEqual(calls, ["get_global_shortcut_snapshots"]);
  assert.deepEqual(GLOBAL_SHORTCUT_COMMANDS, {
    getSnapshots: "get_global_shortcut_snapshots",
  });
});

test("maps every native registration state to localized status copy", () => {
  assert.equal(shortcutStatusMessageKey("active"), "prefs.shortcuts.active");
  assert.equal(shortcutStatusMessageKey("inactive"), "prefs.shortcuts.inactive");
  assert.equal(shortcutStatusMessageKey("conflict"), "prefs.shortcuts.conflict");
  assert.equal(shortcutStatusMessageKey("unsupported"), "prefs.shortcuts.unsupported");
});
