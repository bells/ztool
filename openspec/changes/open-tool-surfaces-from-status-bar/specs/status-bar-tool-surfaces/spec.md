## ADDED Requirements

### Requirement: Launch status-bar activation matches the global shortcut
On macOS, the system SHALL route a direct activation of the visible Zero Launch status-bar glyph to the same dedicated Launch window entry point used by `CommandOrControl+Shift+Space`, without opening the general Zero shell first.

#### Scenario: User clicks Launch while its window is hidden
- **WHEN** the user directly clicks the visible Launch glyph in the macOS status bar
- **THEN** the centered Launch window is shown and focused with the search input ready for typing
- **AND** the main or tray Zero shell is not shown as part of that activation

#### Scenario: Launch is invoked again
- **WHEN** the Launch window is already visible and the user clicks the Launch glyph again
- **THEN** the existing Launch window hides instead of creating another window

#### Scenario: Launch shortcut is invoked again
- **WHEN** the Launch window is already visible and the user invokes the global shortcut again
- **THEN** the existing Launch window is brought forward, focused, and reset through the same shown lifecycle without creating a duplicate window

#### Scenario: Launch is dismissed
- **WHEN** the user presses Escape or the Launch window loses focus outside an active launch operation
- **THEN** the Launch window hides without disabling Zero Launch or clearing its persisted index and usage data

### Requirement: Paper opens in a dedicated glyph-anchored surface
On macOS, the system SHALL show Zero Paper in a dedicated transient window positioned below the activated Paper status-bar glyph and SHALL keep the complete window within the clicked display's usable bounds.

#### Scenario: User clicks Paper while its window is hidden
- **WHEN** the user directly clicks the visible Paper glyph in the macOS status bar
- **THEN** one Paper window is shown and focused immediately below that glyph
- **AND** the window is horizontally aligned to the Paper glyph unless screen-edge clamping is required

#### Scenario: Paper is near a display edge
- **WHEN** the preferred Paper position would place any part of the window outside the clicked display's usable bounds
- **THEN** the system clamps the window position so the full surface remains visible on that display

#### Scenario: Precise glyph geometry is unavailable
- **WHEN** Zero can resolve the Paper activation but cannot obtain usable native cell or monitor geometry
- **THEN** the system uses a safe tray-relative fallback position and still shows one usable Paper window

#### Scenario: User clicks Paper again
- **WHEN** the Paper window is visible and the user clicks the Paper glyph again
- **THEN** the existing Paper window hides instead of creating another window

#### Scenario: Paper loses focus
- **WHEN** the Paper window loses focus and no Paper action is using an operating-system-owned dialog
- **THEN** the Paper window hides and retains its persisted wallpaper cache

### Requirement: The dedicated Paper surface contains only Paper tool content
The dedicated Paper window SHALL render the existing Zero Paper experience and MUST NOT render the general Zero shell, tool list, plugin navigation, preferences, about, quit controls, or content owned by another tool.

#### Scenario: Paper content loads from cache
- **WHEN** the dedicated Paper window opens with valid wallpaper cache available
- **THEN** it renders the existing Paper wallpaper card, preview, metadata, navigation, download, apply, loading, stale, and error behaviors using the same hook and typed IPC services as the main-shell Paper panel

#### Scenario: User performs a Paper action
- **WHEN** the user navigates wallpapers, downloads an image, applies an image, refreshes, or retries from the dedicated Paper surface
- **THEN** the action follows the existing Paper service and Rust cache contracts and exposes the same success, busy, and error semantics as the main-shell panel

#### Scenario: Paper surface becomes narrow
- **WHEN** Paper content must fit the dedicated compact window
- **THEN** the existing content reflows without horizontal overflow and all actions remain keyboard accessible with visible focus indicators

### Requirement: Dedicated tool surfaces coordinate without changing status-bar state
The system SHALL prevent Zero's transient tray, Launch, and Paper surfaces from obscuring each other while preserving status-bar visibility, collapse state, and menu behavior.

#### Scenario: User switches from Paper to Launch
- **WHEN** Paper is visible and the user activates the Launch glyph
- **THEN** Paper and the tray surface hide before the Launch window is shown and focused

#### Scenario: User switches from Launch to Paper
- **WHEN** Launch is visible and the user activates the Paper glyph
- **THEN** Launch and the tray surface hide before the Paper window is positioned, shown, and focused

#### Scenario: User collapses and expands tool glyphs
- **WHEN** the user collapses or expands tool glyphs from the primary Zero status-bar menu
- **THEN** the persisted collapse behavior, primary glyph, visible-tool preferences, and direct Launch and Paper activation behavior remain intact

#### Scenario: User uses a tool quit menu
- **WHEN** the user right-clicks a tool glyph and invokes its existing quit action
- **THEN** Zero follows the existing quit behavior and does not treat the action as a Launch or Paper surface activation

### Requirement: Fallback-platform tool activation remains compatible
On platforms that use the single primary tray icon and fallback action row, the system SHALL preserve the existing generic tool-opening path rather than requiring macOS status-item geometry or a Paper-anchored window.

#### Scenario: User invokes Launch or Paper from a fallback action row
- **WHEN** a non-macOS user activates Launch or Paper through the existing fallback action row
- **THEN** Zero opens the tool through the existing main-window plugin navigation behavior
- **AND** no macOS-only positioning operation is attempted
