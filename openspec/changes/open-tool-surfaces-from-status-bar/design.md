## Context

On macOS, Zero renders the primary glyph and enabled tool glyphs as cells in one compact native status item. Hit testing already resolves a click to a stable status-bar item, but every `open-plugin` action then follows the same generic path: show the main window and emit `status-bar-open-plugin`. Consequently, Launch does not use its existing shortcut surface and Paper is wrapped in the whole Zero shell.

Zero Launch already has the correct dedicated window (`launcher`) and one host function, `show_quick_launcher_window`, used by the global shortcut. Zero Paper already isolates its behavior in `BingWallpaperPanel`, `useBingWallpaper`, and typed services, but it has no dedicated Tauri window or React surface. The Paper window must be anchored to the virtual Paper cell inside the grouped status item, not merely centered beneath the full item. Native `SystemUIServer` geometry and multi-monitor screen edges require runtime verification.

## Goals / Non-Goals

**Goals:**

- Route a macOS click on the Launch glyph to the exact existing shortcut window entry point.
- Show a compact Paper-only window directly below the clicked Paper cell.
- Reuse one Paper UI/business-state implementation across the main shell and dedicated window.
- Define deterministic toggle, focus, blur, window-coordination, and edge-clamping behavior.
- Preserve typed boundaries, testable pure routing/geometry helpers, status-item collapse, and existing fallback-platform behavior.

**Non-Goals:**

- Redesigning the Launch results shown in the reference screenshot or changing its search/index/activation semantics.
- Redesigning the Paper card or duplicating its hook, cache, IPC, download, or apply logic.
- Moving Snap or Awake into new dedicated windows.
- Changing the primary Zero glyph, grouped status-item layout, collapse/expand persistence, tool visibility preferences, or right-click menus.
- Replacing the non-macOS single-tray fallback row with multiple native items.

## Decisions

### 1. Dispatch resolved status-bar targets through a surface-aware activation policy

The Rust status-bar service will keep hit testing and native window operations under host control. After resolving the clicked item, it will select an activation target from the canonical plugin name and activation context:

- native macOS Launch → show the Launch window;
- native macOS Paper → toggle the anchored Paper window;
- all other tools and fallback invocations → retain their existing action behavior.

The grouped click path will carry the clicked cell rectangle, derived from the grouped item rectangle and resolved cell index, into Paper positioning. The generic `run_status_bar_item_action` command used by the non-macOS fallback UI will retain its current main-window/plugin-navigation semantics.

This keeps platform policy out of React and avoids expanding the serialized `StatusBarAction` contract solely for two bundled tools. An alternative was to add new public action types such as `show-launcher` and `show-paper`; that would make third-party manifests and both Rust/TypeScript contracts understand host window details while still not providing the click geometry Paper needs.

### 2. Launch reuses one host-owned show function

The global shortcut will continue to call `show_quick_launcher_window`, while native status-bar activation will call a thin `toggle_quick_launcher_window` wrapper that delegates its show branch to that same entry point. This preserves the same window label, centered placement, focus, `quick-launcher-shown` reset event, blur dismissal, index state, and activation flow whenever Launch is shown. The status-bar path will not open the main window or emit generic plugin navigation first.

Before showing Launch, Zero will hide other transient Zero tool surfaces (the tray panel and Paper window) so overlapping always-on-top utilities cannot remain visible. A second status-bar click hides the visible Launch window, while a repeated shortcut invocation retains the existing show/focus/reset behavior. Blur dismissal is delayed briefly and rechecks focus so the menu-bar click can resolve the visible-window toggle before blur independently hides it.

An alternative was to reproduce the launcher panel inside a tray-relative window. That would diverge from the explicitly requested shortcut-equivalent behavior and create a second interaction lifecycle.

### 3. Paper gets a host-created `paper` window and a thin React app wrapper

Rust will own creation, placement, visibility, focus, and hiding of a single lazily created `paper` webview window. Its options will describe a compact, fixed-size, undecorated, transparent, always-on-top, taskbar-hidden utility surface. The Tauri capability allowlist and `resolveAppSurface` will include the new label.

`PaperApp` will resolve the current language and render `BingWallpaperPanel` inside a Paper-specific root only. The panel and `useBingWallpaper` remain the sole implementation of loading, refresh, preview, selection, save, apply, stale, and error states. Surface-specific CSS may adjust the outer size/radius/padding, but it will not fork Paper behavior or add the shell's tool rail, navigation, footer, preferences, about, or other plugins.

The window will hide when it loses focus, unless a Paper operation temporarily transfers focus to an operating-system dialog. Reopening reloads/refreshes through the existing Paper lifecycle and persisted Rust cache.

An alternative was to reuse the tray window and hide its navigation conditionally. A separate label gives Paper independent placement and lifecycle without making the general tray shell depend on its launch source.

### 4. Paper placement uses the clicked cell and clamps to the active display

The status-bar geometry helper will calculate the Paper cell rectangle using the same ordered visible item IDs and equal-cell subdivision used by hit testing. The Paper window's preferred horizontal center is the Paper cell center; its preferred top edge is immediately below the menu bar/status-item rectangle with a small visual gap. The final physical position will be clamped to the active monitor's usable bounds so the whole window remains visible near left/right screen edges and on displays with different scale factors.

If native geometry or monitor data is unavailable, Zero will fall back to the existing tray-positioner placement rather than failing to open Paper. Positioning failures return an error and do not claim the surface was shown.

An alternative was `Position::TrayCenter` alone. Because Zero's glyphs are cells inside one grouped native item, that centers under the whole group rather than the clicked Paper glyph.

### 5. Transient surfaces coordinate without changing persistent settings

Clicking a hidden Paper glyph shows and focuses Paper; clicking the same glyph while Paper is visible hides it. Showing Paper hides the tray panel and Launch. Showing Launch hides the tray panel and Paper. Escape and focus loss dismiss each dedicated surface without disabling the plugin, changing tool visibility, or modifying `pluginItemsCollapsed`.

The main window can continue to display Paper and Launch panels through normal navigation; those panel views share backend state but do not need synchronized transient UI state such as the current query or selected wallpaper index.

## Risks / Trade-offs

- **[Virtual glyph geometry can drift from native pixel geometry]** → Use one ordered-cell calculation for hit testing and anchor derivation, include unit tests for cell rectangles, and manually inspect real `SystemUIServer` placement.
- **[Mixed DPI or a screen edge can place Paper partially off-screen]** → calculate in physical coordinates, resolve the clicked monitor, clamp to usable bounds, and cover representative pure geometry cases.
- **[Blur dismissal can fire during a save dialog or other OS-owned interaction]** → suppress dismissal while an operation is actively handing focus to an OS surface and verify download/apply flows manually.
- **[Two React mounts can issue overlapping Paper refreshes]** → retain Rust-owned cache/state safety and make each hook ignore stale/disposed async completions; no new in-memory data contract is introduced.
- **[Changing generic `open-plugin` behavior could break fallback platforms or market plugins]** → specialize only the native macOS resolved-click path; keep the serialized action and fallback command behavior stable, with regression tests.
- **[Automated tests cannot prove menu-bar positioning and focus]** → keep macOS multi-monitor, repeated-click, blur, Escape, collapse/expand, and right-click smoke checks as explicit completion gates.

## Migration Plan

1. Add pure activation-target and cell-geometry tests before changing runtime dispatch.
2. Add the Paper window host, `paper` React route, capability label, and shared panel wrapper.
3. Route native Launch and Paper clicks to their dedicated surfaces while preserving fallback action routing.
4. Run focused and full automated gates, then manually verify the real macOS status bar.
5. Roll back by restoring generic `open-plugin` dispatch and removing the `paper` route/window; no persisted data or settings migration is required.

## Open Questions

- No product decision is blocking implementation. Exact Paper window dimensions and visual gap should be tuned against the existing panel during the implementation smoke test without altering the interaction contract above.
