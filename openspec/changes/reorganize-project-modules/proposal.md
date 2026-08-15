## Why

Zero's frontend currently stores host infrastructure (`pluginHost` and global `preferences`) beside concrete tools under `src/plugins/`, while backend helper modules and a flat test directory make ownership less obvious as the plugin set grows. Establishing explicit host-core, plugin, backend-service, and test boundaries now will make future plugins easier to locate and change without altering runtime behavior.

## What Changes

- Move frontend plugin-host/runtime/market code and global preferences/about code into `src/core/`, leaving `src/plugins/` for the four concrete bundled tools only.
- Relocate shared plugin metadata types with the host core and update imports so host infrastructure, concrete plugins, shell composition, and shared services have explicit dependency directions.
- Give every bundled plugin a self-contained module descriptor that owns its manifest, UI entry points, localization, contracts, and host contributions; register these descriptors only in a composition root instead of hard-coding plugin switches and manifests inside host core.
- Prevent concrete plugins from importing one another. Move cross-plugin window coordination and shared contracts into host-owned abstractions so adding or removing one bundled plugin does not require edits inside another plugin.
- Align Rust command and service modules with the frontend plugin names (`caffeine`, `bing_wallpaper`, `quick_launcher`, and `screenshot`), nesting plugin-specific helper modules beneath their owning plugin while retaining separate host-wide modules such as app windows, plugin registry/runtime, native resources, and status-bar management.
- Organize JavaScript tests under `tests/unit/` and `tests/integration/` with module-oriented subdirectories, and update fixture paths, documented commands, and package scripts so the full suite remains discoverable and runnable recursively.
- Preserve all Tauri command names, serialized Rust/TypeScript contracts, window labels, persisted settings/data, plugin IDs, UI behavior, and platform behavior; this is a source-layout refactor rather than a product feature.
- **BREAKING (internal):** source-module and test-file import paths change for maintainers and any out-of-tree code importing Zero internals; user-facing and IPC APIs do not change.

## Capabilities

### New Capabilities
- `project-module-boundaries`: Defines the required frontend host/plugin separation, Rust plugin-module alignment, dependency direction, and recursively runnable test organization.
- `plugin-module-pluggability`: Defines self-contained plugin descriptors, peer-plugin isolation, build-time registration/removal for bundled plugins, and the safe runtime boundary for installed third-party plugins.

### Modified Capabilities

None. Existing product requirements and user-visible behavior remain unchanged.

## Impact

- Frontend: `src/App.tsx`, `src/main.tsx`, `src/core/`, `src/plugins/`, shared components/services, and TypeScript imports.
- Backend: `src-tauri/src/commands/`, `src-tauri/src/services/`, module exports, command registration, and Rust tests that refer to source paths.
- Tests and developer workflow: `tests/`, `package.json`, temporary TypeScript fixture output/import paths, `README.md`, `openspec/project.md`, and path-sensitive source-contract tests.
- Active OpenSpec changes that mention old paths must be reviewed for implementation guidance, but their historical artifact text will not be rewritten solely to match the new layout.
- No new runtime dependency, persisted-data migration, permission, or capability change is expected.
