## Why

Zero lacks a focused local document-conversion tool, so users must leave the app, upload sensitive files to online services, or pay for a full office suite for a small recurring task. Adding Zero File establishes a reusable file-tool plugin boundary while delivering an honest, offline PDF and Word conversion workflow whose availability and fidelity reflect the conversion engines actually present on the device.

## What Changes

- Add a trusted bundled plugin named **Zero File**, with canonical plugin ID `zero.file`, that participates in the existing frontend and Rust composition roots without coupling to another concrete plugin.
- Add local `.pdf` to `.docx` and `.docx` to `.pdf` conversion jobs behind typed Rust/TypeScript contracts and capability-detected engine adapters.
- Add drag-and-drop and file-picker intake, deterministic format detection, collision-free output naming, a bounded sequential queue, cancellation, truthful progress states, and structured failure recovery.
- Add result actions to open a completed output or reveal it in Finder/Explorer without granting arbitrary paths to sandboxed third-party plugins.
- Add a compact bilingual panel that adapts between the tray surface and the main window and covers empty, queued, running, completed, unavailable, cancelled, and failed states.
- Keep conversion offline after an engine is installed. Report missing engines, unsupported/scanned/encrypted inputs, automation permission failures, and fidelity limitations rather than claiming universal conversion support.
- Gate any bundled PDF-to-DOCX sidecar on representative quality benchmarks, binary-size and startup budgets, security review, and dependency-license compatibility; do not silently download or execute an unapproved engine.

## Capabilities

### New Capabilities

- `local-file-conversion`: Detect available local conversion providers and safely execute PDF-to-DOCX and DOCX-to-PDF jobs with explicit output, cancellation, validation, and failure contracts.
- `file-conversion-workflow`: Let users add supported files, review and run a bounded batch queue, understand real job state, and open or reveal successful outputs from the Zero File panel.

### Modified Capabilities

None. Zero File conforms to the existing bundled-plugin composition and host boundaries without changing the requirements of the current main specs.

## Impact

- Frontend: new self-contained `src/plugins/file/` module, one registration in `src/appShell/bundledPluginModules.ts`, a `zero.file` brand identity, bilingual strings, adaptive panel styles, and focused unit/integration coverage.
- Rust: new thin file-conversion commands, a service-owned job registry and provider adapters, one bundled-plugin state registration, command registration, and narrowly scoped open/reveal operations.
- Contracts: symmetric serialized capability, job, progress, error, and result types across Rust and TypeScript; existing plugin manifest permission vocabulary may need narrow bundled file/process capability entries rather than broad filesystem access.
- Dependencies and packaging: LibreOffice and installed Microsoft Word are candidate DOCX-to-PDF providers; PDF-to-DOCX needs a benchmarked sidecar/provider decision. `pdf2docx`, PyMuPDF, OpenCV, LibreOffice, and ONLYOFFICE licenses and bundle sizes must be reviewed before redistribution.
- Platforms: the first supported product targets are macOS and Windows desktop. Linux and mobile remain capability-gated until an approved local provider and runtime model exist.
