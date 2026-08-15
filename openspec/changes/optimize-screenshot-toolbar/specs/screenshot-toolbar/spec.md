## ADDED Requirements

### Requirement: Capture editor uses a borderless screen overlay
The macOS custom capture editor MUST appear as a borderless overlay sized to the target display without entering the operating system's native fullscreen window mode.

#### Scenario: Screenshot shortcut opens the editor
- **WHEN** the user starts a macOS custom screenshot session
- **THEN** the system creates the capture window hidden, disables window decorations, sizes it to the primary display's physical bounds, and only then shows and focuses it

#### Scenario: Capture overlay becomes visible
- **WHEN** the prepared capture window is shown
- **THEN** no system title bar or native fullscreen Space transition is visible

#### Scenario: Capture window preparation fails
- **WHEN** positioning, sizing, showing, or focusing the hidden capture window fails
- **THEN** the system closes that window and returns an error through the existing session cleanup path

### Requirement: Screenshot selection is distinct from annotations
The system SHALL maintain a real screenshot selection in source-image pixel coordinates independently from rectangle and other annotations.

#### Scenario: Capture editor opens
- **WHEN** a macOS custom screenshot session is initialized
- **THEN** the system selects the complete captured image and displays the full-screen selection frame

#### Scenario: User drags a new selection
- **WHEN** the Select tool is active and the user drags on an area that does not select an existing annotation
- **THEN** the system creates a normalized screenshot selection from the drag bounds without adding a rectangle annotation

#### Scenario: User clicks without creating a valid selection
- **WHEN** a Select drag does not reach the minimum selection size
- **THEN** the system preserves the previous valid screenshot selection

#### Scenario: Rectangle annotation exists
- **WHEN** the user creates or selects a Rectangle annotation
- **THEN** the system keeps the screenshot selection unchanged and treats the rectangle only as an annotation

### Requirement: Screenshot commits use the real selection
The system MUST crop copy and save output to the current screenshot selection after rendering annotations.

#### Scenario: User copies a selected region
- **WHEN** the current screenshot selection covers less than the complete image and the user activates Copy
- **THEN** the system renders the current annotations, crops the rendered image to the screenshot selection, and sends only that cropped PNG through the existing commit command

#### Scenario: User saves a full-screen selection
- **WHEN** the current screenshot selection covers the complete image and the user activates Save
- **THEN** the system submits the complete rendered screenshot without adding the selection frame or outside dimming to the image

#### Scenario: Annotation crosses the selection edge
- **WHEN** an annotation extends beyond the current screenshot selection
- **THEN** the final copy or save output contains only the part of the rendered annotation inside the selected crop

#### Scenario: macOS writes the copied PNG
- **WHEN** the macOS custom editor commits a screenshot with the Copy action
- **THEN** the system writes the submitted bytes to the native pasteboard as PNG data without depending on AppleScript image-type syntax

### Requirement: Selection coordinates map consistently to the viewport
The system SHALL derive the visible selection frame and toolbar anchor from the same source-image selection across display scale and object-fit offsets.

#### Scenario: Captured image is scaled in the window
- **WHEN** the captured image pixel dimensions differ from its CSS viewport dimensions
- **THEN** the system maps selection position and size using the contain scale and centering offsets used for pointer coordinates

#### Scenario: Selection is being dragged
- **WHEN** the user updates a valid selection draft
- **THEN** the visible frame, size badge, outside dimming, and toolbar anchor reflect the current draft without writing those decorations into the exported image

### Requirement: Toolbar follows the screenshot selection
The system SHALL position the Zero Snap editing toolbar relative to the current screenshot selection instead of fixing it to the viewport bottom.

#### Scenario: Space is available below the selection
- **WHEN** the current screenshot selection has enough visible space below it for the complete toolbar and the configured gap
- **THEN** the system places the toolbar immediately below the selection and keeps the toolbar inside the viewport safe margins

#### Scenario: Full-screen selection has no space below
- **WHEN** the current screenshot selection reaches the bottom viewport boundary and is tall enough to contain the toolbar
- **THEN** the system places the toolbar inside the selection along its bottom edge with the configured inner inset

#### Scenario: Short selection cannot contain the toolbar
- **WHEN** there is not enough space below the selection and placing the toolbar inside would not preserve the minimum inner spacing
- **THEN** the system prefers a complete position above the selection and otherwise clamps the toolbar to the visible viewport

### Requirement: Toolbar remains visible at viewport boundaries
The system MUST keep every toolbar action visible and reachable when the screenshot selection or viewport dimensions change.

#### Scenario: Selection is narrower than the toolbar
- **WHEN** the selection width is smaller than the measured toolbar width
- **THEN** the system may extend the toolbar beyond the selection but MUST clamp its horizontal position within the viewport safe margins

#### Scenario: Selection touches a horizontal edge
- **WHEN** right-alignment with the selection would move any part of the toolbar beyond the left or right viewport edge
- **THEN** the system shifts the toolbar horizontally until the complete toolbar is visible inside the safe margins

#### Scenario: Selection or viewport changes
- **WHEN** the screenshot selection bounds, toolbar dimensions, or viewport dimensions change
- **THEN** the system recalculates the toolbar placement from the current measured values without returning to a fixed viewport-bottom position

### Requirement: Toolbar actions use consistent graphical icons
The system SHALL represent the visible Select, Rectangle, Ellipse, Arrow, Pen, Text, Mosaic, Pin, Undo, Redo, Delete, Cancel, Save, and Copy actions with consistent vector icons rather than visible English action labels.

#### Scenario: Toolbar is ready
- **WHEN** the Zero Snap editing toolbar is displayed
- **THEN** each supported tool and action is shown as a graphical icon in the existing functional order and no English short label is required for the visible button face

#### Scenario: Tool is selected
- **WHEN** the user activates an annotation tool
- **THEN** the corresponding icon button exposes a clear selected state without changing the meaning or behavior of the tool

#### Scenario: Ellipse tool is available
- **WHEN** the toolbar is displayed
- **THEN** the Ellipse icon appears immediately after Rectangle and before Arrow

#### Scenario: User draws an ellipse
- **WHEN** the Ellipse tool is active and the user drags a valid area
- **THEN** the system creates a selectable ellipse annotation inside the normalized drag bounds and includes it in undo, redo, delete, copy, and save behavior

#### Scenario: Action is unavailable or committing
- **WHEN** an action is disabled by the current editor state or a screenshot commit is in progress
- **THEN** its icon button exposes a distinct disabled state and does not invoke the action

### Requirement: Icon-only controls remain understandable and accessible
The system MUST preserve localized meaning, keyboard access, and non-color state cues for every icon-only toolbar control.

#### Scenario: User inspects an icon
- **WHEN** the user hovers or focuses a toolbar icon button
- **THEN** the system exposes a localized action name through a tooltip and an accessible name

#### Scenario: User navigates with the keyboard
- **WHEN** the user tabs through the screenshot toolbar and activates a focused control
- **THEN** focus is visibly indicated, controls follow the visual order, and activation performs the same action as pointer input

#### Scenario: Existing editor hotkey is used
- **WHEN** the user invokes an existing screenshot editor hotkey such as Escape, Delete or Backspace, undo, or redo
- **THEN** the system preserves the existing command behavior after the toolbar becomes icon-only

#### Scenario: Destructive and confirmation actions are shown
- **WHEN** delete, cancel, save, and copy actions are visible together
- **THEN** icon shape, grouping, state treatment, and accessible names distinguish their meanings without relying on English text or color alone

### Requirement: Text annotation input remains usable
The system SHALL keep the text annotation editor focused after the user chooses the Text tool and clicks inside the screenshot selection, allow multiline input, and complete editing by pointer interaction outside the editor.

#### Scenario: User starts a text annotation
- **WHEN** the Text tool is active and the user clicks a valid point inside the screenshot selection
- **THEN** the system displays a focused text input at that point without letting the canvas pointer sequence immediately dismiss it

#### Scenario: User inserts a line break
- **WHEN** the text editor is active and the user presses Enter
- **THEN** the system inserts a newline and keeps the text editor open, including while an input method is composing text

#### Scenario: User completes multiline text
- **WHEN** the text editor contains text and the user left-clicks outside it
- **THEN** the system commits one text annotation that preserves every entered line and does not create an additional empty text draft from the same click

#### Scenario: Multiline text is exported
- **WHEN** a committed text annotation contains newline characters and the user copies or saves the screenshot
- **THEN** the system renders every line with stable line spacing inside the selected crop
