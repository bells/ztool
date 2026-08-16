## Context

Zero is a tray-first Tauri 2 toolbox with trusted bundled plugins registered at build time and sandboxed third-party `.zplugin` packages registered at runtime. The current frontend composition root is `src/appShell/bundledPluginModules.ts`; each concrete plugin owns its descriptor, localization, panel, model, and service bridge. Native capabilities are implemented by thin handlers under `src-tauri/src/commands/`, domain services under `src-tauri/src/services/`, and explicit state/handler registration in `src-tauri/src/bundled_plugins.rs` and `src-tauri/src/lib.rs`.

Document conversion crosses several trust and quality boundaries. React may select files and display state, but Rust must validate paths and formats, own the queue, start and stop local processes, protect temporary/output files, and report provider-specific failures. PDF and DOCX are containers rather than symmetric document models, so no open-source engine can honestly promise perfect bidirectional layout fidelity for every document. Scanned or encrypted PDFs add OCR/password requirements that are outside a basic converter.

The upstream review on 2026-08-16 found:

- LibreOffice exposes a maintained cross-platform `soffice --headless --convert-to ... --outdir ...` interface and is the most practical default DOCX-to-PDF provider when installed.
- `docx2pdf` is an MIT wrapper around installed Microsoft Word automation, but its latest PyPI release is 0.1.8 from 2021; Zero should implement the small platform adapter itself instead of shipping that Python wrapper.
- Artifex `pdf2docx` 0.5.13 is MIT, but its README says it is no longer actively maintained. It also requires Python, PyMuPDF, OpenCV, NumPy, and `python-docx`; current PyMuPDF is AGPL-3.0 or commercially licensed. It is a candidate provider, not an automatic bundled dependency.
- ONLYOFFICE Desktop Editors is actively maintained and handles common office/PDF formats, but it is AGPL-3.0 and its exact unattended desktop conversion contract and redistribution cost need a separate benchmark before adoption.

This design therefore separates stable product contracts from replaceable local providers and makes capability availability visible to the user.

## Goals / Non-Goals

**Goals:**

- Add Zero File as a self-contained trusted bundled plugin with canonical ID `zero.file` and no sibling-plugin dependency.
- Support local PDF-to-DOCX and DOCX-to-PDF jobs whenever an approved provider for that direction is available.
- Keep job state, process control, input validation, temporary files, output naming, and result actions in Rust.
- Expose symmetric, serializable Rust and TypeScript contracts with discriminated states and structured errors.
- Provide a compact drag/pick/review/start workflow in the tray and a roomier batch view in the existing main window.
- Continue running a queue when the panel unmounts or the tray closes, and restore the authoritative snapshot when it remounts.
- Make conversion quality, missing dependencies, permissions, unsupported inputs, and indeterminate progress explicit.
- Establish a repeatable quality, security, size, and licensing gate before any conversion engine is redistributed with Zero.

**Non-Goals:**

- Perfect pixel-equivalent conversion for all PDFs, Word features, fonts, macros, embedded objects, tracked changes, or form controls.
- OCR, password entry/decryption, legacy `.doc`, image-to-PDF, PDF editing, document merging, compression, or cloud conversion in this change.
- Automatically downloading engines or silently installing LibreOffice, Microsoft Word, Python, or ONLYOFFICE.
- Exposing arbitrary filesystem paths, process execution, or File plugin commands to sandboxed third-party plugins.
- Adding a new dedicated Tauri window or a default menu-bar item for Zero File; the existing tray panel and main window are sufficient for the first release.
- Mobile support. iOS and Android do not share the desktop process/provider model and remain unavailable.

## Decisions

### 1. Add a bundled plugin that follows the current composition contract

The frontend module will use the existing ownership pattern:

```text
src/plugins/file/
  FilePanel.tsx
  contracts.ts
  fileModel.ts
  fileService.ts
  i18n.ts
  plugin.tsx
  useFileConversion.ts
```

`plugin.tsx` owns the `zero.file` manifest, localized presentation, `accent-file`, and panel renderer. `src/appShell/bundledPluginModules.ts` receives one import/registry entry. `src/brand/identity.ts` and Rust `brand.rs` receive the same canonical ID. The Rust registry receives a bundled record, `bundled_plugins.rs` manages `FileConversionState`, and `lib.rs` registers thin commands. No core module or sibling plugin imports File.

The initial File manifest uses only existing bundled-plugin vocabulary and does not broaden the third-party Extension API with generic filesystem access. File's file/process capabilities remain trusted Rust implementation details. A future permissioned filesystem API for runtime extensions requires its own spec and security review.

Zero File contributes a main plugin view but no default status-bar item. This avoids adding permanent menu-bar width for a task that is usually opened intentionally.

Alternatives considered:

- Ship File as a `.zplugin`. Runtime plugins cannot load arbitrary Rust or run unrestricted conversion sidecars, so this would either fail or weaken the extension boundary.
- Add `file_converter_cmd.rs` and a central manifest JSON. Current Zero conventions use unsuffixed domain command modules and plugin-owned descriptors; following them keeps removal and testing predictable.
- Add a dedicated File window immediately. The existing main window is already 920 by 660 and can host the batch view without another lifecycle surface.

### 2. Keep one Rust-owned queue and expose snapshot plus event contracts

`src-tauri/src/services/file/` owns an in-memory `FileConversionState` containing jobs, queue order, active child-process metadata, provider capabilities, and cancellation handles. It permits one active job at a time in the first release. Sequential execution is predictable for Office automation, avoids concurrent profile locks, and limits CPU/memory spikes from PDF parsing.

The frontend never treats local React state as authoritative. It calls:

```text
get_file_conversion_capabilities
choose_file_conversion_inputs
enqueue_file_conversions
list_file_conversion_jobs
start_file_conversion_queue
cancel_file_conversion_job
remove_file_conversion_job
retry_file_conversion_job
open_file_conversion_output
reveal_file_conversion_output
```

The Rust service emits `zero://file-conversion/job-updated` with the same serialized job snapshot returned by `list_file_conversion_jobs`. `useFileConversion` subscribes and also refreshes after mounting so a missed event cannot lose state.

The IPC model uses matching Rust enums/structs and TypeScript unions:

- direction: `pdfToDocx | docxToPdf`
- state: `queued | preparing | running | completed | failed | cancelled`
- progress: `indeterminate` or a provider-reported integer percentage from 0 through 100
- stage: `validating | waitingForProvider | converting | finalizing`
- provider: stable ID, display name, direction support, availability, and unavailability reason
- error: stable code, user-safe message, retryability, and optional provider diagnostics that exclude document content

Commands accept explicit input objects and return `Result<T, String>` at the Tauri boundary. Domain failures are represented in job snapshots so one failed file does not reject or erase the rest of a batch.

Alternatives considered:

- Run conversion directly from React and await one invoke. That loses cancellation/background continuity and lets panel lifecycle own native work.
- Start one command per file in parallel. LibreOffice profiles and Microsoft Office automation are not safely parallel by default, and PDF conversion can consume substantial memory.
- Persist full history. The first release keeps jobs session-scoped to avoid retaining sensitive source paths after app restart.

### 3. Validate the real file and reserve outputs before launching a provider

Rust canonicalizes each selected path, requires a readable regular file, and rejects directories, missing files, temporary Office lock files, duplicate active sources, and unsupported extensions. Extension checks are supplemented by lightweight signatures:

- PDF starts with a valid PDF header.
- DOCX is a ZIP package containing `[Content_Types].xml` and `word/document.xml`.

The default output is beside the source with the opposite extension. Existing files are never overwritten. The resolver first uses `<stem>-converted.<ext>` and then an incrementing suffix. An optional output directory selected by the native folder picker is canonicalized and tested for writability before the job starts.

Each job converts in an owner-only temporary directory under Zero's cache root. The service validates the provider result, writes or moves it to a reserved temporary destination in the final directory, then atomically renames it into place where the filesystem permits. Partial files are removed on failure or cancellation, and stale job directories are cleaned on startup.

The open/reveal commands accept a completed job ID, not an arbitrary frontend path. Rust resolves the recorded output again and invokes a platform API or direct executable arguments without constructing a shell string.

Alternatives considered:

- Trust the extension alone. Renamed archives and malformed inputs would produce confusing engine errors or unsafe output behavior.
- Let providers write directly over a user-named file. A crash or cancellation could destroy an existing document.
- Expose `showInFolder(path)` as drafted. A job-scoped result operation is narrower and prevents the command from becoming a general path launcher.

### 4. Use direction-specific providers behind one adapter interface

The internal provider interface performs capability probing and conversion without leaking provider-specific process output into the UI:

```text
FileConversionProvider
  id()
  supported_directions()
  probe()
  convert(request, progress_sink, cancellation_token)
```

Provider selection is deterministic and recorded on each job.

For DOCX-to-PDF:

1. Use a detected LibreOffice `soffice` binary by default because its headless CLI is cross-platform and does not require Office automation permission.
2. If LibreOffice is unavailable, use installed Microsoft Word through a small native macOS AppleScript adapter or Windows COM adapter.
3. Report `engineUnavailable` when neither provider is usable. A permission denial or Office activation failure is not reported as a missing engine.

For PDF-to-DOCX:

1. Use an approved bundled `pdf2docx` sidecar only when the engine gate in Decision 5 has passed for the shipping build.
2. Otherwise detect a compatible user-installed `pdf2docx` executable and report its version and limitations.
3. Keep ONLYOFFICE or a future native provider behind the same interface after its CLI and licensing are separately approved.
4. Report `engineUnavailable`, `passwordRequired`, `ocrRequired`, or `unsupportedInput` instead of producing an apparently successful empty document.

Child processes are launched directly with argument arrays, a dedicated working directory, bounded stdout/stderr capture, a timeout, and cancellation that terminates the process tree. Zero itself makes no network request during probing or conversion. The UI names the selected external provider because an installed office application can have its own licensing or network behavior outside Zero's control.

Alternatives considered:

- Build PDF parsing and DOCX layout reconstruction from Rust crates such as PDF parsers plus `docx-rs`. Those crates provide primitives, not a mature high-fidelity converter; this would become a large document-layout engine project.
- Use `docx2pdf`. It adds Python and an old wrapper without removing the Microsoft Word dependency.
- Treat LibreOffice as a PDF-to-DOCX provider. Its headless conversion is dependable for office documents to PDF but does not provide a validated high-fidelity PDF-to-DOCX path.
- Bundle a full LibreOffice or ONLYOFFICE distribution immediately. Installer size, updates, licensing, notarization, and attack surface are disproportionate before product usage is proven.

### 5. Require an engine acceptance gate before redistribution

Implementation starts with provider probes, fake-provider contract tests, and a non-sensitive benchmark corpus containing Latin/CJK paragraphs, fonts, headings, tables, images, headers/footers, page breaks, columns, scanned pages, encrypted PDFs, malformed packages, and large documents. For every candidate, the project records:

- supported direction and platform versions;
- upstream maintenance status and pinned version/hash;
- license of the provider and all redistributed dependencies;
- compressed/uncompressed app-size impact and startup/probe time;
- editable text coverage, reading order, table/image preservation, page/layout comparison, and failure classification;
- offline behavior, temporary files, process cleanup, timeout, and cancellation behavior;
- macOS signing/notarization and Windows packaging/antivirus behavior.

A candidate can be bundled only after the results and license decision are committed to a durable engine decision document and the release owner approves redistribution. If no PDF-to-DOCX candidate passes, the adapter and UI ship capability-gated with user-installed-provider detection; the app does not disguise the missing bundled engine.

Alternatives considered:

- Choose by GitHub stars. Popularity does not establish conversion fidelity, licensing compatibility, or package safety.
- Make the engine a task-time informal choice. That would let CI pass while producing materially different app behavior across developer machines.

### 6. Design the panel as a file transfer surface, not a generic card dashboard

Design read: a compact desktop utility for privacy-conscious users, using Zero's Modern Terminal / Minimalist Cyberpunk language with restrained motion and explicit operational state.

Design dials are `DESIGN_VARIANCE: 4`, `MOTION_INTENSITY: 3`, and `VISUAL_DENSITY: 5`. The panel inherits the existing Avenir Next/system type stack and shell theme rather than adding a font or design-system dependency. It uses the existing warm-neutral surfaces and one File accent, a cool document blue near `#3978C5`; semantic success/error colors remain global system feedback. Interactive controls follow the existing 7-8px radius family and visible focus treatment.

The signature element is a source-to-output transfer rail. In the empty state it explains `.pdf` and `.docx`; after selection it becomes a real queue line connecting the detected source format, truthful stage/progress, and target format. It is functional, not decorative.

```text
Tray / compact                         Main window / roomy
+----------------------------+        +--------------------------------------+
| Zero File       2 queued   |        | Zero File           Convert all     |
| +------------------------+ |        | +----------------------------------+ |
| | Drop PDF or DOCX       | |        | | Drop files or choose files       | |
| +------------------------+ |        | +----------------------------------+ |
| report.pdf -> DOCX   ...   |        | report.pdf  -> DOCX  Converting 42% |
| notes.docx -> PDF   queued |        | notes.docx   -> PDF   Queued         |
| [Convert all]              |        | invoice.pdf -> DOCX  Engine missing |
+----------------------------+        | [queue actions and result actions]  |
                                      +--------------------------------------+
```

Dropping files detects and queues them but does not immediately start external processes. The explicit `Convert all` action prevents accidental conversion and lets users review targets. The compact panel uses one scroll region and concise rows; the existing main window shows filename, size, provider, output location, and per-row controls. At narrow widths the row stacks into source/target, state, then actions, with at least 44px touch targets.

Motion is limited to state feedback: the drop target border changes on valid drag, rows settle when added, and real progress updates without continuous decorative animation. Reduced-motion users receive instant state changes. If a provider cannot report percentage, the UI uses a stage label and an indeterminate treatment, never a fabricated percent.

The panel includes loading skeletons shaped like queue rows, a directional empty state, inline unsupported-file feedback, engine-unavailable guidance, retryable failures, cancelled state, completed actions, keyboard file selection, accessible live status, and bilingual Chinese/English copy. Icons reuse the project's approved shared icon language; no hand-authored one-off SVG paths or emoji are introduced.

Alternatives considered:

- Use two large equal conversion cards. That duplicates intake and forces users to choose a direction the file format already determines.
- Auto-start on drop. It is surprising for batches and can trigger Office automation permission prompts before the user reviews output locations.
- Add glass, animated format icons, or marketing-style illustrations. The subject is operational and privacy-sensitive; clarity is the appropriate visual identity.

### 7. Keep quality and privacy claims precise

Visible copy says that files are processed locally by the named provider and are not uploaded by Zero. It does not say every provider is bundled, every layout is preserved, or all documents are supported. Completed jobs may display a fidelity note for known provider limitations. Scanned PDFs direct the user to an OCR-capable workflow; encrypted PDFs explain that password-protected conversion is not supported in this release.

Diagnostics use stable error codes such as `invalidInput`, `unsupportedFormat`, `engineUnavailable`, `engineVersionUnsupported`, `automationPermissionDenied`, `passwordRequired`, `ocrRequired`, `timeout`, `cancelled`, `outputConflict`, `outputNotWritable`, `providerFailed`, and `invalidProviderOutput`. Logs may include provider/version, job ID, timings, exit code, and bounded stderr, but must not include document bytes, extracted text, passwords, or unredacted full paths.

Alternatives considered:

- Advertise "100% private" or "perfect formatting". Zero controls its own upload behavior but cannot make universal claims about third-party office applications or lossy format conversion.

## Risks / Trade-offs

- [PDF-to-DOCX open-source quality is inconsistent] -> Use a representative benchmark corpus, capability-gate the direction, classify scanned/encrypted inputs, and avoid bundling a provider that fails the gate.
- [A permissively licensed wrapper depends on copyleft or commercially licensed libraries] -> Review the complete dependency graph and redistribution mode, pin approved artifacts, and keep user-installed-provider detection as the safe fallback.
- [Office automation prompts, activates UI, or fails on headless machines] -> Prefer LibreOffice for default DOCX-to-PDF, probe automation separately, return permission/activation errors, and never claim silent execution when the provider cannot guarantee it.
- [Large or malformed documents consume excessive CPU, memory, disk, or time] -> Execute one job at a time, validate containers, isolate temporary directories, apply timeouts/output limits, bound logs, support cancellation, and clean process trees.
- [The tray panel becomes too dense for batch work] -> Keep compact rows and a single scroll region, while using the existing main window for complete provider/output/action detail.
- [Closing the panel appears to cancel work or loses progress] -> Keep the queue in Rust, emit snapshots, refresh on mount, and expose a persistent busy count in the File plugin navigation metadata where supported.
- [Result actions become a general arbitrary-path launcher] -> Accept completed job IDs only and revalidate the recorded output before open/reveal.
- [Source or existing output is overwritten] -> Reserve collision-free names, write to temporary outputs, validate results, and commit atomically without overwrite.
- [Tests pass with fake providers but platform integration fails] -> Retain manual macOS and Windows smoke tasks for real LibreOffice/Word/provider processes, permission prompts, signing, Finder, and Explorer.

## Migration Plan

1. Add the canonical `zero.file` identity, frontend descriptor, Rust bundled registry record, and isolated module boundaries with no conversion engine enabled.
2. Add symmetric contracts, input/output validation, naming, queue state machine, event snapshots, cancellation, temporary cleanup, and fake-provider tests.
3. Implement provider discovery and the LibreOffice DOCX-to-PDF adapter, then add optional Microsoft Word adapters for macOS and Windows.
4. Run and document the PDF-to-DOCX provider gate. Integrate an approved sidecar or retain user-installed `pdf2docx` detection when redistribution is not approved.
5. Build the adaptive File panel and bilingual workflow against the stable service contract, including all empty/busy/result/error states.
6. Run source-boundary tests, frontend unit/integration tests, production build, Rust formatting/check/tests, strict OpenSpec validation, license/package audits, and manual provider smoke tests on macOS and Windows.

Rollback removes the File descriptors/registrations and command handlers, then deletes only Zero-owned temporary job directories. Source documents and completed outputs remain user-owned and are never removed by rollback.

## Open Questions

None. The shipping provider set is deliberately resolved by the documented engine gate during implementation; lack of an approved provider results in an explicit unavailable capability rather than an implicit architecture decision.
