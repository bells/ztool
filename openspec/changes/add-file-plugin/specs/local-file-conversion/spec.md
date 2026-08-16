## ADDED Requirements

### Requirement: Conversion availability is capability-detected per direction
The system SHALL probe approved local providers for PDF-to-DOCX and DOCX-to-PDF independently, SHALL expose the selected provider and any unavailability reason through a typed capability snapshot, and MUST NOT report a direction as available until its provider executable, version, and required platform integration pass validation.

#### Scenario: One direction is available
- **WHEN** LibreOffice is usable for DOCX-to-PDF but no approved PDF-to-DOCX provider is available
- **THEN** the capability snapshot marks DOCX-to-PDF available with LibreOffice and PDF-to-DOCX unavailable with an actionable engine reason

#### Scenario: Provider version is unsupported
- **WHEN** a provider executable exists but its detected version does not satisfy the approved range
- **THEN** the system marks that provider unavailable and returns `engineVersionUnsupported` instead of attempting conversion

### Requirement: Conversion inputs are validated before execution
The system MUST canonicalize every selected source, require a readable regular file, verify the supported extension and lightweight PDF or DOCX container signature, and reject missing files, directories, malformed packages, temporary Office lock files, and duplicate active sources before starting a provider.

#### Scenario: Valid PDF is accepted
- **WHEN** a readable `.pdf` file has a valid PDF header and is not already queued or running
- **THEN** the system classifies it as PDF-to-DOCX and returns a validated candidate without reading document content into the frontend

#### Scenario: Renamed or malformed DOCX is rejected
- **WHEN** a `.docx` path is not a ZIP package containing the required DOCX entries
- **THEN** the system rejects it with `invalidInput` and does not launch a provider or create an output

#### Scenario: Duplicate active source is rejected
- **WHEN** the same canonical source and direction already exist in a queued or running job
- **THEN** the system reports a duplicate candidate and preserves the existing job

### Requirement: Conversion remains local and provider identity is explicit
Zero MUST NOT upload document bytes, extracted content, source paths, or conversion outputs and MUST NOT initiate a network request while probing or executing a conversion. The system SHALL identify the local provider used for each job and SHALL distinguish Zero's offline behavior from any independent behavior of an externally installed office application.

#### Scenario: Local provider converts a document
- **WHEN** a validated job runs with an available local provider
- **THEN** Zero passes only local input, temporary, and output paths to that provider and performs no network request for the conversion

#### Scenario: No local provider is available
- **WHEN** no approved local provider supports the requested direction
- **THEN** the job remains unstarted with `engineUnavailable` and Zero does not fall back to a cloud service or download an engine

### Requirement: Provider redistribution is gated by evidence
The system MUST NOT bundle a conversion provider until its pinned artifact and dependency graph pass documented maintenance, license, security, package-size, startup, offline, cancellation, platform-signing, and representative fidelity checks for every claimed platform and direction.

#### Scenario: PDF-to-DOCX candidate fails a gate
- **WHEN** a candidate has unacceptable fidelity, an incompatible redistribution license, an unbounded process, or an unapproved package impact
- **THEN** the shipping build excludes that sidecar and keeps PDF-to-DOCX capability-gated or uses another approved provider

#### Scenario: Candidate is approved
- **WHEN** the release owner accepts the recorded benchmark and licensing decision for a pinned provider artifact
- **THEN** the build may register that artifact for only the tested platforms and directions

### Requirement: Jobs execute through a bounded Rust-owned lifecycle
The Rust service SHALL own a session-scoped queue, SHALL execute at most one conversion job at a time in the first release, and SHALL preserve each job as `queued`, `preparing`, `running`, `completed`, `failed`, or `cancelled` with its selected provider, stage, and structured result.

#### Scenario: Mixed batch runs sequentially
- **WHEN** the user starts a queue containing supported PDF and DOCX candidates
- **THEN** the service runs one job at a time in queue order and a failure does not prevent later queued jobs from running

#### Scenario: Panel closes during conversion
- **WHEN** the React panel unmounts while a job is running
- **THEN** Rust continues the job and returns the authoritative current queue snapshot when the panel mounts again

#### Scenario: User cancels an active job
- **WHEN** the user cancels a running conversion
- **THEN** the system terminates the provider process tree, removes partial temporary output, marks the job `cancelled`, and proceeds to the next queued job

### Requirement: External provider execution is constrained and recoverable
The system MUST launch provider executables directly with argument arrays rather than shell strings, use a per-job working directory, bound captured output, enforce a timeout, support cancellation, classify abnormal exits, and clean child processes and Zero-owned temporary files after success, failure, cancellation, or application shutdown.

#### Scenario: Provider times out
- **WHEN** a provider exceeds its configured execution deadline
- **THEN** the system terminates its process tree, records `timeout` as retryable where appropriate, removes partial output, and leaves the source unchanged

#### Scenario: Provider emits excessive diagnostics
- **WHEN** a provider writes more stdout or stderr than the configured capture budget
- **THEN** the system truncates captured diagnostics without exhausting memory and retains enough metadata to classify the failure

### Requirement: Outputs are collision-free and committed safely
The system SHALL default the target beside the source or inside a user-selected writable directory, MUST NOT overwrite an existing file, SHALL convert into a Zero-owned temporary path, SHALL validate the expected output type, and SHALL commit the completed result to a reserved collision-free final name only after successful validation.

#### Scenario: Default output name already exists
- **WHEN** `<stem>-converted.<target-extension>` already exists
- **THEN** the system reserves an incremented collision-free name and preserves every existing file

#### Scenario: Provider produces invalid output
- **WHEN** a provider exits successfully but the output is missing, empty, or not a valid target container
- **THEN** the job fails with `invalidProviderOutput`, the partial output is removed, and no completed result is exposed

#### Scenario: Final directory is not writable
- **WHEN** the selected output directory cannot be written before conversion starts
- **THEN** the system reports `outputNotWritable` without launching the provider

### Requirement: Known unsupported document classes are reported honestly
The system SHALL classify password-protected PDFs, image-only PDFs that require OCR, unsupported legacy Word formats, and provider-specific unsupported features when detectable, and MUST NOT present an empty or corrupted target as a successful conversion.

#### Scenario: Encrypted PDF needs a password
- **WHEN** the provider or preflight detects that a PDF requires a password
- **THEN** the job fails with `passwordRequired` and explains that password entry is not supported in this release

#### Scenario: Scanned PDF needs OCR
- **WHEN** a PDF contains no convertible editable text and the selected provider cannot perform OCR
- **THEN** the job fails with `ocrRequired` instead of reporting a successful editable DOCX

### Requirement: Rust and TypeScript conversion contracts are symmetric
All cross-boundary capability, candidate, request, job, progress, provider, result, and error objects SHALL have explicit serializable Rust definitions and matching TypeScript interfaces or discriminated unions, and TypeScript MUST NOT use `any` to consume conversion payloads.

#### Scenario: Job update event is received
- **WHEN** Rust emits a `zero://file-conversion/job-updated` payload
- **THEN** the frontend narrows the typed job state and renders only fields valid for that state

#### Scenario: Malformed command input arrives
- **WHEN** a conversion command receives a missing field, invalid enum, arbitrary result path, or unknown job ID
- **THEN** the command rejects the request without starting a process or opening a path

### Requirement: Completed output actions are job-scoped
The system SHALL open or reveal only the validated output recorded for a completed job ID, MUST revalidate that output immediately before the action, and MUST NOT expose a general arbitrary-path open or reveal command through the File plugin API.

#### Scenario: Reveal completed output
- **WHEN** the user requests reveal for a completed job whose output still exists and matches the recorded target type
- **THEN** the system highlights that file in Finder or Explorer through a direct platform operation

#### Scenario: Output was moved after completion
- **WHEN** the recorded output no longer exists or no longer resolves to the validated result
- **THEN** the system returns `outputMissing` and does not open a parent directory or another path
