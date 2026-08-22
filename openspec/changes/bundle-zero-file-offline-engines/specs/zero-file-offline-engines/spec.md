## ADDED Requirements

### Requirement: Zero File installation includes its approved offline engine assets
The system SHALL install Zero File's versioned engine bundle, dependency notices, manifest, and per-file digests atomically with the first-party plugin, MUST verify a Zero-pinned first-party signature and host compatibility before activation, and MUST NOT require or download Python, LibreOffice, Microsoft Office, ONLYOFFICE, Chromium, or another conversion runtime after installation.

#### Scenario: Fresh supported installation activates engines
- **WHEN** a user installs a correctly signed Zero File package on a compatible Zero host
- **THEN** the host verifies every engine asset and activates the plugin with no secondary runtime installation or engine download

#### Scenario: Package asset is corrupt
- **WHEN** an engine asset is missing or its digest does not match the signed manifest
- **THEN** installation or activation fails atomically with a repairable plugin-integrity error and the host does not execute the asset

#### Scenario: Generic plugin claims privileged engine access
- **WHEN** an unsigned or non-first-party plugin declares Zero File engine metadata or the `document.convert` permission
- **THEN** the host rejects the privileged capability and does not grant file staging or native PDF-export access

### Requirement: Built-in PDF-to-DOCX conversion works offline without an external runtime
On supported desktop hosts, the system SHALL provide a built-in PDF-to-DOCX provider from the installed Zero File engine bundle, SHALL parse and render locally, SHALL create a structurally valid DOCX in the existing Rust-owned output lifecycle, and MUST perform no network request during readiness checks or conversion.

#### Scenario: Fresh install converts a text PDF
- **WHEN** a user queues a valid non-encrypted text PDF after installing Zero File with no Python, LibreOffice, or Microsoft Office present
- **THEN** the built-in provider completes a valid DOCX locally and records the actual engine version and quality profile

#### Scenario: Fresh install converts a scanned PDF as a facsimile
- **WHEN** a valid image-only PDF has no extractable text and does not require a password
- **THEN** the built-in provider creates a page-preserving DOCX with `layoutPreserving` quality and states that the output text is not editable without claiming OCR

#### Scenario: Encrypted PDF requires a password
- **WHEN** PDF preflight determines that a password is required
- **THEN** the job fails with `passwordRequired`, creates no final DOCX, and does not upload or decrypt the input

### Requirement: PDF-to-DOCX quality profiles are truthful and recoverable
The system SHALL classify PDF-to-DOCX output as `editableReconstruction` or `layoutPreserving`, MUST use editable reconstruction only when the approved confidence policy accepts the document, and SHALL fall back to layout-preserving output when reconstruction is unsafe or fails without corrupting the source or exposing a false fidelity claim.

#### Scenario: Simple extractable document passes the confidence gate
- **WHEN** PDF pages have stable extractable text order and no disallowed complexity signal
- **THEN** the provider may produce editable paragraphs, runs, images, and page breaks and records `editableReconstruction` with a fidelity warning

#### Scenario: Complex layout fails the confidence gate
- **WHEN** a PDF contains ambiguous columns, unsupported rotations, overlapping text, or another approved complexity signal
- **THEN** the provider renders page-sized content into a valid DOCX, records `layoutPreserving`, and does not describe the result as editable reconstruction

#### Scenario: Editable generation fails partway
- **WHEN** editable reconstruction fails after validation but page rendering remains available
- **THEN** the provider removes the partial artifact, retries once using `layoutPreserving`, and reports failure only if the fallback also fails

### Requirement: macOS provides silent built-in DOCX-to-PDF conversion
On macOS 11 or newer, the system SHALL render DOCX locally with the installed Zero File engine assets, SHALL wait for document fonts and images before export, SHALL capture a bounded set of page rectangles with the native WebKit PDF API, SHALL merge them with the system PDFKit framework without showing a dialog, and SHALL validate and commit that PDF through the existing Rust-owned output lifecycle.

#### Scenario: Fresh macOS install converts a multi-page DOCX
- **WHEN** a user queues a valid multi-page DOCX on macOS 11+ with no LibreOffice or Microsoft Word installed
- **THEN** the built-in provider creates a valid multi-page PDF locally and reports the built-in macOS provider as selected

#### Scenario: Rendered resources are not ready
- **WHEN** fonts or embedded images do not become ready before the rendering deadline
- **THEN** the job fails or retries with a structured timeout/provider error and never commits a partial PDF as completed

#### Scenario: Platform API is unsupported
- **WHEN** Zero runs on a macOS version that lacks the required native WebKit PDF API
- **THEN** the built-in DOCX-to-PDF provider is unavailable with an explicit platform-version reason and the UI does not imply that installing Word or Python is required

### Requirement: Engine work remains isolated and bounded
The system SHALL run plugin engine code in a dedicated hidden WebView with network access denied, SHALL restrict it to a Zero-owned per-job staging directory and a one-time capability token, MUST keep the visible panel independent from engine lifetime, and SHALL bound startup, conversion, page pixels, temporary storage, diagnostics, and cancellation.

#### Scenario: File panel unmounts during conversion
- **WHEN** the user switches tools or closes the tray while a built-in conversion is running
- **THEN** the hidden engine WebView and Rust queue continue the job and the panel restores the authoritative state when reopened

#### Scenario: Engine attempts an out-of-scope file access
- **WHEN** engine code requests a user path, another plugin's data, or a path outside its active job staging directory
- **THEN** the host denies the operation and fails the job without changing the requested external file

#### Scenario: User cancels a built-in job
- **WHEN** the user cancels during PDF parsing, page rendering, DOCX generation, DOCX rendering, native page capture, or PDF merging
- **THEN** the engine acknowledges cancellation within the approved deadline, Rust removes partial staging output, and the queue advances without committing a result

#### Scenario: Engine WebView crashes
- **WHEN** the hidden engine WebView exits or stops responding during a job
- **THEN** the job receives a structured retryable provider failure, its capability token is revoked, and the host can recreate the engine for a later retry

### Requirement: Supported fresh installs expose ready conversion actions
The system SHALL derive direction readiness from the authoritative verified provider snapshot, SHALL select built-in providers before optional externally installed compatibility providers by default, and SHALL present both directions as ready on a fresh macOS 11+ Zero File installation so valid queued items can be started with `Convert all`.

#### Scenario: No office suite is installed on supported macOS
- **WHEN** verified Zero File assets are ready on macOS 11+ and LibreOffice and Microsoft Word discovery both return absent
- **THEN** the provider strip marks PDF-to-DOCX and DOCX-to-PDF ready with their built-in provider names and no “missing engine” guidance

#### Scenario: User queues valid files after readiness
- **WHEN** at least one valid PDF or DOCX job is queued and its built-in provider is ready
- **THEN** `Convert all` is enabled and starts the existing bounded queue without another setup action

#### Scenario: Optional compatibility provider is present
- **WHEN** LibreOffice or Microsoft Word is detected alongside a ready built-in provider
- **THEN** the built-in provider remains the default and the external provider is represented only as an optional compatibility choice

#### Scenario: Installed engine needs repair
- **WHEN** the authoritative snapshot reports corrupt assets or a failed engine startup
- **THEN** the UI shows a plugin repair/reinstall action and a structured diagnostic rather than instructions to install Python or an office suite

### Requirement: Cross-boundary engine contracts remain symmetric and explicit
All plugin manifest fields, provider readiness objects, engine requests, progress events, cancellation acknowledgements, output metadata, quality profiles, warnings, and errors crossing Rust and TypeScript SHALL have matching serialized definitions, and TypeScript MUST NOT use `any` to consume them.

#### Scenario: Built-in conversion completes
- **WHEN** the engine returns a completed output to Rust
- **THEN** Rust validates a typed response containing engine version, output type, quality profile, warnings, and measured page metadata before committing the result

#### Scenario: Engine sends a stale token or malformed response
- **WHEN** the host receives an expired job token, unknown enum, missing required field, unexpected path, or mismatched engine version
- **THEN** the host rejects the response, commits no output, and records a structured provider protocol error

### Requirement: Redistributed engines pass reproducible release gates
The system MUST pin all redistributed engine and transitive dependency versions, MUST include required license and notice files, MUST enforce the approved license allowlist and package-size budget, and MUST pass representative offline, security, fidelity-profile, startup, cancellation, memory, platform-signing, and packaged-application smoke gates before an engine version is marked approved.

#### Scenario: Candidate exceeds a release gate
- **WHEN** a candidate package exceeds the approved size budget, introduces an incompatible license, performs a network request, fails a required corpus case, or cannot be verified in the signed installed package
- **THEN** `file-engine-policy.json` keeps that version unapproved and shipping capability discovery cannot select it

#### Scenario: Candidate passes every release gate
- **WHEN** the pinned package and dependency graph pass every recorded gate for a claimed platform and direction
- **THEN** the policy may approve exactly that package digest, platform, direction, quality profile, and minimum host version

#### Scenario: Source-tree test passes but packaged smoke fails
- **WHEN** engine tests pass from the repository but the installed signed plugin cannot load, render, capture/merge PDF pages, cancel, or validate an output
- **THEN** the release fails and the engine is not advertised as ready
