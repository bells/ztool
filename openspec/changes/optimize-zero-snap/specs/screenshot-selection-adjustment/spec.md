## ADDED Requirements

### Requirement: Screenshot selection supports eight-direction pointer resizing
The capture editor SHALL expose four interactive corner handles and four interactive edge handles that resize the current screenshot selection in source-image pixel coordinates.

#### Scenario: User drags a corner handle
- **WHEN** the user drags any circular corner handle while a valid screenshot selection exists
- **THEN** the system moves the two adjacent selection edges on both axes while keeping the opposite corner fixed

#### Scenario: User drags a horizontal edge handle
- **WHEN** the user drags the top or bottom edge handle
- **THEN** the system changes only the corresponding vertical edge and selection height while preserving x and width

#### Scenario: User drags a vertical edge handle
- **WHEN** the user drags the left or right edge handle
- **THEN** the system changes only the corresponding horizontal edge and selection width while preserving y and height

#### Scenario: Handle approaches its opposite edge
- **WHEN** a resize gesture would make selection width or height smaller than the configured minimum
- **THEN** the active edge stops at the minimum size and does not cross or silently exchange roles with the fixed edge

#### Scenario: Handle reaches the captured image boundary
- **WHEN** a resize gesture moves an active edge beyond the original screenshot dimensions
- **THEN** the system clamps that edge to the image boundary and keeps the complete selection valid

#### Scenario: Pointer leaves the visible handle during resize
- **WHEN** an active resize pointer moves outside the handle or selection before release
- **THEN** pointer capture keeps the same resize gesture active until it is committed or cancelled

#### Scenario: Resize gesture is cancelled
- **WHEN** the resize pointer is cancelled, capture is lost, or the user presses Escape before commit
- **THEN** the system restores the selection from before the gesture and clears the resize draft

### Requirement: Selection adjustments provide live, non-exported feedback
The system SHALL use the active resize draft as the single source for capture-editor selection feedback while keeping all adjustment UI out of the exported screenshot.

#### Scenario: User resizes a selection
- **WHEN** a corner or edge drag changes the resize draft
- **THEN** the visible selection border, outside dimming, source-pixel dimension badge, and selection-relative toolbar position update from that draft

#### Scenario: User releases a valid resize
- **WHEN** the user releases the active resize pointer with a valid draft
- **THEN** the system commits that draft as the current screenshot selection and keeps Copy and Save cropped to it

#### Scenario: User exports the adjusted selection
- **WHEN** the user copies, saves, or pins content after adjusting the selection
- **THEN** the output uses the current source-image selection and excludes handles, cursors, size badges, toolbar, and outside dimming

### Requirement: Arrow keys move the complete selection by source pixels
When the Select tool is active and no text or pointer selection edit is in progress, the system SHALL let unmodified arrow keys move the complete screenshot selection by one source-image pixel per keydown while preserving its dimensions.

#### Scenario: User presses an arrow key
- **WHEN** the Select tool is active and the user presses Left, Right, Up, or Down with a valid movable selection
- **THEN** the system changes x or y by exactly one original screenshot pixel in that direction
- **AND** width and height remain unchanged

#### Scenario: User holds an arrow key
- **WHEN** the operating system emits repeated keydown events for a held arrow key
- **THEN** the selection continues moving one source pixel for each event and the live frame, dimensions, and toolbar anchor stay synchronized

#### Scenario: Selection reaches an image edge
- **WHEN** an arrow key would move any part of the selection outside the original screenshot
- **THEN** the system clamps the selection at that edge without shrinking it or changing the opposite coordinate

#### Scenario: Selection fills an axis
- **WHEN** the selection width equals image width or selection height equals image height
- **THEN** arrow input along that full axis leaves the selection unchanged and valid

#### Scenario: Text input or pointer adjustment is active
- **WHEN** the user is editing a text annotation, using an input method, or performing a selection pointer gesture
- **THEN** the global selection nudge handler does not consume arrow keys from that interaction

#### Scenario: Modified arrow shortcut is pressed
- **WHEN** Command, Control, Option, or Alt participates in an arrow-key shortcut
- **THEN** the selection nudge handler leaves the shortcut untouched

### Requirement: Selection adjustment coexists with selection creation and annotations
The system MUST keep selection resize and keyboard movement distinct from creating a new selection and from selecting or drawing annotations.

#### Scenario: User starts on a resize handle
- **WHEN** the Select tool is active and pointer down hits a selection handle
- **THEN** resize takes precedence and the same pointer does not start a new selection or annotation

#### Scenario: User drags annotation-free canvas
- **WHEN** the Select tool is active and the user drags an area that hits neither a handle nor an existing annotation
- **THEN** the existing normalized new-selection behavior remains available

#### Scenario: User clicks an existing annotation
- **WHEN** the Select tool is active and pointer input hits an existing annotation but no selection handle
- **THEN** the annotation remains selectable and the screenshot selection bounds do not change

#### Scenario: Selection is adjusted
- **WHEN** the user resizes or nudges the screenshot selection
- **THEN** the system does not create an annotation or add the adjustment to the annotation undo/redo history
