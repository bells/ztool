## Context

Zero currently creates one primary Tauri tray item and, on macOS only, four plugin-owned native status items. The `tray-icon` macOS backend creates each item with `NSVariableStatusItemLength`, so the system assigns more horizontal space than the 18×18 template images require. Every status-bar refresh removes and rebuilds the native items in deterministic reverse creation order; this happens at startup, after settings and plugin lifecycle changes, and when the Awake icon changes state.

Status-bar preferences already live in the Rust-owned `status-bar.json` so they are available before React starts. Rust `StatusBarSettings` and `UpdateStatusBarSettingsInput` are mirrored by TypeScript settings contracts. Windows and other non-macOS platforms deliberately use `FallbackActionRow` and create only the primary native tray icon.

Tauri 2.11 exposes `TrayIcon::with_inner_tray_icon`, and its macOS inner tray type exposes the underlying `NSStatusItem`. This provides a narrow public seam for setting `NSStatusItem.length` without replacing the existing Tauri event and resource ownership model.

## Goals / Non-Goals

**Goals:**

- Reduce the macOS native group width by applying one 22pt compact width to the primary and expanded tool items after the initial 24pt value proved too spacious on a real menu bar.
- Collapse tool items by hiding their native status items so no blank slots remain, then restore them to the compact width in deterministic order.
- Persist collapse state in the existing Rust-owned settings file and migrate older files without losing their other settings.
- Preserve primary left-click behavior and add a state-aware collapse/expand command to its right-click menu.
- Keep the TypeScript and Rust IPC contracts symmetric and test the state transition independently of AppKit.
- Preserve the Windows/Linux single-icon fallback and its action row.

**Non-Goals:**

- Controlling, moving, or hiding tray/menu-bar icons owned by other applications.
- Providing arbitrary per-icon spacing, user drag ordering, or a user-facing width slider.
- Creating separate tool tray icons on Windows or Linux.
- Replacing the primary Ø icon, tool icon artwork, tool actions, or tray quick-panel flow.

## Decisions

### Decision 1: Apply a single macOS width through Tauri's inner tray handle

Define one macOS-only compact-width constant:

```rust
const MACOS_COMPACT_STATUS_ITEM_LENGTH: f64 = 22.0;
```

After each `TrayIconBuilder::build`, use `with_inner_tray_icon` and `ns_status_item()` on the main thread to call `NSStatusItem.setLength`. The primary and every expanded tool item receive the compact width. Collapsed tool items are hidden through Tauri's native visibility API instead of receiving a synthetic width. Keep the compact width in one constant so a manual 22pt comparison is a one-line adjustment.

Use the existing Tauri/tray-icon native types through inferred handles rather than introducing a custom AppKit status-item owner. Because Tauri documents that the inner tray type can change across minor releases, pin Tauri to the current 2.11 minor while this seam is used.

Alternatives considered:

- Shrink the PNG canvases: all tray images are already 18×18 and Tauri still allocates a variable-width native item, so this does not control the click slot.
- Build one composite custom `NSStatusItem`: this would require custom hit testing, accessibility, tooltips, menus, and action routing for five regions.
- Use private Cocoa pointers or fork `tray-icon`: both widen maintenance and compatibility risk without adding product value.

### Decision 2: Collapse by hiding tool status items

Keep every eligible Tauri tray resource registered, call `set_visible(false)` on its macOS status item while collapsed, and call `set_visible(true)` plus the compact length while expanded. Real-device verification showed that `NSStatusItem.length = 0` hides the glyph but can retain a blank slot on the tested macOS version; native visibility releases that space completely.

macOS recreates an `NSStatusItem` when visibility returns and inserts it at the leading edge. Restore tools in deterministic creation order, but never hide or reinsert the primary Ø item during this transition. Real-device testing showed that a primary visibility round trip can fail to restore the Ø item. Preserving the recovery entry point therefore takes precedence over forcing the pre-collapse visual order.

The existing refresh path may continue rebuilding the status items for startup, Awake state updates, visibility changes, enable/disable, install/uninstall, and restore-defaults. It must read the stored collapse state before construction and apply the correct length immediately to every rebuilt item. The primary menu action is different: because its callback is owned by the current primary status item, collapse/expand must update the existing primary and tool handles in place rather than removing and rebuilding the callback owner.

Alternatives considered:

- `NSStatusItem.length = 0`: this was the initial implementation, but real-device verification exposed persistent blank slots after collapse.
- Inflate a separator like Hidden Bar: Zero owns its tool items and can collapse them directly; screen-width geometry would introduce multi-display and notch failure modes unnecessarily.

### Decision 3: Add collapse state to the existing settings contract

Add `plugin_items_collapsed: bool` to Rust `StatusBarSettings`, serialized as `pluginItemsCollapsed`, and `plugin_items_collapsed: Option<bool>` to the update input. Add the matching `pluginItemsCollapsed` fields to the TypeScript snapshot, update input, default, normalization, and optimistic update helpers.

The Rust field uses a serde field default so an older valid settings file keeps `enabled`, `showPluginItemsOnLaunch`, and `visiblePluginItems` rather than failing the entire parse. The default is `false` (expanded), preserving current behavior. Reuse `update_status_bar_settings`; no new IPC command is required.

The native menu action calls a focused Rust toggle service that reads the current snapshot, applies the inverted layout to the existing native items, and writes the inverted field through the same persistence path. If persistence fails, the service restores the previous layout. React does not own the collapse transition, but its contract remains symmetric for settings reads, previews, and future controls.

Alternatives considered:

- Store collapse state only in memory: restart would not preserve the user's choice.
- Add a separate file or frontend localStorage key: native startup would either duplicate persistence or wait for React, causing layout flicker.
- Reuse `showPluginItemsOnLaunch`: that setting controls whether plugin items participate at all and must remain distinct from temporary group layout.

### Decision 4: Use a macOS-only native right-click menu

Build a Tauri native menu for the primary item on macOS. Its first item has a stable id and state-aware localized text for collapse or expand; its second item has a separate stable id and localized text for quitting Zero. Every tool item receives a one-item native menu whose first action uses the same localized quit label. Tool quit items use per-tool menu ids so Tauri's global menu event dispatch invokes the clean exit path only from the matching tool callback. Keep `show_menu_on_left_click(false)` on every item so existing left-click actions remain unchanged. The collapse callback must return before mutating the current menu, so it dispatches the toggle service asynchronously; that service retains and never hides the primary tray resource, updates only tool visibility, and replaces the attached primary menu to refresh its label. All quit actions route to the existing clean application-exit behavior.

Resolve the menu copy from the native system language with focused Chinese and English strings; no new frontend preference or IPC language field is introduced. Windows and other fallback platforms receive no collapse item because they have no separate native tool items.

Alternatives considered:

- Repurpose primary left click: this would break the existing primary tray-panel requirement and reduce access to the fallback tool surface.
- Use double-click or modifier-click: these are less discoverable and can conflict with the existing click debounce.
- Add a sixth chevron status item: this consumes more menu-bar space and contradicts the compaction goal.

### Decision 5: Separate platform layout policy from shared settings normalization

Keep `StatusBarSupport::NativeMultiItem` macOS-only. A pure layout helper resolves native widths from item kind, platform support, and collapse state; AppKit application is behind `#[cfg(target_os = "macos")]`. On fallback platforms, `pluginItemsCollapsed` remains parseable but does not filter or hide the quick-panel action row.

This preserves one settings schema across platforms while keeping platform-native behavior explicit and testable. Windows does not create, remove, hide, or restore four extra tray icons as part of this change.

## Risks / Trade-offs

- [Risk] Separate macOS status items retain system-managed inter-item spacing even at 22pt → Mitigation: keep the width centralized and treat a Sogou-style single composite status item as a separate larger redesign if 22pt remains insufficient.
- [Risk] Restoring hidden tool items can change their relative position because macOS recreates each `NSStatusItem` → Mitigation: restore tools in deterministic order and accept system placement rather than hiding the primary, which could remove the only recovery icon.
- [Risk] Tauri's inner tray type changes in a future minor release → Mitigation: pin the Tauri 2.11 minor, isolate native access in one macOS helper, and cover compilation in the macOS gate.
- [Risk] Refresh rebuilds briefly expose variable-width items before the length is applied → Mitigation: apply the resolved length immediately after each item is built, before the event loop presents the completed group, and verify for visible flicker in `pnpm tauri dev`.
- [Risk] Rebuilding or hiding the primary tray item from its own menu callback can remove the callback owner or fail to restore its native item, leaving no recovery icon → Mitigation: dispatch the toggle after the callback returns, retain the primary Tauri tray resource, and never change its visibility during collapse or expansion.
- [Risk] Adding a required boolean breaks old JSON settings → Mitigation: use serde's per-field default and test migration from a pre-change settings fixture.
- [Trade-off] Expanding recreates native tool status items → This releases all collapsed whitespace at the cost of a brief native reinsertion step whose order must be managed explicitly.

## Migration Plan

1. Extend Rust and TypeScript status-bar settings contracts with an expanded-by-default collapsed field and migration tests.
2. Add pure state/menu-label/native-width/visibility resolution helpers and focused tests.
3. Add the macOS native-length helper and pin the compatible Tauri minor version.
4. Attach the macOS primary right-click menu, route collapse through existing settings persistence plus native visibility updates, and route the second menu item to clean application exit.
5. Reapply layout widths at every native item creation/refresh and verify fallback platforms remain single-icon.
6. Run automated gates, then manually verify the selected 22pt width plus collapse, restart restoration, left click, tool actions, and Awake icon refresh on macOS.

Rollback is straightforward: remove the native-length/menu integration and collapsed field while leaving the existing status-bar settings and multi-item refresh behavior intact. A settings file containing the extra camelCase field remains forward-compatible with older serde deserialization because unknown fields are ignored.

## Open Questions

- None. The product interaction, selected 22pt width, persisted state, and macOS-only native scope are settled by this proposal.
