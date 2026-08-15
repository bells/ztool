# Zero Project Context

> 本文件用于给 OpenSpec 和协作代理提供项目级上下文。创建 proposal、design、spec 或 tasks 前，先阅读这里，再结合 live repo 校准判断；不要把它当作一次性模板。

## Project Overview

Zero 是一个托盘优先（tray-first）的跨平台桌面工具箱，目标是把常用小工具做成紧凑、稳定、可渐进扩展的插件集合。它不是 SaaS 网页应用，也不是大而全的控制台；主窗口应该像一个轻量的桌面控制中心：工具列表、当前工具面板、系统操作和偏好入口。

当前产品形态：

- 主窗口：隐藏在托盘后的紧凑无边框窗口，默认宽约 400px、高约 500px。
- 插件化工具：每个用户可见工具都应该像独立插件一样演进。
- 当前插件/模块：
  - Zero Launch：快速搜索、启动或切换应用与系统设置。
  - Zero Snap：全局快捷键、选择/编辑窗口、复制、保存、钉图。
  - Zero Awake：保持屏幕和系统唤醒。
  - Zero Paper：浏览、缓存、保存并应用 Bing 每日壁纸。
  - 偏好/关于：登录自启动、语言、工具显示开关、关于信息。

## Tech Stack

- Desktop shell: Tauri 2
- Native backend: Rust 2021
- Frontend: React 19 + TypeScript
- Build tool: Vite
- Package manager: pnpm 10.33.0
- Tauri plugins:
  - @tauri-apps/plugin-autostart / tauri-plugin-autostart
  - @tauri-apps/plugin-global-shortcut / tauri-plugin-global-shortcut
  - @tauri-apps/plugin-opener / tauri-plugin-opener
  - tauri-plugin-positioner
- Native Rust helpers include base64, rfd, image, macOS cocoa/objc/window-vibrancy, and Windows power APIs through the windows crate.

## Source Layout

    src/
      main.tsx                         Routes React entry by Tauri window label
      App.tsx                          Main tray shell and top-level plugin selection
      App.css                          Compact tray UI and capture/pin window styles
      plugins/
        types.ts                       PluginId and PluginMeta contract
        screenshot/                    Zero Snap panel, capture editor, pin window helpers
        caffeine/                      Zero Awake UI and frontend state bridge
        preferences/                   Preferences, about, i18n, preference model

    src-tauri/
      src/
        lib.rs                         Tauri builder, tray, shortcut, command registration
        commands/                      Thin #[tauri::command] handlers
        services/                      Native service logic and platform-specific behavior
      capabilities/default.json        Tauri permissions and allowed window labels
      tauri.conf.json                  Window, bundle, build, identifier config

    tests/
      *.mjs                            Node tests for extracted TypeScript business logic

## Architecture Principles

Follow a Clean Architecture style with a pragmatic Tauri boundary:

- Rust owns native capabilities: tray behavior, global shortcuts, screenshot capture, clipboard/file save, power-management APIs, Tauri window creation, process/system calls, and platform-specific error handling.
- React owns UI rendering, local interaction state, keyboard/pointer handling inside WebViews, local preferences UI, and small pure UI-domain helpers.
- Tauri commands should be thin. Put command handlers in src-tauri/src/commands/, delegate real work to src-tauri/src/services/, and register commands in src-tauri/src/lib.rs.
- Frontend plugin code should stay close to the plugin that owns it. Prefer focused plugin-scoped changes over broad app-wide rewrites.
- Shared contracts across the Rust <-> TypeScript IPC boundary must be explicit, stable, and symmetric.
- Do not introduce TypeScript any. Use explicit interfaces, discriminated unions, or unknown with narrowing.
- Keep logic testable: extract pure helpers from React components when behavior can be tested without Tauri.

## Plugin Model

Each user-facing tool is treated as a plugin.

When adding or renaming a plugin, update these together:

- src/plugins/types.ts: PluginId and metadata contracts
- src/App.tsx: plugin list and routing
- plugin folder under src/plugins/<plugin>/
- i18n keys in src/plugins/preferences/i18n.ts
- preference defaults and normalization in src/plugins/preferences/preferencesModel.ts
- relevant tests under tests/
- Rust commands/services and Tauri capabilities if native APIs are needed

The main shell should preserve three stable areas:

1. Tool list
2. Current tool display
3. System actions and preferences/about/quit controls

## IPC and Data Contracts

IPC payloads are part of the public contract between Rust and React.

- Rust structs crossing IPC should derive Serialize and/or Deserialize as appropriate.
- Frontend should define matching TypeScript interfaces for IPC responses and request payloads.
- Rust-facing command inputs currently use snake_case field names where the Rust struct expects them. Convert at the boundary with helper functions instead of scattering field-name conversions through UI code.
- Example current pattern:
  - Rust input: CommitScreenshotInput { session_id, action, png_base64, save_path }
  - TS helper: buildCommitScreenshotPayload(...) returns { input: { session_id, action, png_base64, save_path } }
- Tauri commands should return Result<T, String> when failure is possible.
- Frontend invoke calls should catch and surface errors in plugin state instead of throwing through the UI tree.

## Window and Runtime Model

Current Tauri windows:

- main: tray control center; routes to MainApp.
- capture: full-screen screenshot editor; routes to CaptureApp.
- pin-*: always-on-top pinned image windows; route to PinApp.

src/main.tsx routes by getCurrentWindow().label:

- main and unknown labels -> MainApp
- capture -> CaptureApp
- labels starting with pin -> PinApp

src-tauri/capabilities/default.json must stay in sync with command-created windows and currently allows main, capture, and pin-*.

The tray icon toggles the main window and uses tauri-plugin-positioner to move it near the tray. A debounce guards repeated tray click events.

## Platform Behavior

Preserve platform-specific behavior unless a proposal explicitly changes it.

### Zero Snap

- Global shortcut: CommandOrControl+Shift+A.
- macOS:
  - start_screenshot hides main, captures a full-screen PNG through screencapture, stores an active session, and opens the full-screen capture window.
  - CaptureApp handles annotation/edit interactions in React and commits a rendered PNG back to Rust.
  - Copy uses AppleScript/clipboard flow; save uses filesystem or file dialog; pin creates pin-* windows.
- Windows:
  - Uses the system launcher path: explorer.exe ms-screenclip: with SnippingTool.exe fallback.
  - Do not force macOS-only custom editor assumptions into the Windows path without an explicit design.
- Linux/other:
  - Zero Snap launcher support may be unsupported and should report a clear error.

Key screenshot risks:

- rendered-image-to-original-pixel coordinate conversion
- stale or mismatched session_id
- failure to restore main after commit/cancel/failure
- macOS Screen Recording permission failure from screencapture
- capability drift for capture and pin-* windows

### Zero Awake

- macOS uses a managed caffeinate -d -i child process.
- Windows uses SetThreadExecutionState with display and system required flags.
- Other platforms currently return unsupported.
- Keep Rust CaffeineSnapshot and frontend state in sync when changing this feature.

### Preferences

- Preferences are stored in localStorage under zero.preferences.v1.
- Language options are system, zh-CN, and en-US.
- At least one tool must remain visible.
- Login autostart uses the official Tauri autostart plugin; read/write errors should be surfaced in the preferences panel.

## UI and Interaction Guidelines

- The UI should stay dense, readable, and stable at tray-window size.
- Avoid large marketing-page layouts inside the app shell.
- Keep touch/click targets practical, but prioritize desktop tray ergonomics.
- Prefer familiar compact controls for toggles, tool actions, and system actions.
- For new windows, define the routing, size, focus behavior, taskbar behavior, and capability permissions together.
- For screenshot editor changes, verify real Tauri window behavior; browser-only checks are not enough.

## Code Style and Quality Rules

- TypeScript strict mode is enabled; keep noUnusedLocals, noUnusedParameters, and noFallthroughCasesInSwitch clean.
- Rust uses 2021 edition. Keep native service functions small and explicit about failure paths.
- Prefer single-responsibility React components and hooks.
- Avoid deep async nesting in React; wrap async state transitions in focused hooks/services.
- Comments should explain why, not restate what the code already says.
- Do not commit generated output such as dist/, node_modules/, .pnpm-store/, or src-tauri/target/.

## OpenSpec Workflow Expectations

For non-trivial behavior changes:

- Start with a proposal that states the user problem, scope, non-goals, affected surfaces, platform implications, and verification plan.
- Include Rust command/service changes and TS invoke/contract changes in the same design when IPC is involved.
- Break tasks into small, verifiable steps; prefer plugin-scoped milestones.
- Main specs should describe externally observable behavior, not implementation details.
- If a change touches screenshot, tray, windows, global shortcuts, autostart, or platform APIs, call out manual verification needs.

## Verification Commands

Use focused checks while iterating:

    pnpm exec tsc src/plugins/preferences/preferencesModel.ts src/plugins/types.ts --module ES2020 --moduleResolution bundler --target ES2020 --outDir /private/tmp/zero-preferences-test --noEmit false --skipLibCheck
    node --test tests/preferencesModel.test.mjs

    pnpm exec tsc src/plugins/preferences/i18n.ts src/plugins/preferences/preferencesModel.ts --module ES2020 --moduleResolution bundler --target ES2020 --outDir /private/tmp/zero-i18n-test --noEmit false --skipLibCheck
    node --test tests/i18n.test.mjs

    pnpm exec tsc src/plugins/screenshot/screenshotMeta.ts --module ES2020 --moduleResolution bundler --target ES2020 --outDir /private/tmp/zero-screenshot-test --noEmit false --skipLibCheck
    node --test tests/screenshotMeta.test.mjs

For screenshot capture helpers:

    pnpm exec tsc src/plugins/screenshot/capture/captureReducer.ts src/plugins/screenshot/capture/captureHotkeys.ts src/plugins/screenshot/capture/captureSerialize.ts src/plugins/screenshot/capture/captureCanvas.ts src/plugins/screenshot/capture/captureSelectionModel.ts src/plugins/screenshot/capture/captureToolbarModel.ts --module ES2020 --moduleResolution bundler --target ES2022 --outDir /private/tmp/zero-capture-test --noEmit false --skipLibCheck
    node --test tests/captureReducer.test.mjs tests/captureHotkeys.test.mjs tests/captureSerialize.test.mjs tests/captureCanvas.test.mjs tests/captureSelectionModel.test.mjs tests/captureToolbarModel.test.mjs tests/captureToolbarSource.test.mjs

Before considering implementation complete, run the relevant subset plus:

    node --test tests/*.mjs
    pnpm build
    cd src-tauri && cargo check && cargo test
    git diff --check

For UI, tray, shortcut, screenshot-window, copy/save, pin-window, or native behavior changes, also run:

    pnpm tauri dev

Then manually inspect the real desktop app flow.

## Current Product Direction

Zero should grow as a desktop toolbox with small, reliable native utilities. A future public website, if built, should live separately from the app UI and present Zero as a desktop toolbox rather than a generic SaaS. Prior planning favored a lightweight static site, e.g. Astro for the site framework and Vercel for preview/deployment infrastructure.
