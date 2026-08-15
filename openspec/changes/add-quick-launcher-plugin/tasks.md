## 1. Dependency and platform feasibility gates

- [x] 1.1 Benchmark `nucleo-matcher` and `fuzzy-matcher` against a deterministic 10,000-entry English/Chinese/alias fixture, recording release-mode p50/p95, match quality, license, maintenance activity, binary impact, and the selected adapter.
- [x] 1.2 Validate a maintained Rust pinyin library against simplified/traditional characters, polyphonic fallback, full-pinyin and initials generation, then record and lock the selected version behind a `Romanizer` adapter.
- [ ] 1.3 Validate `notify` on the configured macOS and Windows application roots, including missing-directory, permission-denied, event-storm, shutdown, and cross-target compile behavior.
- [x] 1.4 Complete a macOS spike for Info.plist parsing, NSWorkspace icon extraction, NSRunningApplication probing, application launch/focus, and settings URI opening using maintained crates or a narrow native adapter.
- [ ] 1.5 Complete a Windows spike for `.lnk` parsing, Shell icon extraction, process/window identity, `SetForegroundWindow`, `ShellExecuteW`, and `ms-settings:` opening using the existing `windows` crate plus a maintained shortcut parser if required.
- [ ] 1.6 Add only the selected dependencies and target-specific features to `src-tauri/Cargo.toml`, confirm license compatibility, and prove macOS plus `x86_64-pc-windows-msvc` compile paths before production modules depend on them.

## 2. Symmetric contracts and permission vocabulary

- [x] 2.1 Define Rust wire types for item kind, index snapshot/source/support, search input/result/item, icon input/result, activation input/result/action, running state, diagnostics, and structured launcher errors with camelCase serialization.
- [x] 2.2 Define matching TypeScript interfaces and unions in `src/plugins/quickLauncher/contracts.ts` without `any`, including the exact enum and optional-field semantics used by Rust.
- [x] 2.3 Add `system.apps.read`, `system.apps.execute`, `system.window.focus`, and `system.settings.open` to Rust and TypeScript permission vocabularies, manifest validators, fixtures, registry persistence, permission-review labels, and developer protocol types.
- [x] 2.4 Refactor Extension Bridge authorization from one required permission to a required-permissions set while preserving all existing methods and their current permission behavior.
- [x] 2.5 Add contract tests for stable Rust serialization and TypeScript shapes, malformed fields, unsupported enums, query/limit bounds, activation actions, error codes, and the four new permission values.
- [x] 2.6 Add Bridge tests proving full multi-permission approval succeeds and missing declaration, missing approval, disabled state, identity mismatch, arbitrary target fields, and unsupported methods fail before native dispatch.

## 3. Search entities, settings catalog, and ranking

- [x] 3.1 Create `src-tauri/src/services/quick_launcher/` with internal item, private launch-target, stable-ID, source-metadata, search-field, usage, and adapter traits separated from public IPC contracts.
- [x] 3.2 Implement stable item-ID generation for macOS Bundle IDs/canonical bundles, Windows normalized launch identities, and system-setting catalog IDs, with collision and platform-scope tests.
- [x] 3.3 Implement the versioned macOS/Windows system-settings catalog with stable IDs, zh-CN/en-US titles, aliases, platform support, and private `x-apple.systempreferences:`/`ms-settings:` destinations.
- [x] 3.4 Add catalog tests for general, display, network, Bluetooth, sound, keyboard, mouse/trackpad, notifications, and privacy/security, including platform omission and proof that public results do not serialize URIs.
- [x] 3.5 Implement Unicode/case normalization, English token initials, pinyin full spelling, pinyin initials, and bundled alias fields, including `微信 -> weixin/wx` and `Photoshop -> ps` fixtures.
- [x] 3.6 Implement the selected fuzzy matcher and deterministic relevance-first ranking with exact/prefix tiers, bounded alias/pinyin contribution, logarithmic frequency boost, recency decay, running bonus, and stable tie breakers.
- [x] 3.7 Implement empty-query recent/frequent results, result limiting, oversized-query validation, and match metadata without reading disk or decoding icons in the search path.
- [x] 3.8 Add pure search tests for English, Chinese, pinyin, initials, acronyms, aliases, exact-versus-frequent ordering, deterministic ties, empty query, malformed input, and missing runtime state.
- [x] 3.9 Add the release benchmark harness and CI/reporting command for 10,000 mixed-language entries, then optimize or document a blocking decision if pure matching p95 exceeds 5ms on the reference hardware.

## 4. Versioned cache, usage history, and refresh coordination

- [ ] 4.1 Implement `~/.zero/data/quick-launcher/` resolution and versioned `apps_cache.json` loading that validates schema, platform, entry identities, normalized roots, and source modification metadata.
- [x] 4.2 Implement same-directory temporary writes, flush, atomic replacement, stale `.part` cleanup, and complete-revision swaps so readers never observe a partial index.
- [ ] 4.3 Implement versioned `usage.json` with bounded counts, last-used time, record cap/retention cleanup, success-only updates, atomic writes, and no raw query persistence.
- [x] 4.4 Implement cache-first initialization that publishes a usable memory snapshot before scheduling a background scan, and rebuilds non-fatally from missing, corrupt, incompatible, or cross-platform cache files.
- [x] 4.5 Implement `QuickLauncherState` with read-optimized immutable revisions, one in-flight refresh, blocking-task isolation, refresh diagnostics, and safe shutdown behavior.
- [x] 4.6 Implement the directory Watcher with known-root scoping, approximately 500ms event coalescing, full-rescan/diff refresh, overlap prevention, and degraded fallback when roots cannot be watched.
- [ ] 4.7 Add temporary-directory tests for valid cache startup, corrupt/incompatible recovery, atomic replacement, `.part` cleanup, usage success/failure, retention, event storms, simultaneous refresh, missing roots, and watcher degradation.

## 5. macOS and Windows application adapters

- [x] 5.1 Implement the macOS scanner for `/Applications`, `~/Applications`, `/System/Applications`, and `/System/Applications/Utilities`, with `.app` boundary handling, Info.plist fallback fields, canonicalization, Bundle ID/path deduplication, and non-fatal diagnostics.
- [x] 5.2 Implement macOS running-state batching and short TTL using Bundle ID/NSRunningApplication without persisting authoritative running flags.
- [x] 5.3 Implement macOS launch/focus and settings opening through native APIs, returning truthful `focused`, `launched`, unsupported, not-found, and operation-failed outcomes without shell-string interpolation.
- [x] 5.4 Implement macOS lazy icon extraction and bounded PNG caching keyed by item identity and source mtime, with a non-fatal fallback when conversion fails.
- [x] 5.5 Implement the Windows scanner for machine/user Start Menu Programs `.lnk` and `.exe` entries, including safe shortcut parsing, normalized target/AppUserModel identity, user-entry preference, deduplication, and diagnostics.
- [x] 5.6 Implement Windows running-state/window batching with a short TTL and an explicit unknown state when no reliable identity can be established.
- [x] 5.7 Implement Windows focus/launch/settings adapters with `SetForegroundWindow` and `ShellExecuteW`, preserving `focus_denied` and `launchedFallback` instead of false focus success.
- [x] 5.8 Implement Windows lazy Shell icon extraction and bounded PNG caching keyed by item identity and source mtime, with a non-fatal fallback when extraction fails.
- [ ] 5.9 Add fixture/fake-adapter tests for both scanners, metadata fallback, overlapping roots, unsafe shortcut exclusion, application disappearance, running-state expiry, icon failure, focus denial, launch failure, URI failure, and unsupported-platform behavior.

## 6. Managed service, Tauri commands, and Extension API

- [x] 6.1 Implement service operations for snapshot, coalesced refresh, bounded in-memory search, lazy icon lookup, application launch/focus, and setting open using stable IDs and current revisions.
- [x] 6.2 Reject stale/missing IDs and caller-supplied path, Bundle ID, command-line, shortcut-target, or URI fields before adapter dispatch, with tests proving no arbitrary fallback executes.
- [x] 6.3 Update usage only after confirmed `focused`, `launched`, `launchedFallback`, or `openedSetting` outcomes, and preserve successful activation when only the later usage write fails.
- [x] 6.4 Add thin `commands/quick_launcher.rs` handlers for snapshot, refresh, search, icon, activation, show-window, and hide-window with typed `Result` responses.
- [x] 6.5 Register `QuickLauncherState`, service modules, background cache/refresh setup, Watcher lifecycle, and all launcher commands in `src-tauri/src/lib.rs` without blocking the Tauri main thread.
- [x] 6.6 Extend Extension API dispatch for `launcher.scanApps`, `launcher.search`, `launcher.launchOrFocus`, and `launcher.openSystemSetting`, delegating to the same service used by bundled commands.
- [ ] 6.7 Add IPC/Bridge tests for command names, camelCase inputs, result limits, error envelopes, permission sets, stale revisions, shared-service delegation, disabled callers, and isolation from direct path/cache/process access.

## 7. Bundled plugin registration and registry migration

- [x] 7.1 Add the `zero.launch` TypeScript bundled manifest with short ID `quick-launcher`, author `bells`, version `1.0.0`, `webview` runtime, macOS/Windows platforms, view/command contributions, and four launcher permissions.
- [x] 7.2 Add the matching Rust bundled manifest and approved permissions, bump the registry schema, and migrate only missing Zero Launch records without changing existing plugin lifecycle state.
- [x] 7.3 Extend restore-defaults, bundled kind/renderer resolution, accent/icon fallback, plugin navigation, preference normalization, permission display, and about counts for the fourth bundled tool.
- [x] 7.4 Add zh-CN/en-US metadata, search, running, launch/focus, settings, refresh, cache, unsupported, empty, shortcut-conflict, stale-item, and structured error translations.
- [x] 7.5 Extend bundled manifest, registry seed/migration/restore, preferences, i18n, plugin selection, and three-existing-plugin regression tests.

## 8. Frontend service, state, and reusable search view

- [x] 8.1 Implement `quickLauncherService.ts` typed invoke wrappers and one payload-construction boundary that only sends documented query, limit, item ID, revision, and icon key fields.
- [x] 8.2 Implement pure `quickLauncherModel.ts` helpers for result grouping, default selection, ArrowUp/ArrowDown wrapping, selection preservation/reset, empty state, and activation availability.
- [x] 8.3 Add Node tests for every navigation boundary, result replacement, removed selection, empty/single-item lists, duplicate Enter prevention, and panel versus floating-window dismissal decisions.
- [x] 8.4 Implement `useQuickLauncher` with cache/snapshot load, query generation guards, refresh, lazy visible-icon loading, activation in-flight guard, retry, success feedback, and disposal protection.
- [ ] 8.5 Add controller-level tests for cache-ready/background-refresh sequencing, stale query completion, selection during refresh, icon failure, duplicate activation, structured errors, unmount, and successful action refresh.
- [x] 8.6 Implement `QuickLauncherView` with autofocus search, unified application/setting rows, official/fallback icons, title/subtitle, running/action labels, loading/degraded/unsupported/empty/error states, and accessible live feedback.
- [x] 8.7 Implement `QuickLauncherPanel` using the shared View, including manual refresh and persistent in-panel success/error behavior without coupling it to floating-window hiding.

## 9. Launcher window and global shortcut

- [x] 9.1 Add `launcher` to `AppSurface`, `src/main.tsx` routing, Tauri capabilities, and window-label contract tests.
- [x] 9.2 Implement trusted Rust creation/reuse of one centered approximately 680×420 frameless, always-on-top, skip-taskbar Launcher window with fixed host-controlled options.
- [x] 9.3 Implement `QuickLauncherApp` with the shared View, fresh-input focus on show, `Escape` hide, blur hide, successful-activation hide, and no duplicate WebViews.
- [x] 9.4 Integrate `CommandOrControl+Shift+Space` into a shortcut coordinator shared with screenshot registration, including registration conflict diagnostics, plugin-enabled checks, disable/unregister, and shutdown cleanup.
- [ ] 9.5 Add tests for window option selection, single-window reuse, routed surface, shortcut conflict, disabled plugin, repeated key events, Escape, blur, and activation-time dismissal guards.
- [x] 9.6 Add compact and floating Launcher styles with selected/hover/running states, visible focus, text truncation/reflow, no horizontal overflow, reduced-motion support, and practical desktop click targets.

## 10. Documentation and verification

- [x] 10.1 Update README and plugin developer/protocol documentation with Zero Launch behavior, default shortcut, supported platforms, local cache/usage paths, four permissions, Bridge methods, privacy, and the decision not to use `plugin.wasm`.
- [x] 10.2 Run focused TypeScript compilation and Node tests for launcher contracts, model, service, Hook/controller, app-surface routing, bundled manifest, preferences, permissions, and Bridge changes.
- [ ] 10.3 Run the recursive `pnpm test` suite, `pnpm build`, `cargo fmt --check`, `cargo check`, `cargo test`, and `git diff --check`, fixing regressions across the existing three plugins.
- [x] 10.4 Add or run Windows CI for the selected shortcut parser, Shell/window API feature set, URI catalog, scanner/cache/service tests, and `x86_64-pc-windows-msvc` compilation.
- [x] 10.5 Run the release search benchmark on documented reference hardware and attach the 10,000-entry p50/p95 result, confirming p95 `< 5ms` or resolving the performance gate.
- [ ] 10.6 Run `pnpm tauri dev` on macOS and manually verify panel search, shortcut window, English/Chinese/pinyin/initial/alias queries, cache-first reopen, running app focus, cold app launch, every supported setting link, usage reorder, keyboard operation, blur/Escape, disabled plugin, and shortcut conflict behavior.
- [ ] 10.7 Manually verify application install/uninstall refresh, Watcher degradation, cache/usage corruption recovery, icon fallback, stale selection, no duplicate window, no query persistence, and preservation of screenshot/caffeine/Bing wallpaper behavior.
- [ ] 10.8 Complete equivalent Windows manual verification for `.lnk`/`.exe` discovery, focus success/denial, launch fallback, Start Menu changes, Shell icons, `ms-settings:` destinations, keyboard behavior, and app/shortcut privilege differences.
- [ ] 10.9 Run `openspec validate "add-quick-launcher-plugin"` and confirm `openspec status --change "add-quick-launcher-plugin"` reports every implementation prerequisite complete before applying or archiving the change.
