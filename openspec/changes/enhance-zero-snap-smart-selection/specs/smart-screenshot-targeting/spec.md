## ADDED Requirements

### Requirement: Custom screenshot starts in target selection mode
On macOS, the Zero custom screenshot overlay SHALL start without a committed selection and SHALL let the user choose a window candidate, the complete captured image, or a freely drawn rectangle before editing or export becomes available.

#### Scenario: Capture overlay becomes ready
- **WHEN** the macOS custom overlay finishes loading a valid capture session
- **THEN** no screenshot selection is committed yet
- **AND** the overlay enters target selection mode instead of activating the complete image selection

#### Scenario: No native window candidates are available
- **WHEN** the capture session contains no valid native window candidates
- **THEN** the complete captured image remains available as a fallback target
- **AND** free rectangle dragging remains available

#### Scenario: User commits a target
- **WHEN** the user commits a valid window, complete-image, or free rectangle target
- **THEN** that source-pixel rectangle becomes the current screenshot selection
- **AND** the existing selection adjustment, annotation, Copy, Save, and Pin flows become available for it

### Requirement: Visible top-level windows provide ordered snap candidates
The system SHALL provide session-scoped macOS window candidates in front-to-back order and SHALL convert each candidate into valid source-image pixel bounds before sending it to the capture overlay.

#### Scenario: Pointer overlaps multiple windows
- **WHEN** the pointer lies inside two or more valid overlapping window candidates
- **THEN** the system previews the frontmost candidate from the session ordering

#### Scenario: Window crosses the captured image edge
- **WHEN** a visible window intersects but is not fully contained by the current captured image
- **THEN** its candidate is clipped to the image bounds
- **AND** the resulting rectangle remains positive and valid

#### Scenario: Window belongs to Zero or cannot be captured
- **WHEN** a window belongs to the current Zero process, is minimized or invisible, has invalid geometry, is an identified desktop background layer, or does not intersect the captured display
- **THEN** the system excludes it from snap candidates

#### Scenario: Candidate enumeration fails
- **WHEN** native window enumeration or candidate conversion fails before the overlay opens
- **THEN** screenshot capture continues with an empty native candidate list
- **AND** the failure does not bypass the existing screenshot permission or session error handling

#### Scenario: Candidate data crosses IPC
- **WHEN** the capture session is returned to the capture window
- **THEN** each candidate contains only an opaque session id, a supported target kind, and source-image bounds
- **AND** application name, title, process id, z coordinate, and global display coordinates are not exposed to React

### Requirement: Target preview follows pointer location
While target selection is active, the overlay SHALL preview the valid target under the pointer without treating the preview as an exportable committed selection.

#### Scenario: Pointer enters a window candidate
- **WHEN** target selection is active and the pointer enters a valid window candidate
- **THEN** the overlay highlights exactly that candidate boundary
- **AND** no toolbar, resize handle, or export action is activated from the preview alone

#### Scenario: Pointer moves from one application window to another
- **WHEN** the pointer moves between valid non-overlapping window candidates
- **THEN** the preview switches to the candidate containing the current pointer

#### Scenario: Pointer is over captured background
- **WHEN** no native window candidate contains the pointer but the pointer remains inside the captured image
- **THEN** the overlay previews the complete-image fallback target

#### Scenario: Pointer leaves the rendered image
- **WHEN** the pointer leaves the actual rendered capture image
- **THEN** the overlay clears the target preview

### Requirement: Click snapping and free dragging share one deterministic gesture
The overlay SHALL distinguish a click that commits the current snap candidate from a drag that creates a free rectangle, and SHALL allow only one selection gesture for a pointer id.

#### Scenario: User clicks a stable candidate
- **WHEN** pointer down and pointer up remain within the same candidate without exceeding the drag threshold
- **THEN** the system commits that candidate bounds as the selection

#### Scenario: User drags from a candidate
- **WHEN** pointer movement exceeds the configured CSS-pixel drag threshold before release
- **THEN** the gesture becomes free rectangle creation from the original pointer-down position
- **AND** the original snap candidate no longer controls the draft

#### Scenario: Free drag is released
- **WHEN** the user releases a free rectangle drag with valid minimum source-pixel dimensions
- **THEN** the normalized drag rectangle becomes the committed selection

#### Scenario: Selection gesture is cancelled
- **WHEN** pointer capture is lost, the pointer is cancelled, or Escape cancels an uncommitted target gesture
- **THEN** the system clears its draft and pointer ownership
- **AND** returns to target selection without exporting content

### Requirement: Perpendicular pointer guides aid initial selection
The overlay SHALL render one horizontal and one vertical guide whose intersection follows the pointer while target selection or free rectangle creation is active.

#### Scenario: Pointer moves inside the captured image
- **WHEN** target selection is active and the pointer moves inside the rendered image
- **THEN** the horizontal guide crosses the rendered image at the pointer y coordinate
- **AND** the vertical guide crosses the rendered image at the pointer x coordinate

#### Scenario: User creates a free rectangle
- **WHEN** a target click transitions into a free rectangle drag
- **THEN** both guides continue following the active pointer until commit or cancellation

#### Scenario: Selection is committed
- **WHEN** a window, full-image, or free rectangle selection is committed
- **THEN** both pointer guides are hidden

#### Scenario: Screenshot is exported
- **WHEN** the user copies, saves, or pins the screenshot
- **THEN** pointer guides and target preview chrome are absent from the exported PNG

### Requirement: Smart targeting preserves platform boundaries
The system MUST limit this custom smart-targeting behavior to platforms that use the Zero custom capture overlay.

#### Scenario: Screenshot starts on Windows
- **WHEN** Zero Snap starts on Windows
- **THEN** the existing system screenshot launcher path remains in control
- **AND** Zero does not claim that custom window candidates or pointer guides are active

#### Scenario: Screenshot starts on an unsupported platform
- **WHEN** Zero Snap starts on Linux or another currently unsupported platform
- **THEN** the existing explicit unsupported or launch error behavior remains unchanged

#### Scenario: macOS candidate tests pass
- **WHEN** source and pure-model tests validate candidate ordering and coordinates
- **THEN** the project still treats real Screen Recording permission, z-order, Retina, and multi-display behavior as separate runtime verification

### Requirement: Capture content is ready before the overlay becomes visible
The system SHALL keep a custom capture overlay hidden while its session and frozen screenshot content are preparing, and SHALL reveal it atomically only after the image is decoded and committed to the capture DOM.

#### Scenario: macOS capture content becomes ready
- **WHEN** the capture window has initialized the active session, read the scoped PNG bytes, decoded the image, and committed the image element to the DOM
- **THEN** the capture window requests reveal exactly once for that active session
- **AND** its first visible frame contains the frozen screenshot instead of an empty WebView page

#### Scenario: macOS overlay covers frozen system chrome
- **WHEN** the ready capture overlay is revealed on macOS
- **THEN** the borderless overlay covers the complete selected display including its menu bar and Dock areas
- **AND** the user sees only the system chrome frozen inside the screenshot rather than a second live menu bar or Dock above it

#### Scenario: Capture overlay is resized from its hidden default frame
- **WHEN** the hidden capture WebView is prepared for a target display whose physical size differs from the WebView default size
- **THEN** the system applies the complete target-display size before its global position
- **AND** the final overlay top-left and extent match the target display instead of being shifted or clipped by the native resize anchor

#### Scenario: Target display uses a negative or vertically stacked origin
- **WHEN** the selected capture display has a negative x or y origin, or is arranged above or below another display
- **THEN** the overlay preserves that display's complete global bounds
- **AND** its frozen screenshot, pointer guides, candidates, and selection coordinates remain aligned across the complete display

#### Scenario: Capture preparation or reveal fails
- **WHEN** session initialization, media loading, image decoding, native configuration, or reveal fails
- **THEN** the hidden capture window is closed
- **AND** its session resources are released and the previous Zero shell surface is restored

#### Scenario: Windows screenshot starts
- **WHEN** the Windows system screenshot launcher is started
- **THEN** Zero creates no custom capture WebView that could expose a blank frame or duplicate system chrome
- **AND** the system launcher remains solely responsible for screenshot presentation
