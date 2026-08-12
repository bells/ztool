## 1. Lock down activation and positioning contracts

- [x] 1.1 Add focused Rust tests for native Launch/Paper activation targets and regression coverage proving generic, Snap, Awake, primary, and fallback actions retain their current effects.
- [x] 1.2 Extract and test pure grouped-cell rectangle and Paper window clamping helpers, including collapsed/hidden tools, left and right display edges, invalid geometry, and physical-coordinate cases.
- [x] 1.3 Add or update React routing tests for the new `paper` surface while preserving all existing window-label mappings.

## 2. Build the dedicated Paper surface

- [x] 2.1 Add a host-owned Paper window module with one stable label, explicit fixed utility-window options, lazy creation, show/hide/toggle behavior, focus handling, Escape/blur dismissal, and tray-relative fallback positioning.
- [x] 2.2 Calculate the preferred Paper position from the clicked virtual glyph cell, clamp it to the active display's usable bounds, and return clear errors without claiming success when window operations fail.
- [x] 2.3 Add `paper` to the Tauri capability allowlist and React surface router.
- [x] 2.4 Add a thin `PaperApp` wrapper that resolves the saved/system language and renders the existing `BingWallpaperPanel` and `useBingWallpaper` flow without shell navigation or duplicated service logic.
- [x] 2.5 Add compact Paper-window styles for the existing loading, preview, metadata, navigation, download, apply, stale, error, narrow-width, and keyboard-focus states.

## 3. Route native status-bar clicks to tool surfaces

- [x] 3.1 Refactor the Quick Launcher show entry point so both the global shortcut and native Launch glyph use the same show/center/focus/reset implementation.
- [x] 3.2 Route a resolved native macOS Launch click directly to the shared launcher entry point and ensure it does not show the general Zero shell or emit generic plugin navigation.
- [x] 3.3 Route a resolved native macOS Paper click, with its clicked cell rectangle, to the Paper window toggle.
- [x] 3.4 Coordinate transient windows so showing Launch hides Paper/tray, showing Paper hides Launch/tray, repeated status-bar activation hides the existing Launch or Paper window, and repeated Launch shortcut activation refocuses/resets.
- [x] 3.5 Preserve `run_status_bar_item_action` fallback behavior, primary collapse/expand, visible-tool preferences, grouped hit testing, and right-click quit menus with regression tests.

## 4. Verify automated behavior

- [x] 4.1 Run the focused TypeScript compilation and Node tests for app-surface routing, status-bar models, and any new pure Paper surface helpers.
- [x] 4.2 Run focused Rust status-bar and window tests, then `cargo fmt --check`, `cargo check`, and `cargo test` from `src-tauri`.
- [x] 4.3 Run the full frontend gate (`node --test tests/*.mjs` after recreating documented temporary compiled fixtures when needed) and `pnpm build`.
- [x] 4.4 Run `openspec validate open-tool-surfaces-from-status-bar --type change --strict` and `git diff --check`.

## 5. Verify real macOS interaction

- [ ] 5.1 Launch `pnpm tauri dev` and verify the Launch glyph produces the same centered, focused, immediately typeable UI as `CommandOrControl+Shift+Space`, while a second glyph click hides it and repeated shortcut activation refocuses/resets it; also verify Escape, blur, and item execution.
- [ ] 5.2 Verify the Paper glyph opens only Paper content below the correct glyph, toggles on a second click, dismisses safely, and preserves browse/download/apply/refresh/error behavior.
- [ ] 5.3 Verify Paper placement and clamping near both screen edges and on each available monitor/scale configuration; record any hardware configuration that is unavailable for testing.
- [ ] 5.4 Verify switching among tray, Launch, and Paper never leaves overlapping transient windows, including during Paper save/apply interactions.
- [ ] 5.5 Verify primary collapse/expand, per-tool visibility, grouped glyph targeting, and right-click quit menus still work in the real macOS menu bar.
- [ ] 5.6 Smoke the non-macOS fallback path on available CI/device targets and explicitly leave unavailable Windows/Linux runtime validation pending rather than inferring it from cross-target checks.
