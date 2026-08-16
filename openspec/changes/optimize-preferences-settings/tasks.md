## 1. Preferences Domain Models and Localization

- [x] 1.1 Add typed preferences destination and page-definition models for General, Status Bar, Keyboard Shortcuts, Tools, dynamic `tool:<plugin-name>` entries, and Extensions.
- [x] 1.2 Add pure localized setting-descriptor indexing, search filtering, destination-path formatting, and stable focus-target resolution without introducing a fuzzy-search dependency.
- [x] 1.3 Extend preferences localization with complete Chinese and English navigation, search, setting descriptions, tool metadata, status, empty-state, feedback, and accessibility copy.
- [x] 1.4 Add unit tests for default destination selection, dynamic tool destinations, localized search results, no-results behavior, Escape clearing, and stable setting-to-focus-target mapping.

## 2. Native Shortcut and Status-Bar Synchronization Contracts

- [x] 2.1 Define symmetric Rust and TypeScript read-only global-shortcut snapshot contracts covering stable action ID, owning plugin, accelerator, enabled state, registration state, platform support, and optional diagnostic code.
- [x] 2.2 Add a Rust shortcut snapshot service that derives Zero Snap and Zero Launch state from the existing native constants, plugin registry, global-shortcut manager, and launcher diagnostics.
- [x] 2.3 Add and register a thin `get_global_shortcut_snapshots` Tauri command plus a typed frontend service/hook with explicit error handling.
- [x] 2.4 Add Rust and TypeScript tests for shortcut snapshot serialization, enabled/disabled plugins, active/inactive registration, conflict diagnostics, and unsupported-platform presentation.
- [x] 2.5 Emit a `status-bar-settings-updated` snapshot after command-driven status-bar updates and native collapse/expand actions, including failure-safe behavior that never emits an unpersisted state.
- [x] 2.6 Extend `useStatusBar` with `setPluginItemsCollapsed`, subscribe/unsubscribe handling for native settings events, authoritative snapshot replacement, item refresh, optimistic rollback, and focused controller tests.

## 3. Settings Center Shell and Window Layout

- [x] 3.1 Build focused preferences shell, sidebar, search results, navigation group, content header, and content region components with stable accessible destination state.
- [x] 3.2 Implement regular-width two-pane behavior with a fixed navigation pane and one independently scrolling content pane.
- [x] 3.3 Implement the narrow single-pane navigation/content flow, localized Back to Settings action, destination preservation across resizing, and heading focus after navigation.
- [x] 3.4 Add restrained grouped-row settings styles, dividers, semantic control/status states, visible focus, practical target sizes, WCAG AA text contrast, and reduced-motion behavior without per-row cards.
- [x] 3.5 Update the Tauri preferences window default size to approximately 840×640 while preserving resizing, a compact minimum size, native decorations, platform window controls, and the existing `preferences` label.
- [x] 3.6 Replace the stacked `PreferencesPanel` plus `PluginManagerPanel` composition in `PreferencesWindowApp` with the new shell while leaving other app-window routes and lifecycles unchanged.

## 4. General and Contextual Feedback

- [x] 4.1 Build the General page around the existing language and autostart owners, preserving immediate persistence and removing Save, Cancel, Apply, and OK actions.
- [x] 4.2 Add keyed `idle`/`pending`/`saved`/`error` feedback primitives with nearby polite live regions, control-scoped disabling, transient confirmed success, and persistent actionable errors.
- [x] 4.3 Replace the permanent Saved badge and page-bottom global preference messages with control- or section-local feedback.
- [x] 4.4 Surface the existing at-least-one-visible-tool rejection with localized explanatory feedback instead of silently presenting an accepted hide operation.
- [x] 4.5 Add tests for immediate language changes, autostart success/failure rollback, feedback lifecycle, duplicate-action prevention, and last-visible-tool handling.

## 5. Status Bar, Tools, and Shortcut Pages

- [x] 5.1 Build the Status Bar page with global tool-icon display, launch restoration, collapsed/expanded state, synchronized arrangement preview, and per-tool visibility controls.
- [x] 5.2 Keep dependent status-bar controls visible but disabled with explanatory copy when global tool-icon display, plugin state, platform support, or contribution metadata makes them unavailable.
- [x] 5.3 Build a Tools overview derived from plugin records and localized presentation metadata, showing enabled, navigation-visible, and status-bar-visible states without conflating them.
- [x] 5.4 Build per-tool pages with version/source/health metadata, applicable host controls, read-only shortcut/platform information, and a localized no-additional-settings explanation when appropriate.
- [x] 5.5 Preserve stored navigation and status-bar visibility when a plugin is disabled, restore those choices when it is enabled, and keep uninstall/permission actions out of tool pages.
- [x] 5.6 Build the Keyboard Shortcuts page from native snapshots with read-only accelerator and active/inactive/conflict/unsupported states and no editable or Apply controls.
- [x] 5.7 Add focused tests for status-bar dependency states and native event synchronization, dynamic tool pages, tools without status-bar contributions, disabled-plugin preference preservation, and shortcut state rendering.

## 6. Extensions Page Parity and Localization

- [x] 6.1 Refactor extension management into focused Market, Local Package, Permission Review, Installed Plugins, Restore Defaults, and Diagnostics sections rendered only by the Extensions destination.
- [x] 6.2 Preserve the existing market refresh, local validation, explicit permission approval, install, retry, enable/disable, uninstall, and bundled restore service flows without weakening package validation or permission checks.
- [x] 6.3 Replace all user-visible extension-management English literals with complete Chinese and English localized messages, including permission labels, busy states, success, errors, and diagnostics.
- [x] 6.4 Ensure lifecycle results reload plugin records and immediately refresh Tools navigation, per-tool destinations, selected-destination fallback, and related search descriptors.
- [x] 6.5 Add parity tests for market/local approval, declined install, lifecycle operations, restore, error ownership, dynamic navigation refresh, and preservation of existing security boundaries.

## 7. Search, Accessibility, and Integration Verification

- [x] 7.1 Wire search result activation to destination selection, scrolling, and primary-control focus; add keyboard coverage for search, result activation, Escape clearing, sidebar navigation, Back to Settings, and logical tab order.
- [x] 7.2 Add structural coverage ensuring every searchable setting ID resolves to a real destination and rendered focus target in both supported languages.
- [x] 7.3 Verify screen-reader names/descriptions, selected-destination semantics, live-region feedback, disabled explanations, focus visibility, reduced motion, and regular/narrow layout behavior.
- [x] 7.4 Run `pnpm test` and `pnpm build`, fixing all unit, integration, TypeScript, module-boundary, and production-build failures.
- [x] 7.5 Run `cargo fmt --check`, `cargo check`, and `cargo test` from `src-tauri`, including shortcut and status-bar contract coverage.
- [x] 7.6 Run `openspec validate optimize-preferences-settings --type change --strict` and `git diff --check`.
- [x] 7.7 Run `pnpm tauri dev` and manually verify regular and narrow preference-window navigation, search/focus, immediate language/autostart behavior, distinct tool controls, and extension permission/lifecycle workflows.
- [x] 7.8 On macOS, manually verify that native status-item collapse/expand updates the open Status Bar page and preview, and that the displayed Zero Snap and Zero Launch shortcuts match real invocation behavior; retain Windows runtime shortcut and native-window smoke as explicit pending validation unless run on Windows.
