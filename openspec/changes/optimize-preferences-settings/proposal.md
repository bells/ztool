## Why

Zero's dedicated preferences window now places application preferences, tool visibility, status-bar configuration, and the full extension manager in one narrow vertical surface. As Zero grows beyond four bundled tools, users need a searchable, clearly grouped settings center that preserves the app's compact desktop character without hiding or conflating different kinds of configuration.

## What Changes

- Replace the current stacked preferences layout with a resizable two-pane settings center: fixed search and category navigation on the left, one independently scrolling settings page on the right.
- Organize existing behavior into General, Status Bar, Keyboard Shortcuts, Tools, and Extensions destinations; do not add an empty Appearance destination before Zero has real appearance preferences.
- Keep launch-at-login, language, tool navigation visibility, plugin lifecycle state, and status-bar visibility as distinct controls with explicit copy.
- Move the existing market, local package, permission review, installed-plugin lifecycle, restore, and diagnostics workflows into the Extensions destination instead of rendering them below application preferences.
- Add a Tools overview and per-tool destinations for Zero Snap, Zero Awake, Zero Paper, and Zero Launch. The first release exposes existing host controls and useful read-only runtime metadata without adding new tool-specific settings persistence.
- Expose the persisted status-bar collapsed state in preferences and keep the arrangement preview synchronized with status-bar visibility settings.
- Show registered Zero Snap and Zero Launch global shortcuts as read-only settings, with native registration state as the source of truth. Shortcut editing and conflict reassignment remain out of scope.
- Preserve immediate-apply behavior. Replace the permanently visible "Saved" badge with local pending, success, and error feedback for the setting being changed.
- Localize the complete preferences and extension-management surface in Chinese and English, and keep keyboard navigation, focus visibility, minimum target size, reduced motion, and WCAG AA contrast requirements.
- Increase the default preferences window size to support the two-pane layout while retaining resizing and a compact single-pane fallback at narrow widths.

## Capabilities

### New Capabilities

- `preferences-settings-center`: Searchable preferences navigation, categorized application and tool settings, extension-management placement, immediate-apply feedback, responsive layout, and read-only shortcut/status metadata.

### Modified Capabilities

<!-- No existing main spec requirements change; the new capability adds behavior to the dedicated preferences surface. -->

## Impact

- Frontend app shell and routing: `src/App.tsx`, `src/main.tsx`, and new focused components/models under `src/core/preferences/`.
- Preferences presentation and localization: `src/core/preferences/PreferencesPanel.tsx`, `src/core/preferences/i18n.ts`, `src/App.css`, and `src/core/pluginHost/PluginManagerPanel.tsx`.
- Existing preference sources remain authoritative: `zero.preferences.v1` in localStorage for application/tool navigation preferences, the autostart plugin for login startup, the plugin registry for lifecycle state, and Rust-owned status-bar settings for native item display.
- Status-bar frontend controller/service types gain an immediate-apply control for the existing `pluginItemsCollapsed` field.
- Rust and TypeScript gain a symmetric read-only shortcut snapshot contract and thin command so preferences display native shortcut registration truth without enabling shortcut editing.
- The Tauri preferences window default/minimum size changes in `src-tauri/src/commands/app.rs`; tray, main, about, capture, pin, launcher, and paper window behavior remain unchanged.
- No new third-party runtime dependency, generic plugin-settings engine, appearance system, updater, wallpaper scheduler, or editable global-shortcut workflow is introduced by this change.
