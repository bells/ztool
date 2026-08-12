## Why

Zero's macOS status bar now exposes the primary app icon and four direct tool icons, but each native item uses the system's variable width, making the group consume more menu-bar space than its compact toolbox role warrants. Users also need a persistent way to collapse the four tool items without disabling the tools or losing the primary Zero entry point.

## What Changes

- Render the primary Zero icon and visible tool icons as cells inside one macOS native status item, using a shared 22pt cell width selected after real-device comparison with the initial 24pt value.
- Add a persisted collapsed state for the status-bar tool group; collapsing shrinks the grouped item to the primary Ø cell, and expanding grows the same native item leftward so the tools stay next to a primary icon positioned anywhere in the menu bar.
- Add per-cell native right-click menus: the primary Ø cell switches between “Collapse Tool Icons” and “Expand Tool Icons” and can quit Zero, while tool cells expose the existing quit action. Preserve primary and tool left-click behavior through horizontal cell routing.
- Reapply the grouped template image and compact or collapsed length after startup, settings changes, plugin lifecycle refreshes, and stateful tool-icon refreshes.
- Keep Windows and other non-macOS platforms on the existing single-primary-icon fallback with tool actions in the tray quick panel; they do not create or collapse separate native tool icons.

## Capabilities

### New Capabilities

- `compact-status-bar-items`: Defines compact macOS native item widths, persistent tool-group collapse/expand behavior, the primary icon's right-click control, and the unchanged non-macOS fallback.

### Modified Capabilities

None.

## Impact

- Rust/Tauri status-bar state, persistence, refresh, native tray creation, and primary-item menu routing in `src-tauri/src/services/status_bar.rs` and its app-shell integration.
- Symmetric Rust and TypeScript status-bar settings contracts, normalization helpers, services, and tests for the persisted collapsed state.
- macOS-only native integration for accessing `NSStatusItem.length`, grouped template-image composition, and per-cell click routing without affecting Windows builds.
- Native and frontend localized labels for collapse/expand where the control is surfaced.
- Verification requires OpenSpec validation, TypeScript and Rust regression tests, cross-platform compile checks, and real macOS menu-bar inspection of the selected 22pt width.
