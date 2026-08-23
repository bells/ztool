## ADDED Requirements

### Requirement: Snap status-bar activation opens an anchored tool menu
On macOS, the system SHALL route a left click on the visible Zero Snap status-bar glyph to one dedicated transient Snap menu positioned below the exact clicked glyph instead of starting a screenshot immediately.

#### Scenario: User clicks Snap while its menu is hidden
- **WHEN** the user left-clicks the visible Snap glyph in the macOS status bar
- **THEN** the system shows and focuses one compact Snap menu below the clicked Snap glyph
- **AND** the screenshot capture overlay is not started by the status-bar click alone

#### Scenario: User clicks Snap while its menu is visible
- **WHEN** the Snap menu is already visible and the user left-clicks the Snap glyph again
- **THEN** the existing menu hides without creating another window or starting a screenshot

#### Scenario: Snap is near a display edge
- **WHEN** the preferred menu position would place any part of the menu outside the clicked display's usable work area
- **THEN** the system clamps the complete menu inside that display while keeping it aligned as closely as possible to the Snap glyph

#### Scenario: Snap is clicked on a non-primary display
- **WHEN** the Snap glyph activation resolves to a display with a negative origin, vertical arrangement, or scale factor different from the primary display
- **THEN** the system uses the clicked glyph's physical geometry and that display's work area to position the menu

#### Scenario: Precise glyph geometry is unavailable
- **WHEN** the system resolves a Snap activation but cannot obtain valid glyph or monitor geometry
- **THEN** it uses a safe tray-relative fallback and still shows one usable Snap menu

### Requirement: Snap menu exposes only available typed actions
The Snap menu MUST render its entries from a stable typed action description and SHALL expose Screenshot as the first and currently only action.

#### Scenario: Initial menu version opens
- **WHEN** the Snap menu is displayed in this release
- **THEN** the first actionable row is the localized Screenshot action
- **AND** unavailable screen-recording or audio-recording placeholders are not shown

#### Scenario: Menu receives initial keyboard focus
- **WHEN** the Snap menu becomes visible
- **THEN** focus moves to its first available action and the action has a visible focus indicator and localized accessible name

#### Scenario: User activates Screenshot
- **WHEN** the user clicks the Screenshot row or activates it with Enter or Space
- **THEN** the Snap menu hides before capture begins
- **AND** the system starts the existing Zero Snap copy-oriented screenshot session exactly once

#### Scenario: Screenshot preparation fails
- **WHEN** the menu has handed off to Screenshot but capture permission, capture creation, or overlay preparation fails
- **THEN** the system restores a usable focused Snap menu and exposes the failure without starting a partial session

#### Scenario: Future actions are registered
- **WHEN** a later release adds a supported recording action to the typed action list
- **THEN** the existing menu window, focus order, dismissal, and status-bar activation lifecycle can present it without adding another status-bar dispatch path

### Requirement: Snap menu behaves as a coordinated transient surface
The system SHALL dismiss and coordinate the Snap menu like Zero's other transient tool surfaces without changing persisted tool or status-bar settings.

#### Scenario: User presses Escape
- **WHEN** keyboard focus is in the Snap menu and the user presses Escape
- **THEN** the menu hides without starting an action or disabling Zero Snap

#### Scenario: Snap menu loses focus
- **WHEN** the Snap menu loses focus outside an active screenshot handoff
- **THEN** the menu hides after the transient-surface focus check

#### Scenario: Snap opens while another Zero transient surface is visible
- **WHEN** the user activates Snap while the tray, Launch, or Paper surface is visible
- **THEN** those peer transient surfaces hide before the Snap menu is shown and focused

#### Scenario: Another tool opens while Snap is visible
- **WHEN** Launch, Paper, or the main tray surface opens while the Snap menu is visible
- **THEN** the Snap menu hides before the requested peer surface is shown

#### Scenario: Status-bar layout changes
- **WHEN** the user collapses or expands tool glyphs or changes visible tool preferences
- **THEN** the primary glyph, persisted status-bar state, right-click quit behavior, and subsequent Snap targeting remain unchanged

### Requirement: Existing direct and fallback screenshot entries remain compatible
The system MUST preserve direct screenshot entry points that do not represent a macOS Snap-glyph click.

#### Scenario: User invokes the global screenshot shortcut
- **WHEN** the user presses `CommandOrControl+Shift+A`
- **THEN** the system starts the existing screenshot flow directly without opening the Snap menu

#### Scenario: User starts Copy or Save from the main Snap panel
- **WHEN** the user activates an existing screenshot action from the main Zero Snap panel
- **THEN** the action retains its existing copy or save intent and does not require the Snap menu

#### Scenario: User invokes Snap on a fallback platform
- **WHEN** a non-macOS user activates the existing Snap item through the single-tray fallback action row
- **THEN** the existing platform screenshot launcher or unsupported error path runs without attempting macOS glyph anchoring or opening the macOS Snap menu
