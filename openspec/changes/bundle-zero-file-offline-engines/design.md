## Context

Zero File already has the difficult product plumbing: typed Rust/TypeScript contracts, native input validation, a Rust-owned sequential queue, cancellation and timeout handling, temporary-output validation, collision-safe commits, result actions, provider discovery, and a compact React workflow. Its shipping policy deliberately has `approvedBundledEngines: []`, so `capabilities.rs` hard-codes PDF-to-DOCX unavailable and DOCX-to-PDF depends on LibreOffice or Microsoft Word discovery. `fileConversionQueueActions()` consequently disables `Convert all` when a queued direction has no provider.

The current plugin model has two trust classes. Bundled plugins can call registered Tauri commands and own native Rust services; installed `.zplugin` packages run in isolated WebViews and cannot load plugin-provided Rust code. A solution that requires Python, a user-installed office suite, or dynamically loaded native plugin code would therefore fail either the product goal or the current security boundary.

The upstream review was refreshed on 2026-08-22:

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Mozilla PDF.js + `docx` | PDF.js: Apache-2.0, 53k+ stars, active; `docx`: MIT, 5.8k+ stars, active, browser-capable | Selected for PDF-to-DOCX. Mature parsing/rendering and document-writing primitives; Zero owns the bounded reconstruction policy. |
| `docx-preview` | Apache-2.0, 2k+ stars, active, stable `renderAsync`, browser print support | Selected for DOCX rendering before native macOS PDF capture. Its pagination limitations are covered by export CSS, measured page rectangles, corpus gates, and truthful warnings. |
| `pdf2docx` | MIT and 3.4k+ stars, but its README says Artifex no longer maintains it; requires Python, OpenCV, NumPy and AGPL-3.0 PyMuPDF | Rejected for redistribution. A PyInstaller sidecar would hide, not remove, the runtime, maintenance, license, and size risks. |
| PDFium + `pdfium-render` + `docx-rs` | Pdfium wrapper is MIT/Apache-2.0 and active; `docx-rs` is MIT and active | Retained as a fallback research path, not selected. It adds per-architecture native binaries/signing and still supplies no mature PDF-to-DOCX layout converter. |
| `docx-rs` + `pdf-writer` | Permissive Rust primitives | Rejected as the primary DOCX-to-PDF path. Parsing/writing are not an Office layout engine; Zero would have to invent pagination, shaping, tables, headers, fields, and font fallback. |
| LibreOffice | Very mature headless converter | Kept as an optional compatibility provider only. Redistributing the suite is far beyond the plugin size target and the user explicitly wants no external install. |
| ONLYOFFICE Desktop Editors | Mature office renderer, AGPL-3.0 | Rejected for the built-in plugin due to strong copyleft, package size, and unattended desktop integration cost. |

## Goals / Non-Goals

**Goals:**

- Make installing the first-party Zero File plugin sufficient to provide offline PDF-to-DOCX on supported desktop hosts and offline DOCX-to-PDF on macOS 11+.
- Keep engine code and notices owned by the plugin package while keeping user-file validation, job lifecycle, final output writes, and privileged native PDF capture/merging owned by Rust.
- Use only permissively licensed redistributed components and a system framework already present on macOS.
- Keep large document bytes out of cross-WebView event payloads and ordinary JSON IPC.
- Preserve the existing provider abstraction so optional LibreOffice/Word compatibility providers and a later Windows WebView2 exporter remain replaceable adapters.
- Report whether a PDF-to-DOCX result is editable reconstruction or layout-preserving facsimile, and never equate “conversion completed” with perfect editability.
- Fail plugin installation or engine activation atomically when assets, signatures, compatibility, or required platform APIs are invalid.

**Non-Goals:**

- Pixel-perfect editable reconstruction of every PDF, OCR, password decryption, macros, tracked-change fidelity, embedded OLE objects, or exact proprietary Word pagination.
- Shipping Python, Node.js, Chromium, LibreOffice, ONLYOFFICE, Microsoft Office, PyMuPDF, OpenCV, or an auto-downloaded native sidecar.
- Allowing arbitrary third-party plugins to read user documents, write arbitrary paths, invoke the File Tauri commands, or access the privileged PDF-export bridge.
- Windows DOCX-to-PDF in this change. It requires a separately benchmarked WebView2 `PrintToPdf` adapter and packaged Windows smoke coverage.
- Replacing the existing queue/output architecture or adding cloud conversion.

## Decisions

### 1. Ship a signed first-party engine bundle rather than a native sidecar

Zero File's installable package will contain a versioned `engine/` web bundle, worker assets, an engine manifest, license texts, and per-file SHA-256 digests. The package manifest will declare its minimum Zero host API, supported directions, platforms, architectures where relevant, and a `document.convert` permission. A detached Ed25519 signature from a Zero-pinned first-party key is required before the host grants the engine trust tier; a checksum from `market.json` remains a transport-integrity check, not the authority for privileged access.

The engine bundle contains no executable native code. The updated host provides a generic, narrow conversion worker surface and macOS PDF-export adapter. The package's `engines.zero` constraint prevents installation on hosts that lack that surface. Existing generic `.zplugin` packages cannot opt into the trust tier merely by adding fields to their manifests.

This keeps “install Zero File and use it” true without weakening the rule that runtime plugins cannot dynamically load Rust. It also makes engine upgrades atomic and independently versioned: install into a versioned directory, validate every digest and license record, then switch the active pointer only when no running job references the old version.

Alternatives considered:

- A Python/PyInstaller `pdf2docx` sidecar was rejected for AGPL dependency, maintenance, size, and signing reasons.
- A Rust/PDFium sidecar was rejected for native artifact complexity and lack of an end-to-end DOCX renderer.
- Embedding all engine assets permanently in the Zero app would be simpler but would make the plugin installation boundary misleading and couple engine updates to host releases.

### 2. Run conversion in a persistent hidden engine WebView with Rust-owned staging

The host creates one hidden `zero-file-engine` WebView only after the signed Zero File package passes activation. It loads the installed engine harness from a read-only custom asset protocol, has `connect-src 'none'`, no opener/navigation access, and only the commands required for a validated job. The WebView outlives the visible File panel, so changing tools or closing the tray does not cancel active work.

Rust copies a validated input into a per-job directory under Zero-owned app-local data and gives the engine only that staging directory through a narrowly scoped filesystem capability. The engine reads `input.*` and writes only `provider-output.*`; it never receives an arbitrary destination path. Rust then applies the existing target signature/container validation and collision-safe final commit. This keeps large PDF, image, DOCX, and PDF output bytes off normal events/JSON IPC while preserving Rust ownership of all user-visible file changes.

The bridge uses a random per-job capability token, checks the calling WebView label and active plugin version on every operation, rejects unknown fields, and invalidates the token on success, failure, timeout, or cancellation. Startup cleans abandoned Zero-owned job directories but never user sources or completed outputs.

Alternatives considered:

- Running a worker inside the visible File panel would lose jobs when the panel unmounts.
- Sending whole documents as base64 Tauri payloads would multiply memory use and regress large-file behavior.
- Granting the plugin general filesystem permission would violate the existing path-security contract.

### 3. Implement PDF-to-DOCX with two truthful output profiles

PDF.js performs parsing, password detection, text/image extraction, and page rendering in the engine worker. `docx` produces the final OOXML package. Preflight classifies each document:

- `editableReconstruction`: used only when pages expose extractable text with stable baselines/reading order and do not trip complexity signals such as overlapping text, unsupported rotations, dense vector tables, or ambiguous multi-column flow. The engine reconstructs page sections, paragraphs/runs, basic images, font sizes, alignment, and page breaks. The output is editable but carries a fidelity warning.
- `layoutPreserving`: the guaranteed fallback for valid non-encrypted PDFs. Each page is rendered at a bounded resolution and placed as one page-sized image in a matching DOCX section. It preserves visible layout and works for scanned pages, but its text is not editable and the result metadata says so.

The automatic fallback is deliberate: a usable visual DOCX is preferable to “engine unavailable,” but the UI must name the profile before/after conversion. Encrypted PDFs still return `passwordRequired`. OCR is not implied; scanned PDFs complete only as `layoutPreserving`. Rendering uses page-at-a-time memory, bounded pixel counts, JPEG/PNG selection based on page content, and cancellation between extraction/render/write stages.

The first implementation will pin the classification thresholds in `file-engine-policy.json` after running the existing Latin/CJK, tables, images, columns, scanned, encrypted, malformed, and large-document corpus. A failure in editable generation retries once with the layout-preserving profile before the job is reported failed.

### 4. Implement macOS DOCX-to-PDF with `docx-preview`, bounded WebKit capture, and PDFKit merging

On macOS 11+, the engine WebView loads the staged DOCX with `docx-preview`. It renders fonts, images, headers, footers, footnotes, section dimensions, manual breaks, and available `lastRenderedPageBreak` data. Export-only CSS removes viewer chrome, normalizes CSS points to native PDF points, preserves explicit section/page breaks, and exposes each rendered page as a bounded capture rectangle.

After `renderAsync`, image decoding, and `document.fonts.ready` complete, the engine applies export-only CSS and signals `renderReady` with a bounded list of measured page rectangles. Rust accesses the engine's native `WKWebView` through Tauri's platform WebView handle, captures each approved rectangle with `createPDFWithConfiguration:completionHandler:`, and merges those single-page captures with the system PDFKit framework into a Zero-owned temporary output. The bridge rejects duplicate/out-of-range pages, more than 512 pages, captures above the per-page/document byte limits, and timeouts; no print dialog or spool job is created. The resulting PDF is returned to the existing artifact validator and commit path.

The initial `printOperationWithPrintInfo:`/`NSPrintSaveJob` prototype was rejected during real hidden-WebView smoke because a two-page fixture produced an unbounded 100+ MiB spool before completion. The capture-and-merge path preserves native WebKit rendering while making page count, geometry, memory, output bytes, and timeout enforceable.

The provider is available only when all of the following are true: macOS is 11 or newer, the signed engine assets validate, the hidden WebView reaches ready state within the startup deadline, and the WebKit/PDFKit APIs are available. LibreOffice and Word remain lower-priority compatibility providers that a future preference may select for documents needing higher proprietary fidelity.

### 5. Keep provider selection and contracts explicit

The provider registry gains built-in IDs for `zeroFilePdfToDocx` and `zeroFileDocxToPdfMacos`. Built-in providers have first priority on supported fresh installs; existing providers remain registered as optional compatibility adapters. Capability snapshots add provider origin, engine/package version, supported quality profiles, readiness diagnostics, and platform minimum. Job results add the actual `qualityProfile` and localized warning keys.

Rust and TypeScript use matching enums and tagged objects. The visible panel never decides that an engine is ready from static package metadata alone; it renders the authoritative Rust snapshot. A verified fresh macOS 11+ install therefore shows both directions ready. `Convert all` is enabled when at least one valid queued job has a ready provider, exactly as today, but the normal supported install no longer enters the missing-engine branch.

If an asset is corrupt or a host is incompatible, installation/activation reports a plugin repair or upgrade error. `engineUnavailable` remains in the contract for damaged packages, unsupported platforms, and operational recovery, not as the normal first-run state.

### 6. Make release evidence part of the engine policy

`src-tauri/file-engine-policy.json` becomes the machine-readable authority for approved engine package version, component versions/licenses, artifact digests, output profiles, platform minimums, measured package sizes, and corpus acceptance. CI verifies that the built package contains only approved assets and complete notices, that lockfile-resolved transitive licenses satisfy the allowlist, and that no engine performs a network request.

Initial release budgets are:

- no more than 20 MiB additional compressed Zero File package size and 45 MiB installed engine assets;
- engine ready signal within 2 seconds on the release baseline after WebView creation;
- page-at-a-time PDF processing with a documented large-corpus memory ceiling and cancellation acknowledgement within 2 seconds;
- successful openable output for every supported corpus fixture, with profile-specific editable/layout assertions rather than a single misleading fidelity score.

The release build must smoke the installed/signed package, not merely the source-tree dev bundle. macOS tests include a Word-authored DOCX with no explicit manual breaks, CJK/font fallback, tables/images, a multi-page document, cancellation, app restart cleanup, and a conversion after removing LibreOffice/Word from discovery.

## Risks / Trade-offs

- [PDF-to-DOCX layout-preserving output is not editable] → Expose `layoutPreserving` before/after conversion, prefer editable reconstruction only behind the confidence gate, and keep a future higher-fidelity provider pluggable.
- [`docx-preview` does not implement Word's complete pagination algorithm] → Preserve explicit/last-rendered breaks, split measured overflow into bounded page rectangles, benchmark real Word fixtures, and expose a compatibility-provider option rather than claiming exact Word pagination.
- [A hidden WebView can consume memory or become unresponsive] → Create it lazily, process one Rust-owned job at a time, enforce startup/job deadlines, bound page pixels, cancel workers, and tear down/recreate the WebView after a crash or timeout.
- [Privileged plugin assets increase supply-chain risk] → Require a pinned first-party signature, per-file hashes, exact dependency versions, CSP/network denial, a dedicated WebView identity, and job-scoped staging permissions.
- [Plugin and host upgrades can race active jobs] → Keep versioned engine directories, pin each job to one engine version, and activate/remove versions only outside their reference lifetime.
- [macOS 10.x remains unable to use built-in DOCX-to-PDF] → Declare macOS 11+ in capability metadata and plugin compatibility; do not raise Zero's global minimum or silently claim availability on older systems.
- [The compressed package can exceed the lightweight target] → Ship only the PDF.js generic build/worker and required CMap/font assets, tree-shake generation code, enforce measured CI budgets, and reject release rather than silently accepting bloat.

## Migration Plan

1. Add the host-side engine trust metadata, signature validation, hidden WebView capability, staging service, and typed contracts behind an unapproved policy entry; keep current external-provider behavior active.
2. Build the Zero File engine harness and package from pinned dependencies, generate notices/digests, and make source-tree tests use the same manifest as release packaging.
3. Add the two built-in provider adapters and run the representative corpus, security, cancellation, memory, package-size, and real macOS PDF-export gates.
4. Approve the pinned package in `file-engine-policy.json`, publish it with the minimum host constraint, and migrate an existing bundled Zero File installation to the signed first-party package without changing the user's enabled state or queue history policy.
5. Switch provider priority to built-in-first and update the provider strip/action copy. Keep LibreOffice/Word adapters available for rollback and compatibility.
6. Roll back by disabling the approved package version in policy and selecting the prior external adapters. Never delete user inputs or completed outputs; remove only inactive versioned engine assets and Zero-owned staging files.

## Open Questions

- Windows DOCX-to-PDF remains intentionally outside this apply set. A follow-up must benchmark WebView2 `PrintToPdf`, font behavior, cancellation, installer/runtime compatibility, and packaged signing before enabling that direction by default.
- The exact editable-reconstruction confidence thresholds and large-corpus memory ceiling are release evidence, not guesses in this document; tasks require recording and pinning them before the policy can move from candidate to approved.
