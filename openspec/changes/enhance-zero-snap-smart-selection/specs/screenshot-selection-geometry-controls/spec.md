## ADDED Requirements

### Requirement: A compact geometry control is anchored to the selection
After a selection is committed, the capture overlay SHALL show a compact geometry control associated with the selection's upper-left area and SHALL keep the control reachable inside the capture viewport.

#### Scenario: Space exists above the selection
- **WHEN** a committed selection has enough viewport space above it
- **THEN** the geometry control is left-aligned near the selection's upper-left corner and placed above the selection

#### Scenario: Selection touches a viewport edge
- **WHEN** the preferred control position would extend outside the capture viewport
- **THEN** the system chooses an inside or alternate vertical placement and clamps the control to the viewport

#### Scenario: Selection is being resized
- **WHEN** an active selection resize draft changes width or height
- **THEN** the geometry control displays the draft source-pixel dimensions and follows the draft position
- **AND** its editable controls do not start a competing canvas gesture

### Requirement: Width and height are directly editable in source pixels
The geometry control SHALL expose separate integer width and height inputs that resize the selection in source-image pixels while keeping its upper-left corner fixed.

#### Scenario: User enters a valid width
- **WHEN** the user commits a valid width with Enter, Tab, or focus loss
- **THEN** the selection right edge changes to produce that source-pixel width
- **AND** x, y, and height remain unchanged

#### Scenario: User enters a valid height
- **WHEN** the user commits a valid height with Enter, Tab, or focus loss
- **THEN** the selection bottom edge changes to produce that source-pixel height
- **AND** x, y, and width remain unchanged

#### Scenario: Entered dimension exceeds available image space
- **WHEN** a committed dimension is greater than the space from the fixed upper-left corner to the corresponding image edge
- **THEN** the system limits that dimension to the available source pixels
- **AND** the selection remains inside the image

#### Scenario: Entered dimension is empty or invalid
- **WHEN** the user commits an empty, non-numeric, non-finite, or below-minimum dimension
- **THEN** the committed selection remains unchanged
- **AND** the control exposes an invalid state without propagating invalid geometry

#### Scenario: User cancels dimension editing
- **WHEN** the user presses Escape while editing width or height
- **THEN** the input restores the value from before that edit
- **AND** the screenshot session remains open

#### Scenario: Input method or input key handling is active
- **WHEN** a dimension input is focused or composing text
- **THEN** selection nudge, Delete, annotation hotkeys, and canvas pointer handlers do not consume that input interaction

### Requirement: Corner radius is adjustable and bounded
The geometry control SHALL provide a slider that changes the selection corner radius in source-image pixels from zero through half of the selection's shorter dimension.

#### Scenario: User drags the radius slider
- **WHEN** the user changes the slider value within its current range
- **THEN** the selection border and outside mask update with that source-pixel radius in real time

#### Scenario: Radius is zero
- **WHEN** the corner radius equals zero
- **THEN** the selection preview and output retain rectangular corners

#### Scenario: User requests an excessive radius
- **WHEN** a slider or geometry update would make the radius exceed half of the shorter selection dimension
- **THEN** the system clamps the radius to the valid maximum

#### Scenario: Selection becomes smaller
- **WHEN** width or height is reduced below the size required by the current radius
- **THEN** the system reduces the radius to the new valid maximum in the same geometry update

#### Scenario: Slider is keyboard controlled
- **WHEN** the radius slider has focus and receives arrow input
- **THEN** the slider adjusts through its native keyboard interaction
- **AND** the complete screenshot selection is not nudged

### Requirement: Rounded selection preview and PNG output are consistent
The system SHALL apply one normalized selection geometry to the visible rounded boundary, outside dimming, toolbar anchoring, and Copy, Save, and Pin PNG generation.

#### Scenario: Rounded selection is previewed
- **WHEN** the current radius is greater than zero
- **THEN** the outside dimming exposes a rounded-rectangle hole matching the visible selection border
- **AND** resize handles remain anchored to the selection bounds

#### Scenario: Rounded selection is exported
- **WHEN** the user copies, saves, or pins a selection whose radius is greater than zero
- **THEN** the exported PNG dimensions equal the rectangular selection width and height
- **AND** pixels outside the rounded rectangle have zero alpha

#### Scenario: Annotation crosses a rounded corner
- **WHEN** an annotation extends into pixels outside the rounded selection shape
- **THEN** the same rounded alpha mask clips both the base screenshot and the annotation

#### Scenario: Rectangular selection is exported
- **WHEN** the current radius is zero
- **THEN** the existing rectangular crop result is preserved without transparent corner removal

#### Scenario: Capture chrome is visible during export
- **WHEN** target previews, guides, geometry controls, handles, masks, errors, or toolbars are visible in the capture overlay
- **THEN** none of that capture chrome appears in the exported PNG

### Requirement: Geometry controls do not add unrequested output effects
The selection geometry controls MUST change only selection dimensions and corner radius unless a later capability explicitly adds another effect.

#### Scenario: User adds rounded corners
- **WHEN** the user sets a non-zero corner radius
- **THEN** the system does not automatically add a drop shadow, extra canvas margin, background color, OCR result, or fixed aspect ratio
