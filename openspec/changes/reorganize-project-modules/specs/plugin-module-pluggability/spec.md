## ADDED Requirements

### Requirement: Each bundled frontend plugin is self-contained
Each bundled frontend plugin SHALL own a typed module descriptor, manifest, presentation metadata, plugin-specific localization, UI entry points, and domain contracts/models/services within its own `src/plugins/<plugin>/` directory.

#### Scenario: Inspect a bundled plugin module
- **WHEN** a maintainer opens one of the four bundled plugin directories
- **THEN** the directory exposes one typed descriptor that identifies the plugin and all host contributions required to render and register it

#### Scenario: Change plugin-specific presentation
- **WHEN** a maintainer changes a bundled plugin's title, description, accent, localization, manifest contribution, or primary renderer
- **THEN** the change is made within that plugin without editing host-core dictionaries, manifest tables, or plugin-selection switch statements

### Requirement: Bundled plugins register through one composition boundary
The frontend SHALL register bundled plugin descriptors through one app-shell composition root, and host navigation, manifests, panel rendering, and dedicated plugin surfaces SHALL be derived from the registered descriptors rather than hard-coded per-plugin branches in host core.

#### Scenario: Add a bundled plugin
- **WHEN** a new trusted bundled plugin is added
- **THEN** its frontend integration requires its self-contained module plus one typed composition registration entry and does not require edits inside an existing plugin

#### Scenario: Remove a bundled plugin from a build
- **WHEN** a bundled plugin descriptor is removed from the composition registry and its native registration is removed
- **THEN** the remaining plugins, host preferences/about surfaces, and plugin host continue to build and operate without references to the removed plugin

#### Scenario: Registry contains a third-party plugin
- **WHEN** an enabled installed plugin has no bundled module descriptor
- **THEN** the host routes its declared view through the generic isolated extension surface rather than requiring a new core switch branch

### Requirement: Concrete plugins do not depend on peer plugins
Concrete frontend and Rust plugin modules MUST NOT import or call another concrete plugin module; behavior involving multiple tools SHALL be coordinated through a host-owned contract or service.

#### Scenario: Dedicated tool windows are mutually coordinated
- **WHEN** opening one tool window requires another tool window to close
- **THEN** both plugins call or respond to a host-owned window coordinator without importing or invoking each other's command module

#### Scenario: Plugin needs a shared request contract
- **WHEN** multiple plugins or the host require the same API request shape
- **THEN** the contract is owned by an appropriate core boundary and no sibling plugin becomes the shared dependency

#### Scenario: Structural dependency test runs
- **WHEN** the source-boundary test scans frontend and Rust plugin imports
- **THEN** it fails on a core-to-concrete-plugin dependency outside the composition root or any concrete-plugin-to-peer-plugin dependency

### Requirement: Plugin registration is complete and conflict-free
The build SHALL validate that every registered bundled plugin descriptor has a unique canonical plugin ID, valid manifest, resolvable renderer contributions, and non-conflicting command, view, and status-bar contribution identifiers.

#### Scenario: Registered descriptors are valid
- **WHEN** the bundled plugin registry is constructed during tests or application startup
- **THEN** all descriptors pass identity, manifest, renderer, and contribution uniqueness validation before the host consumes them

#### Scenario: Two plugins declare the same contribution
- **WHEN** bundled descriptors reuse a canonical plugin ID or contribution identifier
- **THEN** validation fails with the conflicting identifiers instead of silently selecting one plugin

### Requirement: Native trust boundary remains explicit
Bundled plugins with native capabilities SHALL be compiled and explicitly registered at build time, while installed third-party plugins MUST remain runtime-pluggable only through validated packages, isolated surfaces, approved permissions, and the Extension API Bridge.

#### Scenario: Bundled plugin uses a native capability
- **WHEN** a trusted bundled plugin requires screenshot, power, wallpaper, launcher, or window APIs
- **THEN** its Rust state and Tauri handlers are explicitly registered by the application composition layer with existing typed IPC contracts

#### Scenario: Third-party plugin requests native access
- **WHEN** an installed third-party plugin requests a native capability
- **THEN** the host permits only the versioned Extension API operation covered by declared and approved permissions and does not dynamically load plugin-provided Rust code
