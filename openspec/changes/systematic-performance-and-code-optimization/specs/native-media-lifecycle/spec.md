## ADDED Requirements

### Requirement: Large native media avoids Base64 JSON IPC
The system SHALL transfer full-resolution screenshots, pinned images, wallpaper previews, and File-engine documents through bounded raw-byte or opaque native-resource contracts and MUST NOT serialize those large payloads as Base64 fields in JSON commands, events, or snapshots.

#### Scenario: WebView reads a screenshot session image
- **WHEN** the macOS capture editor initializes a valid screenshot session
- **THEN** it receives typed metadata and an opaque session-bound resource, reads bounded PNG bytes once, and does not receive a full-resolution `image_base64` JSON field

#### Scenario: WebView uploads a rendered screenshot
- **WHEN** the editor commits copy, save, or pin output
- **THEN** it sends validated bounded PNG bytes under a typed short-lived action lease and does not send a `png_base64` JSON field

#### Scenario: File engine exchanges a document
- **WHEN** the isolated File engine reads an input or writes an output
- **THEN** the existing raw binary request/response contract remains bounded by job identity, engine identity, deadline, and byte limits

### Requirement: Media resources have scoped ownership and expiry
Every opaque media token, temporary file, object URL, decoded image, and native cache entry SHALL have one documented owner, byte/type bounds, terminal events, and an expiry or cleanup path. A caller MUST NOT obtain a resource by supplying an arbitrary filesystem path.

#### Scenario: Invalid resource token is used
- **WHEN** a caller presents an unknown, expired, wrong-window, wrong-session, or wrong-plugin media token
- **THEN** the native service rejects access without revealing a path or resource contents

#### Scenario: Zero starts after an interrupted media workflow
- **WHEN** expired session or staging files remain from a previous crash
- **THEN** startup or first service initialization removes only files owned by the documented media roots and retains unrelated user files

### Requirement: Screenshot resources are released at terminal lifecycle events
Zero Snap SHALL release the original capture session, export canvas backing store, Blob/object URL, upload lease, temporary file, and Rust session data after successful commit, cancel, failure, expiry, or window disposal as applicable, while retaining only the resource required by a live pin window.

#### Scenario: Copy or save succeeds
- **WHEN** the native commit action completes successfully
- **THEN** the capture window closes and all session media not owned by a live pin is revoked and removed

#### Scenario: User cancels capture
- **WHEN** the user cancels or closes the capture editor
- **THEN** the session resource, temporary PNG, object URL, and pending upload lease are released idempotently

#### Scenario: Pin window closes
- **WHEN** a `pin-*` window is destroyed
- **THEN** its Rust pin record and owned media file are removed and subsequent initialization with the old token fails

### Requirement: Wallpaper previews are bounded derivatives
Zero Paper SHALL render UI previews from cached, bounded-dimension derivatives or equivalently bounded decoded resources, SHALL keep full-resolution files native for apply/save, and SHALL release a replaced preview's frontend and native transient resources.

#### Scenario: User selects a cached wallpaper
- **WHEN** a full-resolution wallpaper is available in the validated native cache
- **THEN** the panel loads a bounded preview resource while apply and save continue to use the original native file without transferring it to React

#### Scenario: User navigates between wallpapers
- **WHEN** a new preview replaces the current preview
- **THEN** the prior object URL/token is revoked or released and late completion from the older selection cannot restore it

### Requirement: Small media caches remain explicitly bounded
Launcher icons and other intentionally small inline media SHALL have documented per-item byte limits, total count or byte limits, eviction behavior, and bounded concurrent loading even when Base64 is retained for the first round.

#### Scenario: Icon cache reaches its limit
- **WHEN** adding another launcher icon would exceed the configured count or byte budget
- **THEN** the cache evicts an eligible older entry or rejects the addition without growing without bound

### Requirement: Repeated media workflows recover memory
After ten completed or cancelled repetitions of each measured screenshot, pin, wallpaper-preview, and File-render workflow followed by the documented settle interval, the system SHALL have no orphaned owned resources and SHALL keep aggregate process-tree RSS within the recorded post-workflow budget or report the gate as failed.

#### Scenario: Ten screenshot cycles complete
- **WHEN** the reference workflow performs ten capture, edit, and copy/save/cancel cycles and waits for the settle interval
- **THEN** session/pin/token/file counts return to their expected terminal values and RSS does not show unbounded monotonic retention beyond the approved budget

#### Scenario: Ten wallpaper selections complete
- **WHEN** the reference workflow replaces the preview ten times and waits for the settle interval
- **THEN** only the currently owned preview/cache entries remain and obsolete decoded/object/token resources have been released
