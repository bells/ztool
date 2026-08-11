## ADDED Requirements

### Requirement: macOS status items use one compact width
On macOS, the system SHALL apply one shared compact native width to the primary Zero status item and every expanded visible tool status item. The shared width SHALL be 22pt after real-device comparison with 24pt and MUST be defined in one macOS-specific configuration value so it can be tuned consistently without changing individual item logic.

#### Scenario: Expanded items use the compact width
- **WHEN** Zero creates or refreshes the macOS status bar while the tool group is expanded
- **THEN** the primary item and every visible tool item use the shared 22pt compact width

#### Scenario: Compact width is tuned centrally
- **WHEN** the shared compact-width value is adjusted
- **THEN** the primary item and every expanded visible tool item use the new value without per-item overrides

### Requirement: Tool items can be collapsed and expanded persistently
The system SHALL persist whether the macOS tool-item group is collapsed. Collapsing SHALL remove each tool status item's reserved menu-bar space while retaining the primary Zero item at the compact width; expanding SHALL restore each visible tool item to the shared compact width.

#### Scenario: User collapses visible tool items
- **WHEN** the user selects “Collapse Tool Icons” from the primary Zero item's right-click menu
- **THEN** all visible macOS tool status items are hidden without reserved blank slots, the primary item remains visible at the compact width, and the collapsed state is persisted

#### Scenario: User expands collapsed tool items
- **WHEN** the user selects “Expand Tool Icons” from the primary Zero item's right-click menu
- **THEN** all visible macOS tool items return to the shared compact width, the primary Ø item remains continuously visible without a visibility round trip, and the expanded state is persisted

#### Scenario: Restart restores collapsed state
- **WHEN** Zero starts with the persisted tool-item group marked collapsed
- **THEN** it creates the primary item at the compact width and keeps the tool status items hidden before presenting the native layout

#### Scenario: Existing settings migrate as expanded
- **WHEN** Zero loads a status-bar settings file created before the collapsed-state field existed
- **THEN** it preserves the existing settings values and defaults the tool-item group to expanded

#### Scenario: Native refresh preserves layout state
- **WHEN** a settings update, plugin lifecycle event, or stateful tool-icon update refreshes the macOS status items
- **THEN** the refreshed items reapply the persisted collapsed or expanded widths without changing that state

### Requirement: Primary-item interactions remain unambiguous
The primary Zero item SHALL retain its existing left-click tray-panel action and SHALL expose collapse or expand only through its native right-click menu. The menu label SHALL describe the action that will occur from the current persisted state.

#### Scenario: Left click still toggles the tray panel
- **WHEN** the user left-clicks the primary Zero item in either collapsed or expanded state
- **THEN** Zero toggles the tray quick panel without changing the tool-item collapse state

#### Scenario: Right-click menu reflects expanded state
- **WHEN** the user opens the primary item's right-click menu while tool items are expanded
- **THEN** the menu offers “Collapse Tool Icons” in the applicable native language

#### Scenario: Right-click menu reflects collapsed state
- **WHEN** the user opens the primary item's right-click menu while tool items are collapsed
- **THEN** the menu offers “Expand Tool Icons” in the applicable native language

#### Scenario: Collapse retains the primary recovery icon
- **WHEN** the user selects the collapse action from the primary item's own native menu
- **THEN** the existing primary Ø item remains visible and registered while the tool status items release their menu-bar slots

#### Scenario: Primary menu offers quit as the second action
- **WHEN** the user opens the primary Ø item's right-click menu
- **THEN** the collapse or expand action is first and “Quit Zero Status Bar” is second in the applicable native language

#### Scenario: User quits from the primary menu
- **WHEN** the user selects “Quit Zero Status Bar”
- **THEN** Zero exits cleanly

#### Scenario: Expanded tool actions remain direct
- **WHEN** the tool group is expanded and the user clicks a tool status item
- **THEN** Zero runs the tool's existing approved native action without toggling the tray panel or collapse state

#### Scenario: Tool menu offers quit as its first action
- **WHEN** the user right-clicks any expanded macOS tool status item
- **THEN** its first menu item is “Quit Zero Status Bar” in the applicable native language

#### Scenario: User quits from a tool menu
- **WHEN** the user selects “Quit Zero Status Bar” from any tool status item's menu
- **THEN** Zero exits cleanly without running that tool's left-click action

### Requirement: Non-macOS platforms retain the single-icon fallback
On Windows and other non-macOS fallback platforms, the system SHALL keep only the primary Zero tray icon and SHALL expose tool actions through the existing tray quick-panel fallback. These platforms MUST NOT create separate tool tray icons or expose a collapse control for native tool icons that do not exist.

#### Scenario: Windows keeps one native tray icon
- **WHEN** Zero runs on Windows regardless of the persisted collapsed-state value
- **THEN** Windows displays only the primary Zero tray icon and keeps the tool actions available in the tray quick panel

#### Scenario: Fallback actions ignore macOS layout state
- **WHEN** a non-macOS fallback platform loads settings with the tool-item group marked collapsed
- **THEN** the fallback action row remains available and no separate native tool icons are created or hidden
