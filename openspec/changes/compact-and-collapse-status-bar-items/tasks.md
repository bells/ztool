## 1. Persisted Settings and Shared Contracts

- [x] 1.1 Extend Rust `StatusBarSettings` and `UpdateStatusBarSettingsInput` with the camelCase `pluginItemsCollapsed` field, defaulting missing legacy values to expanded without discarding other stored settings.
- [x] 1.2 Extend the matching TypeScript settings snapshot/update contracts, defaults, normalization, and optimistic update helper without using `any`.
- [x] 1.3 Add Rust tests for legacy JSON migration, default expanded state, collapse/expand updates, persistence, and corrupt-file recovery with the new field.
- [x] 1.4 Add focused TypeScript tests for collapsed-state normalization and optimistic updates while preserving existing visibility settings.

## 2. macOS Compact Native Layout

- [x] 2.1 Add one macOS compact-width constant set to the real-device-selected 22pt value and pure layout-resolution helpers that distinguish primary, expanded tool, hidden collapsed tool, and fallback-platform behavior.
- [x] 2.2 Add tests proving the primary remains 22pt, expanded tools use the same width, collapsed tools are hidden without reserved slots, and non-macOS fallback output is unaffected.
- [x] 2.3 Pin Tauri to the current 2.11 minor required by the `with_inner_tray_icon` seam and keep the dependency/lockfile change scoped to compatibility.
- [x] 2.4 Add a macOS-only helper that accesses the underlying `NSStatusItem` on the main thread and applies the resolved length without changing Tauri's tray ownership or event routing.
- [x] 2.5 Apply the compact width to the primary and expanded tool items, and hide collapsed tool items immediately after native creation.
- [x] 2.6 Ensure startup, settings refresh, plugin enable/disable/install/uninstall/restore, and Awake icon-state refresh all reapply the persisted layout state without changing deterministic item order.
- [x] 2.7 Tune the shared compact width from 24pt to 22pt after real-device spacing feedback and keep the independent-status-item system-spacing limitation explicit.

## 3. Primary Right-Click Collapse Control

- [x] 3.1 Add focused Rust helpers and tests for state-aware Chinese/English “Collapse Tool Icons” and “Expand Tool Icons” native menu labels.
- [x] 3.2 Build and attach the collapse/expand menu only to the macOS primary Ø item, using a stable menu-item id while keeping `show_menu_on_left_click(false)`.
- [x] 3.3 Route the native menu event through a single Rust toggle service that persists the inverted collapsed state and refreshes the native layout and menu label.
- [x] 3.4 Add regression tests showing primary left click still toggles the tray panel, tool actions remain unchanged, and repeated collapse/expand events produce consistent persisted state.
- [x] 3.5 Keep the primary tray item alive during its own collapse/expand menu event, update existing tool visibility and the attached menu in place, and cover successful persistence plus rollback behavior with focused tests.
- [x] 3.6 Add “Quit Zero Status Bar” as the primary menu's second localized item, route it to clean application exit, and cover menu labels and action routing with focused tests.
- [x] 3.7 Keep the primary Ø continuously visible during expansion, add a one-item localized quit menu to every macOS tool item with unique action routing, and cover both behaviors with focused tests.

## 4. Cross-Platform Fallback Protection

- [x] 4.1 Keep `StatusBarSupport::NativeMultiItem` macOS-only and ensure Windows/Linux create only the primary native tray icon with no tool-collapse menu item.
- [x] 4.2 Update frontend preview/fallback helpers as needed so `pluginItemsCollapsed` never suppresses the non-macOS tray quick-panel action row.
- [x] 4.3 Add regression coverage for a fallback platform loading both collapsed and expanded settings while returning the same available tool actions.

## 5. Verification

- [x] 5.1 Run `openspec validate "compact-and-collapse-status-bar-items" --type change --strict`.
- [x] 5.2 Run focused TypeScript compilation and status-bar model/controller/service tests, then run `node --test tests/*.mjs` and `pnpm build`.
- [x] 5.3 Run `cargo fmt --check`, `cargo check`, and `cargo test` for `src-tauri`, plus the Windows target check when the configured toolchain is available; document any device-only boundary.
- [x] 5.4 Run `git diff --check` and review the final diff for unintended `.playwright-cli/`, `output/`, build artifacts, or unrelated workspace changes.
- [ ] 5.5 Manually verify on macOS that the selected 22pt width remains clickable, and test collapse without blank slots, expand with the primary Ø continuously visible, restart restoration, primary left click, tool left clicks, every tool's right-click quit menu, visibility changes, primary-menu quit, and Awake icon refresh without ordering drift or flicker.
- [x] 5.6 Verify on a Windows device that Zero still presents one Ø tray icon and the existing fallback tool row, or explicitly record that Windows runtime behavior remains device-unverified.

## Verification Notes

- macOS source checks and tests pass, but `computer-use` cannot inspect or operate `SystemUIServer`; task 5.5 remains open for a real menu-bar smoke test.
- The installed `x86_64-pc-windows-msvc` Rust target was attempted from macOS, but the third-party `aws-lc-sys` build requires a Windows SDK (`windows.h`). Windows runtime behavior therefore remains device-unverified; source-level fallback tests confirm that collapsed settings do not hide the single-icon quick-panel action row.
- Existing untracked `.playwright-cli/` and `output/` directories were not modified or included in this change.
