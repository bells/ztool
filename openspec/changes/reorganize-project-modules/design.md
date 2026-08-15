## Context

The frontend has four concrete bundled tools under `src/plugins/`, but the same directory also contains the extension registry/market/Bridge host and the global preferences/about surfaces. This makes `plugins` mean both "a tool" and "the application infrastructure that owns tools." Several shared services and components already depend on plugin-host contracts, and concrete tools depend on global translation/preferences utilities, so moving folders without defining a dependency direction would only relocate the ambiguity.

The Rust side already separates thin Tauri handlers in `commands/` from business logic in `services/`. Its concrete plugin modules mostly map to the frontend, but Zero Paper is split across `bing_wallpaper.rs`, a generic-looking `wallpaper.rs`, and `commands/paper.rs`. Host-wide modules such as `status_bar`, `native_resources`, app-window commands, and the extension registry/runtime do not represent concrete tools and should not be forced into a plugin mapping.

The JavaScript tests are flat under `tests/`. Many import focused TypeScript builds from `/private/tmp/zero-*`, while a smaller group reads source/config files directly. Moving the files therefore requires coordinated updates to test discovery, fixture compilation, relative source paths, and developer documentation.

This is a cross-cutting source migration. It must preserve the existing React/Tauri ownership boundary, symmetric Rust/TypeScript IPC contracts, plugin IDs, stored settings, window labels, command names, runtime behavior, and current active OpenSpec work.

## Goals / Non-Goals

**Goals:**

- Make `src/core/` the clear owner of plugin-host infrastructure and global preferences/about behavior.
- Leave `src/plugins/` containing only the concrete `caffeine`, `bingWallpaper`, `quickLauncher`, and `screenshot` tools.
- Establish an acyclic frontend dependency direction: core contracts/utilities are reusable by plugins, and the app-shell composition root wires host records to concrete panels.
- Make each bundled plugin self-contained and build-time pluggable through one typed descriptor and one composition registration entry.
- Ensure no concrete frontend or Rust plugin imports another concrete plugin; host-owned coordinators handle cross-tool behavior.
- Make Rust plugin command/service ownership directly discoverable without mixing host-wide services into plugin modules.
- Group tests by level and owning module while retaining one recursive full-suite command and focused commands.
- Complete the migration atomically enough that no committed state contains duplicate implementations or compatibility shims with two sources of truth.

**Non-Goals:**

- Change UI, tool behavior, Extension API semantics, IPC command strings, Tauri capabilities, storage keys/formats, plugin IDs, or platform support.
- Redesign the plugin SDK, split `App.tsx`, rewrite test logic, introduce a new test framework, or add third-party dependencies.
- Move the Rust extension host in `src-tauri/src/plugins/`; it is already a cohesive host-runtime module and is not a concrete-plugin collection.
- Rewrite historical or active OpenSpec prose only because it records a path that was correct when authored. New implementation and durable project documentation will use the new paths.

## Decisions

### 1. Introduce explicit frontend host-core boundaries

The target frontend structure is:

```text
src/
├── core/
│   ├── pluginHost/
│   │   ├── ...existing host registry, market, Bridge, and host UI modules
│   │   └── pluginTypes.ts
│   └── preferences/
│       └── ...existing global preferences, about, storage, and i18n modules
├── plugins/
│   ├── caffeine/          # plugin.tsx, manifest, i18n, UI, model/service
│   ├── bingWallpaper/     # plugin.tsx, manifest, i18n, UI, model/service
│   ├── quickLauncher/     # plugin.tsx, manifest, i18n, UI, model/service
│   └── screenshot/        # plugin.tsx, manifest, i18n, UI, model/service
├── appShell/
│   └── bundledPluginModules.ts  # composition root
└── ...app shell, shared components, and shared services
```

`src/plugins/types.ts` moves to the plugin-host core as `pluginTypes.ts`, because `PluginId`, `PluginMeta`, and health metadata describe how the host presents plugins rather than the implementation of a specific tool. `App.tsx` remains the composition root that imports both core host modules and concrete panels.

Concrete plugins may import public types and utilities from `core/pluginHost` and `core/preferences`. Core code must not import concrete React panels or tool services, and one plugin must not import another plugin. Where the host Bridge currently imports a tool-owned request type, the host-facing request shape will move to a host-owned contract and the concrete tool will consume or re-export that shared type, avoiding a `core -> plugins` dependency.

Alternatives considered:

- Rename only the two directories while keeping `plugins/types.ts` and cross-imports. This leaves ownership and dependency direction ambiguous.
- Put preferences under `appShell/`. Preferences, i18n, and About are shared by tray, main, capture, launcher, and paper surfaces, so they are broader than one shell.
- Create broad barrel exports for every folder. This hides ownership and can create circular imports; imports will target explicit public modules unless a small stable index is demonstrably useful.

### 2. Make bundled plugins self-contained and build-time pluggable

Core will define a typed `BundledPluginModule` contract. Each concrete plugin exports one `plugin.tsx` descriptor containing its manifest, presentation metadata, plugin-local localization bundle, main-panel renderer, and any dedicated surface renderers. Domain contracts, hooks, models, services, and UI stay within the same plugin directory. Plugin-specific translation keys currently held by the global preferences dictionary move into their owning plugin; core preferences retains only host-wide language selection and host strings.

`src/appShell/bundledPluginModules.ts` is the only frontend composition root that imports all four descriptors. The shell and host registry consume the descriptor collection generically for manifests, navigation/presentation, panel rendering, and dedicated-surface routing. Adding or removing a bundled plugin therefore changes its own directory plus one registration list, without editing host-core switch statements or another plugin.

"Pluggable" has two deliberate levels:

- Bundled React/Rust plugins are **build-time pluggable** because native commands and Tauri state are compiled into the trusted application. Their registration is explicit and type-checked.
- Installed third-party `.zplugin` packages remain **runtime pluggable** through the existing registry, isolated WebView surface, validated manifest, permissions, and Extension API Bridge. They do not load arbitrary native Rust modules.

The Rust composition root will keep one explicit registration section per bundled plugin for managed state, startup hooks, and Tauri handlers. Individual plugin command/service modules must not call peer plugin modules. The current Paper/Quick Launcher mutual hide calls move behind a host-owned tool-window coordinator; status-bar coordination may call plugin-facing host actions because it is itself host-wide.

Structural tests will enforce descriptor completeness, unique plugin IDs/contributions, no imports between sibling `src/plugins/*` modules, no `src/core -> src/plugins` imports, and no Rust concrete-plugin-to-peer-plugin paths. Existing lifecycle tests continue to cover enable/disable/uninstall behavior.

Alternatives considered:

- Keep `bundledPluginKind()` and the `pluginPanel()` switch in host code. This makes every new plugin require host edits and is not genuinely pluggable.
- Put all bundled manifests in the host registry. This centralizes data but separates a plugin's identity/contributions from its implementation and makes removal error-prone.
- Convert bundled tools into dynamically loaded `.zplugin` packages immediately. Screenshot, caffeine, wallpaper, and launcher need trusted native integrations; forcing them through the third-party sandbox would either break capabilities or weaken security.
- Build each bundled tool as a separate Tauri plugin crate now. That can be revisited when native modules are shared across applications, but it adds crate and command-namespace complexity without improving Zero's current in-repo boundary.

### 3. Align Rust plugin modules by ownership, not filename suffix

The concrete plugin command/service names remain idiomatic Rust snake_case mappings of the frontend tools:

| Frontend plugin | Command module | Service module |
| --- | --- | --- |
| `caffeine` | `commands/caffeine.rs` | `services/caffeine.rs` |
| `bingWallpaper` | `commands/bing_wallpaper/` | `services/bing_wallpaper/` |
| `quickLauncher` | `commands/quick_launcher.rs` | `services/quick_launcher/` |
| `screenshot` | `commands/screenshot.rs` | `services/screenshot.rs` |

Zero Paper's existing `commands/bing_wallpaper.rs` and `commands/paper.rs` become one `commands/bing_wallpaper/` module with the Paper window helper nested under that owner. Likewise, `services/wallpaper.rs` moves beneath `services/bing_wallpaper/` as a platform adapter/helper. Public Rust module paths used from `lib.rs` are preserved with re-exports where that reduces churn, while ambiguous `commands::paper` and `services::wallpaper` references are replaced by the owning `bing_wallpaper` path.

Host-wide modules remain outside concrete plugin modules: `commands/app`, `commands/plugins`, `commands/status_bar`, `services/status_bar`, and `services/native_resources`. The existing `src-tauri/src/plugins/` continues to own extension package, registry, market, and runtime behavior.

Command modules will not use a `_cmd` suffix. Being inside `commands/` already communicates the layer, while the unsuffixed domain name keeps `commands::<plugin>` aligned with `services::<plugin>` and existing call sites.

Alternatives considered:

- Rename every handler file to `*_cmd.rs`. This adds redundant layer naming and makes command/service module paths less symmetric.
- Nest all services under `services/plugins/`. That adds another level without resolving which modules are host-wide and would create needless churn for already aligned modules.
- Force status-bar and native-resource code into plugin folders. Both coordinate multiple tools or platforms and therefore belong to the host.

### 4. Classify tests by scope and module

The target test structure is:

```text
tests/
├── unit/
│   ├── core/pluginHost/
│   ├── core/preferences/
│   ├── plugins/{caffeine,bingWallpaper,quickLauncher,screenshot}/
│   ├── services/
│   ├── appShell/
│   └── brand/
└── integration/
    ├── appShell/
    ├── extensionHost/
    └── sourceContracts/
```

Pure model/controller/storage tests belong under `unit/`. Tests that validate multiple module boundaries, Tauri capability routing, extension-host dispatch, or source/config integration belong under `integration/`. Filenames keep the `*.test.mjs` convention.

`package.json` will expose a canonical recursive full-suite command plus focused unit and integration commands. Existing temporary TypeScript fixture compilation remains lightweight and dependency-free, but every compile input, `rootDir`, output import, direct source-file URL, README command, and `openspec/project.md` command will be updated for the new paths. The canonical command must discover nested tests rather than relying on `tests/*.mjs`.

Alternatives considered:

- Keep the tests flat until there are more files. There are already over thirty files spanning unrelated domains, and the source migration is the lowest-risk time to align them.
- Add Vitest or another runner. The current Node test approach is sufficient; a new dependency would expand this structural change without product value.
- Mirror every source directory mechanically. Test level is useful information, but a one-file-per-source mirror would add depth without improving ownership.

### 5. Preserve observable contracts and verify references

Moves use Git-aware renames followed by import/module updates; implementations are not copied. Tauri `#[command]` function names and frontend `invoke` strings stay unchanged. Serde field names, TypeScript interfaces/unions, event names, plugin manifest `main` values, persisted storage keys, config paths, and Tauri window/capability labels remain unchanged unless a value is proven to be a source-only path rather than a runtime identifier.

Path searches cover source, tests, scripts, README/project documentation, and all active OpenSpec changes. Active artifacts are not rewritten as historical records, but implementation must not rely on stale paths from them; any still-pending path-sensitive task is called out during apply verification.

## Risks / Trade-offs

- [Large rename diff obscures accidental code edits] → Perform moves in bounded frontend, Rust, and test phases; inspect rename detection and avoid behavior edits in the same patches.
- [Core still depends on a concrete plugin through types] → Search imports after the move and add a structural source-contract test that rejects `src/core/**` imports from `src/plugins/**`.
- [A plugin remains coupled through hard-coded shell or localization switches] → Require one descriptor per plugin, derive host presentation from descriptors, and enforce sibling-plugin import and descriptor-registration tests.
- [Build-time pluggability is mistaken for arbitrary native runtime loading] → Document and test the trust boundary: bundled native modules are compiled/registered explicitly; third-party packages use the sandboxed Extension API only.
- [Rust reorganization changes module visibility or command registration] → Preserve handler function names/re-exports and run formatting, check, and the complete Rust test suite.
- [Nested tests are silently skipped] → Add canonical recursive package scripts and compare the discovered/passed test count with the pre-migration suite.
- [Temporary compiled fixture imports become stale] → Update fixture compilation and absolute temporary import layouts together, then run every focused fixture group before the full suite.
- [Existing OpenSpec tasks mention old source paths] → Search all unarchived changes during implementation and document path replacements for still-pending work without rewriting completed history.
- [Case-only or cross-platform path problems] → Use exact directory casing (`pluginHost`, `bingWallpaper`, `quickLauncher`) on the frontend and snake_case Rust modules; validate on the current macOS checkout and retain Windows runtime verification boundaries.

## Migration Plan

1. Record the current test inventory and run the available frontend/Rust baseline checks before moving files.
2. Move `pluginHost`, `preferences`, and shared plugin metadata into `src/core/`; update imports and remove any core-to-concrete-plugin type dependency.
3. Add the typed bundled-plugin descriptor, move each manifest/localization/render entry point into its plugin, replace host switches with the app-shell composition registry, and add import-boundary checks.
4. Consolidate Zero Paper Rust command/service helpers under `bing_wallpaper`; move peer window-hiding calls to a host coordinator; update `mod.rs`, `lib.rs`, and per-plugin composition registration.
5. Move tests into unit/integration module groups; update source URLs, temporary fixture paths, package scripts, README, and `openspec/project.md`.
6. Search for stale live-code/documentation paths and inspect active OpenSpec references that affect unfinished tasks.
7. Run focused tests, recursive Node tests, `pnpm build`, Rust formatting/check/tests, strict OpenSpec validation, and `git diff --check`.

Rollback is a source-only revert: restore the previous paths and imports as one change. No user data or persisted schema rollback is required.

## Open Questions

None. The directory boundaries, naming policy, compatibility constraints, and test classification are defined above; individual ambiguous test files will be classified by whether they exercise one module or a cross-module/config boundary.
