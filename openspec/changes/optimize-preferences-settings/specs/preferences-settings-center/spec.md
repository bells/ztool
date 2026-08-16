## ADDED Requirements

### Requirement: Categorized preferences navigation
The system SHALL present the dedicated preferences surface as a categorized settings center with General, Status Bar, Keyboard Shortcuts, Tools, per-tool, and Extensions destinations, and SHALL display only one selected destination's content at a time.

#### Scenario: Preferences opens to General
- **WHEN** the user opens the dedicated preferences window without a previously selected destination in that window session
- **THEN** the system displays the category navigation and selects the General destination

#### Scenario: User selects a settings destination
- **WHEN** the user activates Status Bar, Keyboard Shortcuts, Tools, Extensions, or an installed tool in the navigation
- **THEN** the system marks that destination selected and displays its settings content without opening another window

#### Scenario: Empty categories are not shown
- **WHEN** Zero has no implemented settings for a potential category such as Appearance
- **THEN** the system does not display an empty destination for that category

#### Scenario: Extension management is separated from General
- **WHEN** the user views General settings
- **THEN** market, package installation, permission review, plugin lifecycle, restore, and diagnostic workflows are absent from General and remain reachable from Extensions

### Requirement: Search across available settings
The system SHALL let users search all currently available preferences by localized setting title, description, category, and tool name.

#### Scenario: Search returns settings from multiple destinations
- **WHEN** the user enters a query that matches settings in more than one destination
- **THEN** the system displays matching results with enough destination context to distinguish where each result belongs

#### Scenario: Search result opens and focuses a setting
- **WHEN** the user activates a search result
- **THEN** the system selects the result's destination, brings the matching setting into view, and moves focus to that setting or its primary control

#### Scenario: Search has no matches
- **WHEN** the query does not match any available setting
- **THEN** the system displays a localized no-results state without changing any preference

#### Scenario: Search is cleared
- **WHEN** the user clears the search query or presses Escape from the search field
- **THEN** the system restores the category navigation and preserves the currently selected destination

### Requirement: Responsive preferences layout
The system SHALL provide a two-pane preferences layout at regular desktop widths and a usable single-pane navigation flow when the resizable window becomes too narrow for both panes.

#### Scenario: Regular-width window uses two panes
- **WHEN** the preferences window has sufficient width
- **THEN** the system keeps search and category navigation visible beside one independently scrolling content pane

#### Scenario: Narrow window uses navigation and content views
- **WHEN** the preferences window is resized below the two-pane breakpoint
- **THEN** the system shows a readable navigation/search view or the selected content view instead of squeezing both panes together

#### Scenario: User returns from narrow content view
- **WHEN** the user activates Back to Settings from a narrow content view
- **THEN** the system returns to the navigation/search view without discarding saved settings or the selected destination

#### Scenario: Window returns to regular width
- **WHEN** the user widens the preferences window after selecting a destination in narrow mode
- **THEN** the system restores the two-pane layout with the same destination selected

### Requirement: Immediate-apply General settings
The system SHALL keep launch-at-login and language settings in General, apply changes immediately through their existing authoritative owners, and SHALL NOT require Save, Cancel, Apply, or OK actions.

#### Scenario: User changes language
- **WHEN** the user selects System, Chinese, or English as the application language
- **THEN** the system persists the preference immediately and updates preferences navigation and content to the resolved language

#### Scenario: User changes login startup
- **WHEN** the user enables or disables Open at Login
- **THEN** the system requests the native autostart change immediately and reflects the confirmed result

#### Scenario: Login startup update fails
- **WHEN** the native autostart owner rejects a requested change
- **THEN** the control returns to the last confirmed state and displays a localized error beside the affected setting

### Requirement: Distinct tool state controls
The system SHALL present plugin enabled state, visibility in Zero navigation, and visibility in the native status bar as distinct controls with descriptions of their separate effects.

#### Scenario: User disables a plugin
- **WHEN** the user disables a plugin from a tool or Extensions destination
- **THEN** the plugin becomes inactive while its stored navigation and status-bar visibility choices remain available for restoration when the plugin is enabled again

#### Scenario: User changes navigation visibility
- **WHEN** the user changes whether an enabled tool is shown in Zero navigation
- **THEN** the main and tray tool navigation reflect that choice without changing plugin installation, plugin enabled state, or status-bar visibility

#### Scenario: User attempts to hide the last visible tool
- **WHEN** the user attempts to hide the last tool allowed by the existing at-least-one-visible rule
- **THEN** the system keeps an eligible tool visible and explains why the requested state was not accepted

#### Scenario: Tool lacks a status-bar contribution
- **WHEN** an installed tool does not contribute a status-bar item
- **THEN** its tool destination does not present a misleading status-bar visibility switch

### Requirement: Tool overview and detail destinations
The system SHALL derive the Tools overview and per-tool destinations from installed plugin records and SHALL expose host-owned controls and available read-only metadata without inventing unsupported tool-specific settings.

#### Scenario: Installed tools appear in preferences
- **WHEN** the plugin registry contains bundled or installed tools
- **THEN** the Tools navigation and overview identify each tool using its current display name and state

#### Scenario: User opens a tool destination
- **WHEN** the user selects a tool from the overview or navigation
- **THEN** the system displays its name, version or source metadata when available, health state, and the host controls applicable to that tool

#### Scenario: Tool owns a global shortcut
- **WHEN** a tool has a shortcut in the native shortcut snapshot
- **THEN** the tool destination displays the same read-only shortcut and registration state shown in Keyboard Shortcuts

#### Scenario: Tool has no supported configurable fields
- **WHEN** a tool has no host control or read-only capability beyond its metadata
- **THEN** the system displays a localized explanation instead of generating controls from incomplete manifest setting declarations

### Requirement: Complete status-bar settings destination
The system SHALL group global status-bar display, launch restoration, current collapsed or expanded state, arrangement preview, and per-tool status-bar visibility in the Status Bar destination.

#### Scenario: User changes status-bar display settings
- **WHEN** the user changes global display, launch restoration, collapse state, or a tool's status-bar visibility
- **THEN** the system applies the update immediately through the Rust-owned status-bar settings and refreshes the native layout and preview from the confirmed state

#### Scenario: Global status-bar tool display is disabled
- **WHEN** the user disables status-bar tool icons
- **THEN** dependent launch, collapse, and per-tool controls remain visible but disabled with explanatory text while the primary Zero entry remains available

#### Scenario: Native collapse state changes while preferences is open
- **WHEN** the user expands or collapses tool icons through the native primary status item while the preferences window is open
- **THEN** the Status Bar destination and arrangement preview update to the newly persisted state without requiring the user to close and reopen preferences

#### Scenario: Status-bar update fails
- **WHEN** a requested status-bar update cannot be persisted or applied to the native layout
- **THEN** the affected control returns to the last confirmed state and displays a localized error beside the status-bar group

### Requirement: Read-only native global shortcut status
The system SHALL display Zero Snap and Zero Launch global shortcut accelerators and registration states from the native shortcut owner without offering shortcut editing in this change.

#### Scenario: Shortcut is registered
- **WHEN** the native shortcut owner reports a shortcut as registered
- **THEN** Keyboard Shortcuts displays its owning tool, accelerator, and active state

#### Scenario: Shortcut is inactive or conflicted
- **WHEN** a plugin is disabled, a platform does not support the action, or native registration fails
- **THEN** the system displays an inactive, unsupported, or error state and does not claim that the shortcut is active

#### Scenario: User views shortcut rows
- **WHEN** the user navigates or tabs through Keyboard Shortcuts
- **THEN** shortcut accelerators are presented as read-only information without editable fields, recording controls, or an Apply action

### Requirement: Dedicated Extensions workflows
The system SHALL provide Extensions as the dedicated destination for market refresh, local package validation, permission approval, installed-plugin lifecycle, bundled-plugin restoration, and extension diagnostics while preserving existing security checks.

#### Scenario: User installs a market or local extension
- **WHEN** the user selects an available market package or validates a local `.zplugin` package
- **THEN** the system requires the existing permission review and explicit approval before installation

#### Scenario: User manages an installed plugin
- **WHEN** the user enables, disables, retries, or uninstalls an installed plugin
- **THEN** the system runs the existing plugin lifecycle operation and refreshes tool navigation and affected settings destinations from the resulting registry state

#### Scenario: User restores bundled plugins
- **WHEN** the user activates Restore Bundled Defaults
- **THEN** the system restores the bundled plugin records and refreshes the Tools and Extensions destinations

#### Scenario: Extension operation fails
- **WHEN** market, validation, installation, lifecycle, restore, or diagnostic loading fails
- **THEN** the system displays a localized error within the Extensions section that initiated or owns the operation

### Requirement: Contextual settings feedback
The system SHALL communicate settings progress and results near the affected control or section and SHALL NOT display a permanent global Saved indicator.

#### Scenario: Asynchronous setting is pending
- **WHEN** an immediate-apply operation is waiting for its authoritative owner
- **THEN** the system exposes a localized pending state and prevents unsafe duplicate activation of the affected control

#### Scenario: Setting is saved
- **WHEN** the authoritative owner confirms an immediate-apply change
- **THEN** the system briefly announces localized success near the affected setting without requiring dismissal

#### Scenario: Setting fails
- **WHEN** an immediate-apply operation fails
- **THEN** the system retains a localized actionable error near the affected setting until the user retries or changes that setting again

### Requirement: Localized and accessible settings center
The system SHALL provide Chinese and English copy for all preferences and extension-management content and SHALL support keyboard and assistive-technology operation across navigation, search, settings, and feedback.

#### Scenario: Preferences uses Chinese
- **WHEN** the resolved application language is Chinese
- **THEN** navigation, search, setting labels and descriptions, extension workflows, statuses, empty states, and errors are presented in Chinese

#### Scenario: Preferences uses English
- **WHEN** the resolved application language is English
- **THEN** navigation, search, setting labels and descriptions, extension workflows, statuses, empty states, and errors are presented in English

#### Scenario: Keyboard-only navigation
- **WHEN** the user operates preferences without a pointer
- **THEN** every navigation item and control is reachable in logical order, the current destination is programmatically identifiable, and visible focus is preserved

#### Scenario: Assistive technology reads a setting
- **WHEN** a screen reader reaches a setting with descriptive text or feedback
- **THEN** the setting has an accessible name, its description is associated with the control, and pending, success, or error changes are announced with an appropriate live region

#### Scenario: Reduced motion is requested
- **WHEN** the operating system requests reduced motion
- **THEN** navigation and feedback remain understandable without nonessential animated transitions
