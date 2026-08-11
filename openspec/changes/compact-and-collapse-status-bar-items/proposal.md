## Why

Zero's macOS status bar now exposes the primary app icon and four direct tool icons, but each native item uses the system's variable width, making the group consume more menu-bar space than its compact toolbox role warrants. Users also need a persistent way to collapse the four tool items without disabling the tools or losing the primary Zero entry point.

## What Changes

- Give the primary Zero item and visible tool items a shared macOS compact width of 22pt, selected after real-device comparison with the initial 24pt value and defined by one native constant.
- Add a persisted collapsed state for the status-bar tool group; collapsing hides the four macOS tool status items so they reserve no menu-bar space, and expanding restores them at the configured compact width.
- Add a native right-click menu on the primary Ø item whose first action switches between “Collapse Tool Icons” and “Expand Tool Icons” and whose second action quits Zero, while preserving the existing primary left-click tray-panel behavior. Give every macOS tool item a one-action right-click menu whose first item also quits Zero.
- Reapply compact or collapsed lengths after startup, settings changes, plugin lifecycle refreshes, and stateful tool-icon refreshes.
- Keep Windows and other non-macOS platforms on the existing single-primary-icon fallback with tool actions in the tray quick panel; they do not create or collapse separate native tool icons.

## Capabilities

### New Capabilities

- `compact-status-bar-items`: Defines compact macOS native item widths, persistent tool-group collapse/expand behavior, the primary icon's right-click control, and the unchanged non-macOS fallback.

### Modified Capabilities

None.

## Impact

- Rust/Tauri status-bar state, persistence, refresh, native tray creation, and primary-item menu routing in `src-tauri/src/services/status_bar.rs` and its app-shell integration.
- Symmetric Rust and TypeScript status-bar settings contracts, normalization helpers, services, and tests for the persisted collapsed state.
- macOS-only native integration for accessing `NSStatusItem.length`, using a target-specific dependency or existing safe Tauri native handle seam without affecting Windows builds.
- Native and frontend localized labels for collapse/expand where the control is surfaced.
- Verification requires OpenSpec validation, TypeScript and Rust regression tests, cross-platform compile checks, and real macOS menu-bar inspection of the selected 22pt width.
