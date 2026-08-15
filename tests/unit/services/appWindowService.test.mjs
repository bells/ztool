import assert from "node:assert/strict";
import test from "node:test";
import {
  APP_WINDOW_COMMANDS,
  createAppWindowService,
} from "/private/tmp/zero-tests/services/appWindowService.js";

test("defines explicit Rust command names for app window actions", () => {
  assert.deepEqual(APP_WINDOW_COMMANDS, {
    openMainWindow: "show_main_window",
    openPreferencesWindow: "show_preferences_window",
    openAboutWindow: "show_about_window",
    quitApp: "quit_app",
  });
});

test("invokes app window commands through the injected bridge", async () => {
  const calls = [];
  const service = createAppWindowService(async (command) => {
    calls.push(command);
  });

  await service.openMainWindow();
  await service.openPreferencesWindow();
  await service.openAboutWindow();
  await service.quitApp();

  assert.deepEqual(calls, [
    "show_main_window",
    "show_preferences_window",
    "show_about_window",
    "quit_app",
  ]);
});
