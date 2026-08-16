## Context

The preferences WebView is a dedicated Tauri window created at 520×620. `PreferencesWindowApp` currently renders `PreferencesPanel` and `PluginManagerPanel` as two sibling flex panels. Each panel receives half of the remaining height, while the shared `system-panel` style suppresses overflow. The preferences panel itself mixes launch-at-login, language, tool navigation visibility, status-bar global settings, status-bar preview, per-tool status-bar visibility, and global success/error messages. The extension manager adds market, local install, permission approval, lifecycle, restore, and diagnostic workflows below it.

The state behind those controls intentionally has multiple owners:

- application language and navigation visibility use `zero.preferences.v1` in localStorage;
- login startup uses the official Tauri autostart plugin;
- enabled/disabled plugin lifecycle uses the Rust plugin registry;
- native status-bar display uses Rust-owned `status-bar.json` state;
- global shortcut registration is native and currently defined in Rust.

Those ownership boundaries are valid. The problem is the presentation and orchestration layer, not the absence of one universal settings store. The change therefore reorganizes the dedicated preferences surface without migrating unrelated state or introducing a generic plugin-settings engine.

## Goals / Non-Goals

**Goals:**

- Provide a calm, compact two-pane settings center with predictable categories and one scrolling content region.
- Keep application, plugin lifecycle, navigation visibility, status-bar visibility, and shortcut registration concepts visibly distinct.
- Make settings searchable by localized title, description, category, and tool name.
- Reuse existing state owners and immediate-apply flows while providing control-local pending, saved, and error feedback.
- Give every installed tool a stable preferences destination that can grow later without implementing tool-specific persistence now.
- Expose the existing status-bar collapsed state and keep an open preferences window synchronized with native status-bar changes.
- Display global shortcut registration from a typed native snapshot without allowing edits.
- Preserve desktop keyboard accessibility, bilingual copy, compact responsive behavior, and native window conventions.

**Non-Goals:**

- A generic renderer or persistent store for manifest `contributes.settings` declarations.
- Editable global shortcuts, conflict reassignment, or user-defined shortcut scopes.
- Theme, appearance density, updater, account, cloud sync, wallpaper scheduling, download-location, screenshot-format, or launcher-root settings.
- Moving About, update, or quit workflows into preferences.
- Changing plugin package validation, permission approval semantics, extension isolation, or native tool behavior.
- Changing the tray, main, about, capture, pin, launcher, or paper window lifecycle.

## Decisions

### Decision 1: Use a typed in-window destination model instead of adding a router

Add a preferences destination model under `src/core/preferences/` with stable identifiers for `general`, `status-bar`, `shortcuts`, `tools`, `tool:<plugin-name>`, and `extensions`. `PreferencesWindowApp` owns the selected destination and composes focused page components. The default destination is General. Tool destinations are derived from plugin records and localized bundled presentation metadata rather than hard-coded page branches for each bundled tool.

The preferences window does not need URL history or deep links yet, so React Router or another routing dependency would add lifecycle and bundle complexity without user value. Stable destination IDs still make navigation and search behavior testable and leave room for later deep-link handling through a small host event.

Alternative considered: keep one long page with anchor links. This preserves the current DOM shape but continues to couple unrelated loading/error states, makes extension workflows dominate the page, and scales poorly as tools grow.

### Decision 2: Build a pure localized search index over setting descriptors

Represent discoverable content as typed setting descriptors containing a stable setting ID, destination ID, localized title, localized description, optional tool name, and search keywords. A pure model builds and filters this index. Static application settings and dynamic tool records contribute descriptors; extension market entries do not become search results because they are changing data rather than preferences.

While the query is empty, the sidebar shows the category tree. While it is non-empty, the sidebar shows matching results with their destination path. Activating a result selects the destination, scrolls the corresponding setting row into view, and moves keyboard focus to that row or its primary control. Escape clears search. Search matching is case-insensitive and treats Chinese text and Latin text as ordinary localized strings; no fuzzy-search dependency is added.

Alternative considered: filter only the currently visible page. That would make settings in other tools undiscoverable and would not provide the VS Code-like cross-category behavior requested for a growing plugin toolbox.

### Decision 3: Keep existing sources of truth and add a thin page orchestration layer

The settings center reads and mutates each existing owner through its current hook/service:

- `usePreferences` remains responsible for language, launch-at-login orchestration, and tool navigation visibility.
- `usePluginHost` remains responsible for plugin enabled state and lifecycle operations.
- `useStatusBar` remains responsible for status-bar display state and item actions.
- the extension manager continues to use plugin host and market services.

Page components receive narrow typed controllers instead of one merged mutable settings object. No code copies plugin lifecycle state into localStorage or status-bar state into plugin manifests. Disabling a plugin preserves its stored navigation and status-bar visibility preferences so reenabling it restores the user's prior choices. Dependent controls remain visible but disabled with explanatory copy.

Alternative considered: migrate every preference into one Rust settings file. Native startup would benefit from one store, but this change does not justify a broad migration of stable localStorage and registry behavior, and such a migration would materially increase rollback and compatibility risk.

### Decision 4: Treat each tool destination as a host-owned summary in this release

Each tool page shows the tool name, version/source/health, and the host controls that apply:

- enabled/disabled lifecycle state;
- visible/hidden in Zero navigation;
- visible/hidden in the status bar when the tool contributes a status-bar item;
- read-only global shortcut information when the tool owns one;
- read-only platform/support metadata already available through host records or shortcut snapshots.

The Tools overview provides the same high-level visibility state for scanning, but it does not duplicate extension install/uninstall actions. Destructive uninstall and permission workflows remain in Extensions. The three similarly named controls use explicit labels and descriptions and never collapse into one ambiguous switch.

The existing manifest `contributes.settings` field is not rendered because its current contract lacks descriptions, localized options, constraints, platform rules, persistence scope, and a host storage API. Zero Awake's declared `durationMinutes` remains metadata only until a separate tool-settings capability defines those missing contracts.

Alternative considered: special-case settings for all four bundled tools now. That would introduce four unrelated backend/storage efforts into a layout change and create a UI contract that third-party extensions could not follow.

### Decision 5: Make status-bar collapse an immediate setting and synchronize native changes

Expose `pluginItemsCollapsed` through `useStatusBar.setPluginItemsCollapsed`. The Status Bar page presents it as the current collapsed/expanded tool-icon state, alongside the existing global display, launch restoration, arrangement preview, and per-tool visibility controls.

Rust emits a `status-bar-settings-updated` event with the saved `StatusBarSettings` snapshot after both command-driven updates and the native primary-item collapse/expand action. `useStatusBar` listens for the event, replaces its local snapshot, and reloads item snapshots when necessary. This prevents the preferences window from showing a stale collapse value after the user operates the native menu-bar item.

The update path remains optimistic for preference controls: apply locally, call the existing command, then confirm with the returned snapshot; on failure restore the previous snapshot and show local error feedback.

Alternative considered: reload only when the preferences window gains focus. That misses changes made while both the preferences window and menu bar are visible and makes the preview feel unreliable.

### Decision 6: Add a read-only native shortcut snapshot contract

Add symmetric Rust and TypeScript contracts for a shortcut snapshot with stable action ID, accelerator, owning plugin name, enabled state, registered state, platform support, and optional diagnostic code. Add a thin `get_global_shortcut_snapshots` Tauri command that reads the same Rust constants and native global-shortcut manager used for registration.

The Shortcuts page and relevant tool pages render these snapshots as read-only rows. They do not expose editable inputs or an update command. A disabled plugin may show its shortcut as inactive; a registration conflict or unsupported platform is represented as status copy instead of presenting a false active shortcut.

Alternative considered: duplicate shortcut strings in frontend metadata. Zero Snap already has frontend display metadata, but native registration remains authoritative and duplicate constants can drift. A read-only command is a narrow way to preserve Rust/TypeScript symmetry without designing editable shortcuts.

### Decision 7: Refactor extension management into a localized page, not a nested settings card

Refactor `PluginManagerPanel` into focused sections rendered by the Extensions destination: market, local package, permission review, installed plugins, restore defaults, and diagnostics. Preserve existing service calls and approval behavior. Move all visible strings into the preferences localization bundle or a host-local extension-management bundle with Chinese and English coverage.

Permission review remains inline in the Extensions page rather than becoming a generic modal. Pending install state stays visible next to the operation that created it, and destructive uninstall actions retain explicit labels and disabled/busy states.

Alternative considered: retain the manager below General and hide it behind collapsible cards. That reduces initial height but does not create a stable place for extension lifecycle workflows and continues to mix app preferences with plugin administration.

### Decision 8: Use one responsive content shell with grouped rows and native window chrome

Increase the preferences default size to approximately 840×640 while retaining a minimum width near the current compact window. At regular widths, use a roughly 200px fixed sidebar and a flexible content pane. Below the two-pane breakpoint, show either the navigation/search view or the selected page with a localized Back to Settings control; do not squeeze both columns into an unreadable layout.

Only the content pane scrolls at regular widths. Settings within a section use one restrained group surface with dividers rather than an individual card around every row. The window continues to use Tauri/native decorations and platform window controls. Interactive rows keep visible focus, at least 44px practical targets, accessible names and descriptions, `aria-live` feedback, and reduced-motion behavior.

Alternative considered: raise the minimum width enough to prohibit a compact layout. That would be simpler but would regress users who intentionally keep the resizable preferences window narrow and would weaken the reusable/mobile-friendly content structure.

### Decision 9: Replace global permanent save state with keyed operation feedback

Remove the permanent Saved badge and page-bottom success messages. Each asynchronous controller exposes or maps to a stable operation key and one of `idle`, `pending`, `saved`, or `error`. Pending state disables only the affected control when safe; success is announced briefly through a nearby polite live region; errors remain visible beside the relevant group until the next attempt or dismissal.

Language and tool-navigation changes continue to persist immediately. Autostart, plugin lifecycle, status-bar, market, and shortcut loading failures are not collapsed into one page-level string. The UI must not claim success before the authoritative owner confirms an asynchronous update.

Alternative considered: a global toast for every change. Frequent immediate-apply settings would create noisy, context-free feedback, especially for keyboard and screen-reader users.

## Risks / Trade-offs

- [Risk] The new shell becomes visually larger than Zero's compact product character. → Keep density restrained, use one system font and grouped rows, cap content line length, and avoid marketing-sized headings or decorative cards.
- [Risk] Search descriptors drift from rendered settings. → Define descriptors next to page definitions, use stable setting IDs in both descriptor and DOM output, and add coverage that every searchable ID has a destination and rendered target.
- [Risk] Multiple state owners produce inconsistent dependent controls. → Compose narrow controllers in `PreferencesWindowApp`, document each source of truth, preserve inactive preferences, and test enabled/navigation/status-bar combinations independently.
- [Risk] Native status-bar events race optimistic command updates. → Treat the returned or emitted Rust snapshot as authoritative, make replacement idempotent, and reload item snapshots after revisions that affect preview composition.
- [Risk] A shortcut appears active although native registration failed. → Derive displayed registration state from the native shortcut manager and diagnostics, not frontend constants.
- [Risk] Extension refactoring accidentally changes security or lifecycle behavior. → Reuse existing service functions and pending-install approval model; test the same permission, enable/disable, uninstall, and restore transitions after moving presentation components.
- [Risk] The narrow responsive view traps keyboard focus or loses the selected page. → Keep navigation and content as ordinary document regions, focus the page heading after navigation, provide a visible Back control, and test resize plus keyboard traversal.
- [Trade-off] Tool pages initially contain mostly host controls and metadata. → This provides a stable information architecture now while deliberately deferring an under-specified generic tool-settings system.

## Migration Plan

1. Add pure destination/search models and localized copy without changing the current rendered entry point.
2. Add the native shortcut snapshot contract/command and status-bar settings event, with Rust and frontend service tests.
3. Build the new settings shell and focused General, Status Bar, Shortcuts, Tools, Tool Detail, and Extensions pages behind the existing preferences window label.
4. Connect existing controllers, migrate extension presentation into the Extensions page, and remove the old stacked composition only after parity tests pass.
5. Update the preferences window default size and verify regular and narrow layouts in the real Tauri window.
6. Preserve existing localStorage, plugin registry, autostart, and status-bar files unchanged; no data migration is required.

Rollback consists of restoring the previous `PreferencesWindowApp` composition and window size. The added read-only shortcut command and status-bar event are backward-compatible and can remain unused or be removed without migrating user data.

## Open Questions

- No question blocks implementation of this change. The future schema and persistence model for tool-declared settings should be proposed separately after this settings-center structure has shipped and the four bundled tools' actual configuration needs are prioritized.
