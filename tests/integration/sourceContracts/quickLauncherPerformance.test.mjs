import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

test("Launch search uses immutable snapshots without query-path platform work", () => {
  const source = fs.readFileSync("src-tauri/src/services/quick_launcher/mod.rs", "utf8");
  const search = source.slice(
    source.indexOf("    pub fn search("),
    source.indexOf("    pub fn icon("),
  );
  assert.match(search, /Arc::clone\(&index\.items\)/);
  assert.match(search, /Arc::clone\(&index\.usage\)/);
  assert.match(search, /search_items_thread_local/);
  assert.doesNotMatch(search, /index\.items\.clone|platform::|load_icon|probe_running|std::fs/);
});

test("Launch running state and icons use bounded out-of-query contracts", () => {
  const rust = fs.readFileSync("src-tauri/src/services/quick_launcher/mod.rs", "utf8");
  const commands = fs.readFileSync("src-tauri/src/commands/quick_launcher.rs", "utf8");
  const frontend = fs.readFileSync("src/plugins/quickLauncher/useQuickLauncher.ts", "utf8");

  assert.match(rust, /RUNNING_CACHE_TTL: Duration = Duration::from_secs\(2\)/);
  assert.match(rust, /MAX_ICON_BATCH_ITEMS: usize = 24/);
  assert.match(rust, /MAX_ICON_CACHE_ITEMS: usize = 128/);
  assert.match(rust, /MAX_ICON_CACHE_BYTES: usize = 8 \* 1024 \* 1024/);
  assert.match(commands, /zero:\/\/quick-launcher\/running-state-updated/);
  assert.match(frontend, /createLatestQueryScheduler/);
  assert.match(frontend, /client\.getIcons/);
  assert.doesNotMatch(frontend, /for \(const item of iconItems\)/);
});
