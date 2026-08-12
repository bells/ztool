## Why

Zero Launch and Zero Paper already have dedicated status-bar glyphs, but clicking either glyph currently opens the general Zero window and navigates to the plugin. That extra shell and navigation step conflicts with the purpose of independent tool glyphs: Launch should appear immediately like its global shortcut, while Paper should open a focused surface anchored to its own glyph.

## What Changes

- Make a direct click on the Zero Launch status-bar glyph show the existing centered Zero Launch floating window, with the same focus, reset, dismissal, and indexing behavior as `CommandOrControl+Shift+Space`.
- Add a dedicated Zero Paper popover-style window positioned below the clicked Paper glyph.
- Render only the existing Zero Paper wallpaper content in that window; omit the general tool list, plugin navigation, preferences, about, and other tool content.
- Reuse the current Paper hook, services, typed IPC contracts, cached data, and actions so the dedicated surface does not create a second wallpaper implementation.
- Keep the primary Zero glyph, status-bar collapse/expand behavior, tool visibility preferences, right-click quit behavior, and non-macOS fallback action row unchanged.
- Define safe repeated-click, focus, blur, and cross-surface behavior for the two tool windows.

## Capabilities

### New Capabilities

- `status-bar-tool-surfaces`: Defines tool-specific status-bar activation and the focused Launch and Paper window behaviors.

### Modified Capabilities

- None.

## Impact

- **Rust/Tauri:** status-bar action dispatch, Launch window entry-point reuse, a host-owned Paper window and tray-relative positioning, window visibility/focus coordination, and tests for action routing/window options.
- **React/TypeScript:** window-label routing for a Paper-only app surface, a small wrapper around the existing `BingWallpaperPanel`, shared localization, focused surface styles, and pure routing tests.
- **Configuration:** Tauri capability window labels must include the Paper window.
- **Platform:** the anchored Paper surface targets the native macOS multi-item status bar shown in the request. Existing Windows/Linux single-tray fallback behavior remains unchanged and continues to open tools through the current fallback path.
- **Verification:** automated Rust, TypeScript, build, and OpenSpec checks plus manual macOS `SystemUIServer` smoke testing for exact glyph targeting, screen-edge placement, focus/dismissal, repeated clicks, and collapse/expand compatibility.
