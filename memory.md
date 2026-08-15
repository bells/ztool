# Zero Memory

Last updated: 2026-06-27

This file is a compact project memory for future maintainers and Codex runs. Treat it as a starting map, then verify against the live tree before making changes.

## Project Snapshot

- `zero` is a tray-first desktop utility collection, not a SaaS-style web app.
- Current stack: Tauri 2, Rust 2021, React 19, TypeScript, Vite, pnpm.
- Package manager is `pnpm@10.33.0`; main npm scripts are `pnpm dev`, `pnpm build`, and `pnpm tauri ...`.
- Current product surface is a compact undecorated tray window plus extra Tauri windows for screenshot workflows.
- Bundled plugins are registered as `zero.snap`, `zero.awake`, `zero.paper`, and `zero.launch`; preferences/about remain protected host surfaces.
- The plugin MVP is Git-based: plugin authors publish `.zplugin` ZIP packages through GitHub Releases, and Zero reads a hosted static `market.json` instead of using a server-backed marketplace.

## Current Architecture

- `src/main.tsx` routes the React root by Tauri window label:
  - `main` renders `MainApp`.
  - `capture` renders the screenshot editor `CaptureApp`.
  - `pin-*` renders `PinApp`.
- `src/core/pluginHost/` owns host contracts, market/registry services, extension isolation, Bridge permission checks, and generic extension UI.
- `src/core/preferences/` owns local preferences, About, storage, and host localization; `src/core/pluginHost/pluginTypes.ts` owns dynamic plugin presentation types.
- Each bundled plugin under `src/plugins/{caffeine,bingWallpaper,quickLauncher,screenshot}` owns one typed descriptor, local translations, surfaces, and domain code. `src/appShell/bundledPluginModules.ts` is the only frontend composition registry.
- Bundled native commands/state are explicitly composed at build time in `src-tauri/src/bundled_plugins.rs`; third-party `.zplugin` packages remain runtime-only and cannot load native Rust.
- Local preferences:
  - storage key: `ztool.preferences.v1`
  - launch-at-login uses `@tauri-apps/plugin-autostart`
  - language options are `system`, `zh-CN`, and `en-US`
  - at least one tool must stay visible.
- `src-tauri/src/lib.rs` registers tray behavior, the global screenshot shortcut, managed native state, plugins, and command handlers.
- `src-tauri/src/plugins/` owns Rust plugin contracts, Git market fetch/cache, `.zplugin` download/checksum/extraction, registry persistence under `~/.ztool/plugins/`, and guarded binary/script entrypoint execution.
- `src-tauri/capabilities/default.json` must include every window family used by commands. It currently allows `main`, `capture`, and `pin-*`.

## Screenshot Memory

- The previous screenshot phase-2 work delivered a macOS-first custom editor while keeping Windows on the system screenshot launcher path. Preserve that platform split unless the user explicitly asks for a Windows custom editor.
- Global screenshot shortcut is `CommandOrControl+Shift+A`.
- macOS flow:
  - `start_screenshot` hides the main window, captures the screen through `screencapture`, stores one active session, and opens the full-screen `capture` window.
  - `init_screenshot_session` returns `session_id`, `image_base64`, `initial_action`, `width`, and `height`.
  - `commit_screenshot` validates the active session, decodes the final PNG, copies or saves it, closes `capture`, restores `main`, and clears the active session.
  - `cancel_screenshot_session` closes `capture`, restores `main`, and clears the active session.
  - `pin_screenshot` crops from the final rendered PNG and opens an always-on-top `pin-*` window.
- Non-macOS screenshot flow currently starts the system tool. Windows uses `explorer.exe ms-screenclip:` with `SnippingTool.exe` fallback; Linux reports unsupported for this path.
- The main `ScreenshotPanel` still shows disabled/pending tool buttons. Do not infer actual editor capability from that panel alone: `CaptureApp` has select, rectangle, arrow, pen, text, mosaic, pin, undo/redo/delete, copy, save, and cancel paths.
- Screenshot command payloads intentionally use Rust-facing snake_case fields through `captureSerialize.ts`: `session_id`, `png_base64`, and `save_path`.
- Pin windows need unique labels like `pin-<id>`, capability globbing with `pin-*`, and native size based on decoded PNG dimensions plus titlebar height.
- Main risks in screenshot work:
  - coordinate conversion between rendered image bounds and original screenshot pixels
  - macOS Screen Recording permission failures from `screencapture`
  - stale or mismatched `session_id`
  - forgetting to restore the main window after commit, cancel, or capture failure
  - breaking Windows by forcing macOS-only assumptions into shared code.

## Caffeine Memory

- `src-tauri/src/services/caffeine.rs` owns native awake behavior.
- macOS starts and later kills a `caffeinate -d -i` child process.
- Windows uses `SetThreadExecutionState` with display and system required flags.
- Other platforms currently return unsupported.
- Keep the Rust state snapshot and frontend state in sync when changing this plugin.

## Verification Commands

Use a focused test level while iterating:

```bash
pnpm test:unit
pnpm test:integration
```

Other useful checks:

```bash
node scripts/validate-plugin-package.mjs examples/plugins/minimal-view-command-setting
pnpm test
pnpm build
cd src-tauri
cargo fmt --check
cargo check
cargo test
git diff --check
```

For plugin lifecycle work, also run the focused Rust suites:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test plugin_package --test plugin_registry --test plugin_runtime
```

For screenshot behavior, do at least one manual `pnpm tauri dev` pass on macOS when changing capture windows, copy/save, pin windows, or tray/shortcut behavior.

## Product Site Memory

- If building a public website, present ZTool as a desktop toolbox and tray utility, not as a generic SaaS.
- A separate static `site/` app is a better first shape than mixing product-site content into the Tauri app `index.html`.
- Prior planning favored Astro + Vercel: Astro for the lightweight static site framework, Vercel for previews, CDN, domains, SSL, and Git-based deploys.
- A good first site structure: hero + CTA, short feature blocks, real screenshots/demo, download section, docs/FAQ, then i18n and changelog later.
- Reference balance: Maccy suggests a minimal single-message homepage; CC Switch suggests denser multilingual/docs-oriented product structure.

## Working Preferences

- For bug reports, trace the actual codepath and interaction path before fixing. Avoid stopping at a plausible theory.
- For screenshot work, prioritize making every tool path usable before visual polish.
- Preserve macOS and Windows differences when the user explicitly asks for platform-specific behavior.
- Before saying work is complete, run repo-native verification or state clearly what could not be run.
- If asked to commit, stage only relevant files and commit intentionally.
