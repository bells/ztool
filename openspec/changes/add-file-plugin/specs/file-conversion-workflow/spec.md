## ADDED Requirements

### Requirement: Zero File is an isolated bundled plugin
The system SHALL register a trusted bundled plugin named `Zero File` with canonical ID `zero.file` through the existing frontend and Rust composition roots, SHALL keep its descriptor, presentation, localization, UI, contracts, and domain code under its own plugin ownership, and MUST NOT introduce a concrete dependency between File, host core, or a sibling plugin outside the composition roots.

#### Scenario: File plugin is included in a build
- **WHEN** the bundled registries are constructed
- **THEN** the `zero.file` frontend descriptor and Rust record have matching identity, contributions, supported platforms, and command metadata and pass uniqueness validation

#### Scenario: File plugin is removed from a build
- **WHEN** its frontend and Rust composition entries are removed
- **THEN** the remaining host and plugins build without a File-specific switch branch or sibling reference

### Requirement: Users can add and review supported files before conversion
The File panel SHALL accept multiple files through desktop drag-and-drop and a native multi-file picker, SHALL detect `.pdf` as PDF-to-DOCX and `.docx` as DOCX-to-PDF, and SHALL show the source name, size, detected direction, target name, and validation result before any provider process starts.

#### Scenario: User drops a mixed supported batch
- **WHEN** the user drops valid PDF and DOCX files on the panel
- **THEN** the panel adds one reviewed queue item per file with its automatically detected opposite target format and does not start conversion until the user chooses `Convert all`

#### Scenario: User selects an unsupported file
- **WHEN** a dropped or picked item has an unsupported extension or fails native validation
- **THEN** the panel keeps valid candidates, reports each rejected item inline with a specific reason, and does not add an executable job for the rejected item

#### Scenario: User activates the drop target by keyboard
- **WHEN** keyboard focus is on the empty drop target and the user activates it
- **THEN** the native multi-file picker opens with PDF and DOCX filters

### Requirement: The queue supports controlled batch actions
The File panel SHALL let the user start all queued jobs, cancel an active or queued job, remove a non-running job, retry an eligible failed or cancelled job, and clear completed results while preserving running work and unrelated jobs.

#### Scenario: User starts the queue
- **WHEN** at least one validated queued job has an available provider and the user chooses `Convert all`
- **THEN** the panel disables duplicate starts, displays the active job, and leaves remaining jobs visibly queued in service order

#### Scenario: User removes a queued item
- **WHEN** the user removes a job that has not started
- **THEN** only that job leaves the queue and no source or output file is changed

#### Scenario: User retries a failed job
- **WHEN** a failed job is retryable and its source and provider remain valid
- **THEN** the service creates or resets a queued attempt without overwriting an earlier successful output

### Requirement: Progress and state feedback are truthful
The panel SHALL render real provider percentage only when supplied, SHALL otherwise render a stage label and indeterminate activity, and SHALL provide distinct queued, preparing, running, completed, failed, cancelled, provider-unavailable, and validation-rejected presentations without fabricating progress or success.

#### Scenario: Provider reports percentage
- **WHEN** the active provider reports 42 percent during conversion
- **THEN** the job row exposes 42 percent with an accessible progress value and the `converting` stage

#### Scenario: Provider has no progress protocol
- **WHEN** the active provider reports only process lifecycle
- **THEN** the row shows an indeterminate conversion state and no numeric percentage

#### Scenario: One batch item fails
- **WHEN** one running item fails while later items remain queued
- **THEN** its row shows the structured failure and recovery action while later jobs continue according to queue order

### Requirement: The workflow adapts to tray and main-window space
The same File plugin workflow SHALL remain usable in the compact tray panel and the existing main window. The tray presentation SHALL use a compact drop target and scrollable concise queue, while the main-window presentation SHALL expose provider, output location, and full row actions without requiring a separate File window.

#### Scenario: Queue is viewed in the tray
- **WHEN** multiple jobs exceed the compact panel's available height
- **THEN** the queue scrolls within one bounded region while the primary add/start controls remain reachable

#### Scenario: Queue is viewed at a narrow width
- **WHEN** the panel width is below its desktop multi-column breakpoint
- **THEN** each row stacks source/target, state, and actions in a single-column order with no clipped filename or unreachable control

### Requirement: Empty, loading, unavailable, error, and result states direct the next action
The panel SHALL provide an instructional empty state, queue-shaped loading state, inline invalid-file feedback, direction-specific engine guidance, retryable error actions, cancelled state, and completed open/reveal actions using consistent user-facing vocabulary.

#### Scenario: No conversion engine is installed
- **WHEN** neither direction has an available provider
- **THEN** the panel explains which local providers Zero can detect, states that no cloud fallback will be used, and keeps file intake available for validation without enabling conversion

#### Scenario: Conversion completes
- **WHEN** a job reaches `completed`
- **THEN** its row shows the final filename and offers `Open file` and `Show in folder` actions bound to that job

#### Scenario: Output action fails
- **WHEN** a completed output was moved or deleted before an open/reveal action
- **THEN** the panel displays the returned `outputMissing` error in that row and does not mark the conversion itself failed

### Requirement: Queue state survives panel lifecycle but not application restart
The frontend SHALL restore the current Rust queue snapshot whenever the File panel mounts and SHALL reconcile subsequent typed update events, while the first release MUST NOT persist sensitive source/output paths as conversion history across application restart.

#### Scenario: Tray closes and reopens
- **WHEN** the tray panel closes during a running job and reopens later
- **THEN** the panel restores the active and queued jobs with their current state from Rust

#### Scenario: Zero restarts
- **WHEN** the application starts after a prior session with completed or failed jobs
- **THEN** the File queue starts empty, stale Zero-owned temporary directories are cleaned, and user source/output files remain untouched

### Requirement: File workflow is bilingual and accessible
All Zero File labels, directions, states, provider guidance, actions, and error recovery text SHALL be available in Simplified Chinese and English. The workflow SHALL support keyboard operation, visible focus, semantic status/error announcements, accessible progress, non-color state cues, reduced motion, and action targets suitable for compact desktop and touch use.

#### Scenario: Language changes while queue exists
- **WHEN** the user changes the Zero language with jobs queued or running
- **THEN** the panel relocalizes visible labels and messages without recreating, cancelling, or reordering jobs

#### Scenario: Reduced motion is enabled
- **WHEN** the operating system requests reduced motion
- **THEN** queue insertion and state transitions update without nonessential animation while progress and status remain understandable

#### Scenario: Error is announced
- **WHEN** a job enters a failed state
- **THEN** assistive technology receives the specific error and available recovery action without relying on color or an icon alone

### Requirement: Interface claims match actual local behavior
The File panel SHALL state that Zero does not upload files and that conversion uses the named local provider, and MUST NOT claim perfect fidelity, universal document support, bundled-engine availability, or independence from external provider licensing unless those claims are true for the active build and provider.

#### Scenario: User-installed provider is selected
- **WHEN** a conversion uses an installed LibreOffice, Microsoft Word, or `pdf2docx` provider
- **THEN** the UI names that provider and can show its relevant permission or fidelity limitation before or during the job

#### Scenario: Scanned PDF is rejected
- **WHEN** a PDF-to-DOCX job returns `ocrRequired`
- **THEN** the UI explains that the current provider cannot turn scanned pages into editable text and does not describe the operation as successfully converted
