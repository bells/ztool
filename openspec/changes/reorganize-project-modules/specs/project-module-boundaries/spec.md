## ADDED Requirements

### Requirement: Frontend host core is separated from concrete plugins
The source tree SHALL place plugin registration, market, runtime Bridge, global preferences, About, localization, and shared plugin presentation types under `src/core/`, and `src/plugins/` SHALL contain only the concrete `caffeine`, `bingWallpaper`, `quickLauncher`, and `screenshot` tool modules.

#### Scenario: Maintainer locates host infrastructure
- **WHEN** a maintainer inspects the frontend source tree for plugin-host or global-preference behavior
- **THEN** the implementation is located under `src/core/pluginHost/` or `src/core/preferences/` rather than beside concrete tools under `src/plugins/`

#### Scenario: Maintainer locates a bundled tool
- **WHEN** a maintainer inspects `src/plugins/`
- **THEN** each top-level module represents one concrete bundled tool and no host-runtime or global-preferences module is present

### Requirement: Frontend dependencies follow the host boundary
Frontend core modules MUST NOT import concrete plugin implementations, concrete plugins MAY consume public core contracts and utilities, and the app-shell composition layer SHALL be responsible for wiring host records to concrete plugin panels.

#### Scenario: Host-facing contract is shared with a plugin
- **WHEN** the plugin host and a concrete plugin require the same request or presentation type
- **THEN** the shared contract is owned by the core boundary and consumed or re-exported by the concrete plugin without a `src/core/` import from `src/plugins/`

#### Scenario: App shell renders a concrete plugin
- **WHEN** a host plugin record resolves to a bundled tool
- **THEN** the app-shell composition layer selects the concrete panel while the plugin-host core remains independent of that panel implementation

### Requirement: Rust plugin modules map to frontend plugin ownership
The Rust command and service layers SHALL expose discoverable snake_case modules for `caffeine`, `bing_wallpaper`, `quick_launcher`, and `screenshot`, and plugin-specific helpers SHALL be nested beneath their owning plugin module while cross-plugin host services remain separate.

#### Scenario: Locate backend logic for a frontend plugin
- **WHEN** a maintainer traces one of the four frontend bundled tools into the Tauri backend
- **THEN** its handlers are found under the matching `commands::<plugin>` module and its business logic is found under the matching `services::<plugin>` module

#### Scenario: Locate Zero Paper helpers
- **WHEN** a maintainer traces the Zero Paper window or platform wallpaper operation
- **THEN** both are owned beneath the `bing_wallpaper` command or service module rather than exposed as ambiguous top-level `paper` or `wallpaper` modules

#### Scenario: Locate host-wide backend behavior
- **WHEN** a module coordinates multiple plugins or owns application-wide native behavior such as status-bar, app-window, native-resource, registry, market, or runtime management
- **THEN** it remains a host-wide module and is not forced into a concrete plugin directory

### Requirement: Source reorganization preserves runtime contracts
The reorganization MUST preserve Tauri command names, frontend invoke names, Rust/TypeScript serialized field contracts, event names, plugin IDs and manifests, persisted settings and data formats, window labels, Tauri permissions/capabilities, and observable tool behavior.

#### Scenario: Existing frontend invokes a moved backend module
- **WHEN** the frontend calls an existing Tauri command after the module move
- **THEN** the same command name accepts and returns the same typed contract as before the move

#### Scenario: Existing user data is loaded after the move
- **WHEN** Zero starts with preferences, plugin registry data, status-bar settings, or plugin data written by the previous source layout
- **THEN** the application loads the data without a migration caused solely by the source reorganization

#### Scenario: Existing window or plugin action runs after the move
- **WHEN** a user opens a known window or invokes a bundled plugin action
- **THEN** routing and behavior remain unchanged from the pre-migration implementation

### Requirement: Tests are organized and recursively discoverable
JavaScript tests SHALL be grouped under `tests/unit/` and `tests/integration/` by owning module, and repository scripts SHALL provide focused level commands and one canonical command that discovers every nested `*.test.mjs` file.

#### Scenario: Run a focused test level
- **WHEN** a maintainer runs the documented unit or integration test command
- **THEN** the command executes every nested test in that level without requiring a flat root-directory glob

#### Scenario: Run the complete frontend test suite
- **WHEN** a maintainer runs the canonical full-suite command after preparing required TypeScript fixtures
- **THEN** every unit and integration test is discovered exactly once and the suite does not omit nested module directories

#### Scenario: Test reads a moved module or source file
- **WHEN** a test imports a temporary compiled fixture or directly inspects source/configuration
- **THEN** its compile inputs, output import path, and repository-relative source path resolve to the reorganized location

### Requirement: Durable developer guidance uses the new layout
Current package scripts, README verification instructions, and OpenSpec project guidance SHALL reference the reorganized source and test paths, while historical completed change artifacts MAY retain the paths that were accurate when those artifacts were authored.

#### Scenario: Maintainer follows current verification documentation
- **WHEN** a maintainer copies a focused or full verification command from current repository guidance
- **THEN** the command resolves existing files and exercises the intended reorganized modules

#### Scenario: Unfinished active change contains an old path
- **WHEN** implementation review finds that an unfinished active OpenSpec task depends on a pre-migration path
- **THEN** the migration records the corresponding new path or updates the still-actionable task without rewriting unrelated completed history
