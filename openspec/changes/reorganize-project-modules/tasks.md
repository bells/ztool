## 1. Baseline and module contracts

- [x] 1.1 Record the current frontend test-file/test-case inventory and run the available focused fixture tests, `pnpm build`, `cargo fmt --check`, `cargo check`, and `cargo test` to establish a pre-migration baseline.
- [x] 1.2 Define the core `BundledPluginModule` and render/localization/contribution contracts without `any`, including the distinction between trusted build-time modules and sandboxed runtime extensions.
- [x] 1.3 Add pure descriptor-registry validation for canonical plugin IDs, manifests, renderers, and unique command/view/status-bar contribution identifiers, with focused tests for valid and conflicting modules.

## 2. Frontend host-core separation

- [x] 2.1 Move `src/plugins/pluginHost/` to `src/core/pluginHost/` and `src/plugins/preferences/` to `src/core/preferences/` using renames, preserving one implementation of each module.
- [x] 2.2 Move `src/plugins/types.ts` into the plugin-host core as `pluginTypes.ts` and update host, preference, shell, component, and service type imports.
- [x] 2.3 Move host-facing launcher Extension API request contracts into the core boundary and make the launcher plugin consume or re-export them without duplicating Rust/TypeScript IPC shapes.
- [x] 2.4 Remove the plugin-host core's default import of the concrete Quick Launcher service; inject the launcher host adapter from the app-shell composition boundary.
- [x] 2.5 Update all frontend imports and temporary fixture compilation inputs for the new core paths, then verify that `src/core/**` has no concrete `src/plugins/**` dependency.

## 3. Self-contained bundled frontend plugins

- [x] 3.1 Create one `plugin.tsx` descriptor for Zero Awake that owns its manifest, presentation metadata, localization, primary renderer, and exported module entry point.
- [x] 3.2 Create one `plugin.tsx` descriptor for Zero Paper that owns its manifest, presentation metadata, localization, primary renderer, dedicated Paper surface, and exported module entry point.
- [x] 3.3 Create one `plugin.tsx` descriptor for Zero Launch that owns its manifest, presentation metadata, localization, primary renderer, dedicated Launcher surface, and exported module entry point.
- [x] 3.4 Create one `plugin.tsx` descriptor for Zero Snap that owns its manifest, presentation metadata, localization, primary renderer, capture/pin surfaces, and exported module entry point.
- [x] 3.5 Split plugin-specific message keys/bundles out of global preferences i18n while retaining host-wide language selection and host strings in `src/core/preferences/`.
- [x] 3.6 Add `src/appShell/bundledPluginModules.ts` as the only frontend composition registry and derive bundled manifests, plugin kind/accent metadata, panel rendering, and dedicated-surface routing from registered descriptors.
- [x] 3.7 Replace `App.tsx`, `main.tsx`, and plugin-host hard-coded bundled-plugin switches/tables with generic descriptor lookups while keeping unknown third-party plugins on `ExtensionSurface`.
- [x] 3.8 Add structural tests proving every bundled directory has one registered descriptor, no sibling plugin imports another plugin, core imports plugins only at the composition boundary, and removing a descriptor leaves no host-core reference to that plugin.

## 4. Rust ownership and plugin isolation

- [x] 4.1 Consolidate `commands/bing_wallpaper.rs` and `commands/paper.rs` under `commands/bing_wallpaper/`, nesting/re-exporting the Paper window helpers without changing registered Tauri command names.
- [x] 4.2 Consolidate `services/bing_wallpaper.rs` and `services/wallpaper.rs` under `services/bing_wallpaper/`, keeping wallpaper platform operations owned by Zero Paper and preserving public service types/results.
- [x] 4.3 Introduce a host-owned tool-window coordinator and route Paper/Quick Launcher mutual-exclusion behavior through it, removing direct command/service calls between concrete Rust plugins.
- [x] 4.4 Update `commands/mod.rs`, `services/mod.rs`, `lib.rs`, status-bar routing, internal Rust paths, and path-sensitive tests for the new module ownership while leaving host-wide status-bar/native-resource/extension-runtime modules separate.
- [x] 4.5 Group managed state, startup hooks, and `generate_handler!` entries by bundled plugin at the Rust composition root so adding/removing a trusted plugin is explicit and does not require changes inside peer plugins.
- [x] 4.6 Add or update Rust structural/unit tests for `bing_wallpaper` ownership, host window coordination, peer-plugin isolation, handler registration, and unchanged command/serde contracts.

## 5. Test hierarchy and developer workflow

- [x] 5.1 Move pure tests into `tests/unit/` grouped by `core`, concrete plugin, service, app-shell, and brand ownership, preserving `*.test.mjs` names.
- [x] 5.2 Move cross-module, Extension API, Tauri capability, source-boundary, and source/config contract tests into grouped `tests/integration/` directories.
- [x] 5.3 Update every temporary TypeScript fixture compile input, `rootDir`, output import, direct source URL, and working-directory assumption to resolve after both source and test moves.
- [x] 5.4 Add canonical `test`, `test:unit`, and `test:integration` package scripts that recursively discover nested tests exactly once, without adding a new test framework.
- [x] 5.5 Update `README.md` and `openspec/project.md` with the new source layout, build-time/runtime pluggability boundary, focused fixture commands, and recursive full-suite commands.
- [x] 5.6 Review unarchived OpenSpec changes for unfinished path-sensitive tasks, updating only still-actionable references or recording their new-path equivalents without rewriting completed history.

## 6. Compatibility and final verification

- [x] 6.1 Search live source, tests, scripts, and durable documentation for stale `src/plugins/pluginHost`, `src/plugins/preferences`, `commands::paper`, `services::wallpaper`, and flat `tests/*.mjs` assumptions and resolve all actionable references.
- [x] 6.2 Run descriptor/manifest/import-boundary tests and all focused core/plugin/service TypeScript fixture tests from clean temporary outputs.
- [x] 6.3 Run `pnpm test`, confirm its discovered test-file/test-case inventory matches the reorganized baseline with no duplicates or omissions, and run `pnpm build`.
- [x] 6.4 Run `cargo fmt --check`, `cargo check`, and `cargo test`, confirming the unchanged IPC handler names and serialized contracts.
- [x] 6.5 Run `openspec validate reorganize-project-modules --type change --strict` and `git diff --check`, then inspect the final diff for rename-only moves versus intentional boundary/composition changes.
- [ ] 6.6 Manually smoke-test the tray/main/preferences/about surfaces and Zero Awake, Zero Paper, Zero Launch, and Zero Snap entry points on macOS; keep Windows runtime/device verification explicit if it is not performed.
