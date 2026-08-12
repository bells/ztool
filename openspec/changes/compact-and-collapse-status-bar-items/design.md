## Context

Zero previously created one primary Tauri tray item and, on macOS only, four plugin-owned native status items. The original compact/collapse implementation hid those independent items. Real-device verification showed that when the user moved the only visible primary Ø while collapsed, the hidden tool items did not move with it and expansion restored them at the menu-bar leading edge. AppKit exposes no public API for grouping or positioning independent status items relative to another item.

Status-bar preferences already live in the Rust-owned `status-bar.json` so they are available before React starts. Rust `StatusBarSettings` and `UpdateStatusBarSettingsInput` are mirrored by TypeScript settings contracts. Windows and other non-macOS platforms deliberately use `FallbackActionRow` and create only the primary native tray icon.

Tauri 2.11 exposes `TrayIcon::with_inner_tray_icon`, and its macOS inner tray type exposes the underlying `NSStatusItem`. This provides a narrow public seam for setting `NSStatusItem.length` without replacing the existing Tauri event and resource ownership model.

## Goals / Non-Goals

**Goals:**

- Reduce the macOS native group width by rendering the primary and expanded tools as 22pt cells inside one status item after the initial 24pt value proved too spacious on a real menu bar.
- Collapse and expand by resizing the same native item so the group remains adjacent wherever the user positions its primary Ø cell.
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

### Decision 1: Render one grouped native status item with compact cells

Define one macOS-only compact-width constant:

```rust
const MACOS_COMPACT_STATUS_ITEM_LENGTH: f64 = 22.0;
```

Compose the visible 18×18 template assets into one transparent horizontal image with a 22px source cell for each icon and keep the primary Ø in the rightmost cell. Use `with_inner_tray_icon` and `ns_status_item()` on the main thread to set the native length to `22pt × cell count`. The single native item therefore grows leftward from its primary cell and retains its system-managed position. Keep the compact cell width in one constant so tuning remains a one-line adjustment.

Use the existing Tauri/tray-icon native types through inferred handles rather than introducing a custom AppKit status-item owner. Because Tauri documents that the inner tray type can change across minor releases, pin Tauri to the current 2.11 minor while this seam is used.

Alternatives considered:

- Shrink the PNG canvases: all tray images are already 18×18 and Tauri still allocates a variable-width native item, so this does not control the click slot.
- Build a custom AppKit view hierarchy: this would require custom accessibility, drawing, event tracking, and ownership. The selected grouped Tauri item instead uses a template image plus Tauri click geometry, preserving the standard status item and its native ownership.
- Use private Cocoa pointers or fork `tray-icon`: both widen maintenance and compatibility risk without adding product value.

### Decision 2: Collapse by resizing the grouped item

Keep one Tauri tray resource registered for the app lifetime. Collapsing replaces its image with Ø and sets the length to one cell; expanding replaces the image with the ordered tools plus Ø and grows it to the full cell count. Settings refreshes, plugin lifecycle changes, and Awake state changes update this same resource in place, preventing native ordering drift.

Tauri events include both cursor coordinates and the grouped item's physical rect. A pure resolver maps the horizontal position to a cell: tool cells route through their existing approved action, while the rightmost Ø cell toggles the tray panel and supplies a narrowed Ø-cell rect to the positioner so the panel remains anchored under the primary icon. Right-click disables automatic whole-item menus, selects the clicked cell's native menu, and opens it manually.

Alternatives considered:

- Hide or remove separate tool items: they cannot follow the primary while hidden and reappear at the wrong edge after the user moves Ø.
- `NSStatusItem.autosaveName`: Apple documents it for persisted visibility, not relative grouping or positioning.
- Inflate a separator like Hidden Bar: screen-width geometry would introduce multi-display and notch failure modes without making independent items follow Ø.

### Decision 3: Add collapse state to the existing settings contract

Add `plugin_items_collapsed: bool` to Rust `StatusBarSettings`, serialized as `pluginItemsCollapsed`, and `plugin_items_collapsed: Option<bool>` to the update input. Add the matching `pluginItemsCollapsed` fields to the TypeScript snapshot, update input, default, normalization, and optimistic update helpers.

The Rust field uses a serde field default so an older valid settings file keeps `enabled`, `showPluginItemsOnLaunch`, and `visiblePluginItems` rather than failing the entire parse. The default is `false` (expanded), preserving current behavior. Reuse `update_status_bar_settings`; no new IPC command is required.

The native menu action calls a focused Rust toggle service that reads the current snapshot, applies the inverted grouped image and length, and writes the inverted field through the same persistence path. If persistence fails, the service restores the previous layout. React does not own the collapse transition, but its contract remains symmetric for settings reads, previews, and future controls.

Alternatives considered:

- Store collapse state only in memory: restart would not preserve the user's choice.
- Add a separate file or frontend localStorage key: native startup would either duplicate persistence or wait for React, causing layout flicker.
- Reuse `showPluginItemsOnLaunch`: that setting controls whether plugin items participate at all and must remain distinct from temporary group layout.

### Decision 4: Use a macOS-only native right-click menu

Build the existing Tauri native primary and tool menus, but attach them dynamically to the single grouped item according to the right-clicked cell. Tool quit items keep per-tool menu ids, and all quit actions route to clean application exit. Keep `show_menu_on_left_click(false)` and disable automatic right-click menu presentation so the Rust event router can select the correct cell menu before showing it. The collapse callback dispatches asynchronously and updates the grouped image, length, and primary menu label in place.

Resolve the menu copy from the native system language with focused Chinese and English strings; no new frontend preference or IPC language field is introduced. Windows and other fallback platforms receive no collapse item because they have no separate native tool items.

Alternatives considered:

- Repurpose primary left click: this would break the existing primary tray-panel requirement and reduce access to the fallback tool surface.
- Use double-click or modifier-click: these are less discoverable and can conflict with the existing click debounce.
- Add a sixth chevron status item: this consumes more menu-bar space and contradicts the compaction goal.

### Decision 5: Separate platform layout policy from shared settings normalization

Keep `StatusBarSupport::NativeMultiItem` as the existing macOS capability name, but implement it as one grouped native item. Pure helpers resolve ordered cell ids, grouped width, and click targets; image composition and AppKit length application stay behind `#[cfg(target_os = "macos")]`. On fallback platforms, `pluginItemsCollapsed` remains parseable but does not filter or hide the quick-panel action row.

This preserves one settings schema across platforms while keeping platform-native behavior explicit and testable. Windows does not create, remove, hide, or restore four extra tray icons as part of this change.

## Risks / Trade-offs

- [Risk] A grouped image loses separate native status-item semantics → Mitigation: keep 22pt hit cells, route every existing left-click action, select per-cell right-click menus, and preserve the standard Tauri status-item owner.
- [Risk] The tray panel anchors under the center of the full group → Mitigation: pass only the rightmost Ø cell rect to the existing positioner before toggling the panel.
- [Risk] Tauri's inner tray type changes in a future minor release → Mitigation: pin the Tauri 2.11 minor, isolate native access in one macOS helper, and cover compilation in the macOS gate.
- [Risk] Updating image and length can briefly mismatch → Mitigation: update the grouped template image immediately before its resolved length and verify for flicker in `pnpm tauri dev`.
- [Risk] Rebuilding the primary item can lose its user-chosen position → Mitigation: refresh the existing grouped tray resource in place and only create it when absent.
- [Risk] Adding a required boolean breaks old JSON settings → Mitigation: use serde's per-field default and test migration from a pre-change settings fixture.
- [Trade-off] The grouped item requires coordinate-based action routing → The mapping is a pure tested function, while native actions remain in the existing Rust host service.

## Migration Plan

1. Extend Rust and TypeScript status-bar settings contracts with an expanded-by-default collapsed field and migration tests.
2. Add pure state/menu-label/grouped-cell width and hit-routing helpers with focused tests.
3. Add the macOS native-length helper and pin the compatible Tauri minor version.
4. Attach per-cell macOS right-click menus, route collapse through existing settings persistence plus grouped image/length updates, and route quit items to clean application exit.
5. Reapply grouped image and length at every native refresh and verify fallback platforms remain single-icon.
6. Run automated gates, then manually verify the selected 22pt width plus collapse, restart restoration, left click, tool actions, and Awake icon refresh on macOS.

Rollback is straightforward: remove the native-length/menu integration and collapsed field while leaving the existing status-bar settings and multi-item refresh behavior intact. A settings file containing the extra camelCase field remains forward-compatible with older serde deserialization because unknown fields are ignored.

## Open Questions

- None. The product interaction, selected 22pt width, persisted state, and macOS-only native scope are settled by this proposal.
