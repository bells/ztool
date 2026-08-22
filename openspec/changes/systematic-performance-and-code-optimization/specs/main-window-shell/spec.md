## ADDED Requirements

### Requirement: App-shell routing loads only the required surface code
The system SHALL resolve the Tauri window label before loading a top-level React surface and SHALL dynamically load only the shell or dedicated surface required by that label. The tray/main shell SHALL load only the currently selected bundled plugin panel, and an unknown label SHALL retain the existing safe fallback without eagerly loading every plugin.

#### Scenario: Tray or main window starts
- **WHEN** React starts under a tray or main app-shell label
- **THEN** the shell becomes interactive without loading dedicated capture, pin, launcher, paper, or File-engine surface modules

#### Scenario: Dedicated surface starts
- **WHEN** React starts under capture, pin-*, launcher, paper, preferences, about, or `zero-file-engine`
- **THEN** it loads the matching top-level chunk without initializing the tray/main shell or unrelated bundled plugin renderers

#### Scenario: User selects an unloaded plugin panel
- **WHEN** the tray or main shell selects a bundled plugin whose renderer has not been loaded
- **THEN** the shell presents a compact fallback while loading the panel and remains usable if the import reports an error

### Requirement: Lazy loading preserves plugin composition boundaries
The system SHALL keep bundled plugin metadata and typed render loaders owned by each plugin, SHALL register them only through the frontend composition root, and MUST NOT introduce core-to-concrete-plugin or sibling-plugin imports to implement lazy loading.

#### Scenario: Bundled plugin boundary tests run
- **WHEN** source-contract tests inspect plugin descriptors, host core, sibling imports, and the composition root
- **THEN** every bundled plugin remains independently addable/removable through its own module plus the one composition registration
