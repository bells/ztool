## ADDED Requirements

### Requirement: macOS status item uses compact cells
On macOS, the system SHALL render the primary Zero icon and every expanded visible tool icon as cells within one native status item. Each cell SHALL be 22pt wide after real-device comparison with 24pt, and the shared cell width MUST be defined in one macOS-specific configuration value so it can be tuned consistently without changing individual item logic.

#### Scenario: Expanded icons use compact cells
- **WHEN** Zero creates or refreshes the macOS status bar while the tool group is expanded
- **THEN** one native status item renders every visible tool icon followed by the primary Ø icon in shared 22pt cells

#### Scenario: Compact cell width is tuned centrally
- **WHEN** the shared compact-width value is adjusted
- **THEN** the primary and tool cells use the new value without per-icon overrides

### Requirement: Tool items can be collapsed and expanded persistently
The system SHALL persist whether the macOS tool-item group is collapsed. Collapsing SHALL shrink the grouped native item to the primary Zero cell so no tool space remains; expanding SHALL grow the same native item to include each visible tool cell.

#### Scenario: User collapses visible tool items
- **WHEN** the user selects “Collapse Tool Icons” from the primary Zero item's right-click menu
- **THEN** the grouped native item shrinks to the primary cell without reserved blank slots, the primary remains visible, and the collapsed state is persisted

#### Scenario: User expands collapsed tool items
- **WHEN** the user selects “Expand Tool Icons” from the primary Zero item's right-click menu
- **THEN** the same native status item grows leftward to restore all visible tool cells next to its primary Ø cell instead of creating icons at the leading edge of the menu bar, the primary remains continuously visible, and the expanded state is persisted

#### Scenario: Restart restores collapsed state
- **WHEN** Zero starts with the persisted tool-item group marked collapsed
- **THEN** it creates one native status item at the primary-cell width before presenting the native layout

#### Scenario: Existing settings migrate as expanded
- **WHEN** Zero loads a status-bar settings file created before the collapsed-state field existed
- **THEN** it preserves the existing settings values and defaults the tool-item group to expanded

#### Scenario: Native refresh preserves layout state
- **WHEN** a settings update, plugin lifecycle event, or stateful tool-icon update refreshes the macOS status items
- **THEN** the grouped item reapplies its template image and persisted collapsed or expanded width without changing that state or its native position

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
- **THEN** the existing grouped native item remains registered and shrinks to its primary Ø cell

#### Scenario: Primary menu offers quit as the second action
- **WHEN** the user opens the primary Ø item's right-click menu
- **THEN** the collapse or expand action is first and “Quit Zero Status Bar” is second in the applicable native language

#### Scenario: User quits from the primary menu
- **WHEN** the user selects “Quit Zero Status Bar”
- **THEN** Zero exits cleanly

#### Scenario: Expanded tool actions remain direct
- **WHEN** the tool group is expanded and the user clicks a tool cell
- **THEN** Zero runs the tool's existing approved native action without toggling the tray panel or collapse state

#### Scenario: Tool menu offers quit as its first action
- **WHEN** the user right-clicks any expanded macOS tool cell
- **THEN** its first menu item is “Quit Zero Status Bar” in the applicable native language

#### Scenario: User quits from a tool menu
- **WHEN** the user selects “Quit Zero Status Bar” from any tool cell's menu
- **THEN** Zero exits cleanly without running that tool's left-click action

### Requirement: Non-macOS platforms retain the single-icon fallback
On Windows and other non-macOS fallback platforms, the system SHALL keep only the primary Zero tray icon and SHALL expose tool actions through the existing tray quick-panel fallback. These platforms MUST NOT create separate tool tray icons or expose a collapse control for native tool icons that do not exist.

#### Scenario: Windows keeps one native tray icon
- **WHEN** Zero runs on Windows regardless of the persisted collapsed-state value
- **THEN** Windows displays only the primary Zero tray icon and keeps the tool actions available in the tray quick panel

#### Scenario: Fallback actions ignore macOS layout state
- **WHEN** a non-macOS fallback platform loads settings with the tool-item group marked collapsed
- **THEN** the fallback action row remains available and no separate native tool icons are created or hidden
