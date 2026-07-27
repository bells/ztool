# Zero

Zero is a tray-first desktop utility collection built with Tauri 2, React, and TypeScript. Each tool is designed as a plugin so the app can grow into a larger toolbox without turning the main window into a crowded control panel.

## Current Features

- Zero Snap with selection preview, dimensions, toolbar actions, copy, and save entry points.
- Global screenshot shortcut: `CommandOrControl+Shift+A`.
- Zero Awake to keep the screen and system awake.
- Zero Paper with Bing daily wallpaper history, cache-first loading, save-to-Downloads, and one-click desktop apply.
- Zero Launch for fuzzy application/system-setting search, app launch/focus, and a reusable `CommandOrControl+Shift+Space` window on macOS and Windows.
- Preferences panel with real login autostart support.
- Tool visibility preferences so users can choose which plugins appear in the main tool list.
- Language preference with system default, Chinese, and English.
- About and Exit actions in a compact bottom system section.
- MVP plugin host: GitHub Releases `.zplugin` packages, hosted `market.json`, local package validation/install, permission review, enable/disable/uninstall, and bundled restore.

## Tech Stack

- Tauri 2
- Rust 2021
- React 19
- TypeScript
- Vite
- pnpm

## Prerequisites

- Node.js 18+
- pnpm
- Rust via `rustup`
- Platform build tools:
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio Build Tools and WebView2
  - Linux: WebKitGTK and app indicator dependencies required by Tauri

## Development

Install dependencies:

```bash
pnpm install
```

Run the frontend only:

```bash
pnpm dev
```

Run the desktop app:

```bash
pnpm tauri dev
```

Build the frontend:

```bash
pnpm build
```

Check the Rust side:

```bash
cd src-tauri
cargo check
cargo test
```

Build the desktop app:

```bash
pnpm tauri build
```

## Project Structure

```text
src/
  App.tsx                         Main tray shell
  App.css                         Application styling
  plugins/
    pluginHost/                   Runtime plugin registry, market, extension bridge, and host UI
    caffeine/                     Zero Awake tool UI and state bridge
    bingWallpaper/                Bing wallpaper contracts, model, hook, service, and card
    quickLauncher/                Launcher contracts, model, hook, shared view, panel, and window
    screenshot/                   Zero Snap tool UI and state bridge
    preferences/                  Preferences, about, and preference model
src-tauri/
  src/
    commands/                     Tauri command handlers
    services/                     Native service logic
  capabilities/                   Tauri permission capabilities
tests/
  preferencesModel.test.mjs       Preference model tests
  pluginHost*.test.mjs            Plugin host service/state tests
  extensionRuntime.test.mjs       Extension bridge and isolation tests
  i18n.test.mjs                   Language resolution and translation tests
  screenshotMeta.test.mjs         Zero Snap metadata tests
```

## Zero Paper

`zero.paper` is the third bundled tool. It reads up to 10 records from Bing's `zh-CN` daily image feed, shows a validated cached snapshot immediately, and refreshes metadata and missing images in Rust. The card supports older/newer navigation, a separate Downloads action, and applying the selected image as the desktop wallpaper. Clicking the preview is equivalent to Apply.

The cache lives at `~/.zero/data/wallpaper/`. `index.json` and image files are written through plugin-scoped, staged replacement; the service retains at most 10 indexed entries and removes only obsolete files that an earlier index owned. Unknown files are not deleted. A failed refresh leaves usable cached records visible with stale/error state.

The bundled manifest requests `network`, `storage.plugin`, and `system.wallpaper`. The WebView does not fetch Bing or receive unrestricted filesystem access: Rust restricts requests to bounded HTTPS Bing endpoints, validates image content, resolves paths inside the wallpaper root, and passes only one selected preview back as a bounded data URL.

Desktop wallpaper apply uses the replaceable `WallpaperSetter` adapter around `wallpaper 3.2`. macOS and Windows are supported by the adapter. Linux support depends on the detected desktop environment and its required desktop command; unsupported or missing backends return a structured error while browse/download remain available. Mobile wallpaper apply is not part of this release.

This bundled plugin uses the existing `webview` runtime and React renderer. It intentionally does not introduce a `plugin.wasm` or WASI runtime.

## Zero Launch

`zero.launch` is the fourth bundled tool. The main plugin panel and the floating `launcher` window share one React view and one Rust-owned index. Press `CommandOrControl+Shift+Space`, type an English/Chinese name, full pinyin, initials, acronym, or bundled alias such as `wx`/`ps`, then use `ArrowUp`/`ArrowDown` and `Enter`. `Escape` or loss of focus hides only the floating window.

Rust scans macOS application bundles in `/Applications`, `~/Applications`, `/System/Applications`, and `/System/Applications/Utilities`; Windows scans machine/user Start Menu Programs `.lnk` and `.exe` entries. Running applications are focused when the OS provides a reliable identity; otherwise the validated indexed entry is launched. Common system settings are host-maintained catalog records mapped privately to `x-apple.systempreferences:` or `ms-settings:` destinations. Linux and mobile return explicit unsupported state in this release.

The versioned local data is stored under `~/.zero/data/quick-launcher/`: `apps_cache.json` contains rebuildable application metadata, `usage.json` contains bounded success-only counts and last-used timestamps, and icons are lazy/rebuildable. Raw queries are never persisted or uploaded. Startup uses the cache immediately, refreshes in the background, and coalesces application-directory changes.

The manifest requests `system.apps.read`, `system.apps.execute`, `system.window.focus`, and `system.settings.open`. Bundled Tauri commands and approved Extension Bridge methods resolve only host-issued `itemId` values; callers cannot submit arbitrary paths, Bundle IDs, command lines, shortcut targets, or URIs. Like the other bundled tools, Zero Launch uses the existing React `webview` renderer and intentionally does not add `plugin.wasm`/WASI.

## Plugin MVP

The first plugin market is repository-based, not server-backed:

- plugin authors publish a `.zplugin` ZIP archive in their own GitHub Releases;
- Zero reads a hosted static `market.json`;
- install extracts packages under `~/.zero/plugins/<plugin>/<version>/`;
- `manifest.json` requires `name`, `version`, `author`, `main`, and `permissions`;
- permissions are reviewed before install and enforced by a host-mediated Extension API bridge.

## Upgrade compatibility

Zero keeps `com.watson.ztool` as its immutable Tauri bundle identifier so an existing installation upgrades in place and retains operating-system permissions, autostart registration, and WebView storage. This identifier is a compatibility value, not the current product name.

On startup, Zero copies missing data from `~/.ztool` into `~/.zero`, normalizes the four known first-party plugin IDs, and leaves the legacy directory untouched for rollback. Canonical `~/.zero` data wins when both locations contain the same unit. Frontend preferences follow the same rule: `zero.preferences.v1` wins, otherwise `ztool.preferences.v1` is read and copied forward without deleting the legacy key. Extension API v1 continues to accept `engines.ztool`, while current manifests use `engines.zero`.

Developer docs:

- [MVP protocol](docs/plugins/mvp-plugin-protocol.md)
- [Developer guide](docs/plugins/developer-guide.md)
- [GitHub Releases publishing guide](docs/plugins/publishing-github-releases.md)
- [Minimal example plugin](examples/plugins/minimal-view-command-setting)

Validate an unpacked plugin directory:

```bash
node scripts/validate-plugin-package.mjs examples/plugins/minimal-view-command-setting
```

## Verification

Recommended checks before pushing:

```bash
pnpm exec tsc src/brand/identity.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src --outDir /private/tmp/zero-brand-test --noEmit false --skipLibCheck
node --test tests/brandIdentity.test.mjs
pnpm exec tsc src/plugins/preferences/preferencesModel.ts src/plugins/preferences/preferencesStorage.ts src/plugins/types.ts src/brand/identity.ts --module ES2020 --moduleResolution bundler --target ES2020 --rootDir src --outDir /private/tmp/zero-preferences-test --noEmit false --skipLibCheck
node --test tests/preferencesModel.test.mjs tests/preferencesStorage.test.mjs
pnpm exec tsc src/plugins/preferences/i18n.ts src/plugins/preferences/preferencesModel.ts src/plugins/types.ts src/brand/identity.ts --module ES2020 --moduleResolution bundler --target ES2020 --rootDir src --outDir /private/tmp/zero-i18n-test --noEmit false --skipLibCheck
node --test tests/i18n.test.mjs
pnpm exec tsc src/plugins/screenshot/screenshotMeta.ts --module ES2020 --moduleResolution bundler --target ES2020 --outDir /private/tmp/zero-screenshot-test --noEmit false --skipLibCheck
node --test tests/screenshotMeta.test.mjs
pnpm exec tsc src/plugins/pluginHost/contracts.ts src/plugins/pluginHost/validation.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src/plugins/pluginHost --outDir /private/tmp/zero-plugin-host-test --noEmit false --skipLibCheck
pnpm exec tsc src/plugins/pluginHost/contracts.ts src/plugins/pluginHost/pluginHostServiceCore.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src/plugins/pluginHost --outDir /private/tmp/zero-plugin-host-service-test --noEmit false --skipLibCheck
pnpm exec tsc src/plugins/pluginHost/contracts.ts src/plugins/pluginHost/pluginHostModel.ts src/plugins/pluginHost/pluginMarketModel.ts src/brand/identity.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src --outDir /private/tmp/zero-plugin-host-state-test --noEmit false --skipLibCheck
pnpm exec tsc src/plugins/pluginHost/contracts.ts src/plugins/pluginHost/extensionBridge.ts src/plugins/quickLauncher/contracts.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src/plugins --outDir /private/tmp/zero-extension-runtime-test --noEmit false --skipLibCheck
node --test tests/pluginHostService.test.mjs tests/pluginHostModel.test.mjs tests/pluginMarketModel.test.mjs tests/extensionRuntime.test.mjs
pnpm exec tsc src/plugins/bingWallpaper/contracts.ts src/plugins/bingWallpaper/bingWallpaperModel.ts src/plugins/bingWallpaper/bingWallpaperController.ts src/plugins/bingWallpaper/bingWallpaperServiceCore.ts --module ES2020 --moduleResolution bundler --target ES2022 --outDir /private/tmp/zero-bing-wallpaper-test --noEmit false --skipLibCheck
node --test tests/bingWallpaperModel.test.mjs tests/bingWallpaperController.test.mjs tests/bingWallpaperService.test.mjs
pnpm exec tsc src/plugins/quickLauncher/contracts.ts src/plugins/quickLauncher/quickLauncherModel.ts src/plugins/quickLauncher/quickLauncherServiceCore.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src/plugins/quickLauncher --outDir /private/tmp/zero-quick-launcher-test --noEmit false --skipLibCheck
pnpm exec tsc src/components/statusBarIconSources.ts src/plugins/pluginHost/contracts.ts --module ES2020 --moduleResolution bundler --target ES2022 --rootDir src --outDir /private/tmp/zero-status-bar-icons-test --noEmit false --skipLibCheck
node --test tests/quickLauncherModel.test.mjs tests/quickLauncherService.test.mjs
node --test tests/statusBarIconSources.test.mjs
pnpm build
cd src-tauri && cargo check && cargo test
cargo test --release --test quick_launcher_benchmark -- --ignored --nocapture
```

## Notes

- `node_modules`, frontend build output, and Rust/Tauri build output are intentionally ignored.
- The login autostart preference uses the official Tauri autostart plugin.
- Zero Snap annotation tools are intentionally phased: the UI direction is in place, while deeper annotation behavior can be implemented plugin by plugin.
