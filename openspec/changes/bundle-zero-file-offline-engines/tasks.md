## 1. Baseline and pinned engine evidence

- [x] 1.1 Run and record the current File unit/integration tests, `pnpm test`, `pnpm build`, `cargo fmt --check`, `cargo check`, `cargo test`, and `git diff --check` before changing provider or plugin-package behavior.
- [x] 1.2 Re-run the existing file-conversion corpus and preserve its current external-provider results so the built-in engine comparison has a reproducible baseline.
- [x] 1.3 Pin approved versions of `pdfjs-dist`, `docx`, and `docx-preview`, record their direct/transitive licenses and upstream revisions, and add required license/NOTICE material without adding Python, PyMuPDF, OpenCV, LibreOffice, ONLYOFFICE, Chromium, or Office runtime dependencies.
- [x] 1.4 Add a build script that emits only the required PDF.js build/worker/CMap/font assets plus the Zero File engine harness, measures compressed/installed size, and fails above the 20 MiB/45 MiB budgets.
- [x] 1.5 Extend the conversion corpus expectations with `editableReconstruction`, `layoutPreserving`, and `webRenderedPdf` outcomes, including CJK, columns, rotations, tables, scans, encrypted inputs, Word pagination, and large documents.

## 2. First-party engine package trust and lifecycle

- [x] 2.1 Extend Rust and TypeScript plugin manifest contracts with exact first-party engine metadata for host API range, directions, platform minimums, asset digests, notices, and `document.convert`, preserving backward compatibility for generic `.zplugin` packages.
- [x] 2.2 Add a pinned Ed25519 first-party verification key and verify the detached engine manifest signature plus every declared file digest before granting privileged engine trust.
- [x] 2.3 Reject unsigned, malformed, generic, wrong-host-version, wrong-platform, path-escaping, duplicate, oversized, or digest-mismatched engine packages with focused package-validation tests.
- [x] 2.4 Install candidate engine bundles into versioned staging directories and atomically activate them only after full validation, leaving the previous active version recoverable on failure.
- [x] 2.5 Track running-job references to engine versions so upgrade/uninstall cannot remove active assets, then clean only inactive version directories after jobs release them.
- [x] 2.6 Migrate the existing `zero.file` bundled record to the signed first-party engine package while preserving its canonical ID, enabled state, contributions, localization, and module-boundary guarantees.
- [x] 2.7 Add package/registry tests proving an installed Zero File plugin becomes ready on a compatible host and that another plugin cannot impersonate its trust tier or privileged bridge.

## 3. Persistent engine runtime and staging boundary

- [x] 3.1 Add a Rust engine-session service that creates canonical per-job directories under Zero-owned app-local data, copies only an already validated input, reserves fixed input/output names, and never exposes the final user destination to engine code.
- [x] 3.2 Issue random one-time job capability tokens bound to plugin ID, engine version, WebView label, job ID, direction, staging root, and deadline; revoke them on every terminal path.
- [x] 3.3 Add a read-only installed-engine asset protocol with traversal/symlink/MIME protections and a strict engine CSP that denies remote network, navigation, opener, and undeclared Tauri APIs.
- [x] 3.4 Create the hidden `zero-file-engine` WebView lazily after asset verification, route it to the engine harness instead of `MainApp`, and keep it alive independently from File panel mount/unmount.
- [x] 3.5 Grant the engine WebView only job-staging read/write and typed engine-control operations, with command-side checks for caller label, token, plugin version, expected filenames, byte limits, and exact fields.
- [x] 3.6 Implement typed readiness, stage/progress, completion, cancellation, and crash messages with symmetric Rust/TypeScript definitions and protocol-version rejection tests.
- [x] 3.7 Add startup timeout, health probing, crash detection, bounded diagnostics, lazy teardown/recreation, and retryable provider errors without letting engine failures terminate the main app.
- [x] 3.8 Clean abandoned Zero-owned job directories on startup and after success/failure/cancel/timeout while proving user inputs and completed outputs are never removed.
- [x] 3.9 Add source-contract and integration tests proving document bytes stay in staging files rather than cross-WebView events or unbounded base64/JSON IPC.

## 4. Built-in PDF-to-DOCX provider

- [x] 4.1 Build the plugin engine worker around PDF.js with local worker/CMap/font resolution, password/error classification, page-at-a-time loading, bounded pixel counts, and no network fallback.
- [x] 4.2 Implement deterministic PDF complexity analysis for text coverage, baseline order, overlap, rotations, columns, vector/table density, and image-only pages, then pin accepted confidence thresholds in tests.
- [x] 4.3 Implement `editableReconstruction` DOCX generation with matching page sections, ordered paragraphs/runs, basic alignment/font sizing, supported images, and explicit page breaks using `docx`.
- [x] 4.4 Implement the guaranteed `layoutPreserving` DOCX path with bounded page rasterization, page-size/rotation preservation, adaptive PNG/JPEG choice, and an explicit non-editable warning.
- [x] 4.5 Remove partial editable artifacts and retry once with `layoutPreserving` when editable generation fails, while preserving `passwordRequired` for encrypted PDFs and never claiming OCR.
- [x] 4.6 Add cooperative cancellation checkpoints around parsing, extraction, page render, image encoding, and DOCX packaging and acknowledge cancellation inside the approved two-second deadline.
- [x] 4.7 Add `zeroFilePdfToDocx` to the Rust provider registry, route it through the existing queue/output validator/commit path, and select it before optional external providers when its signed engine is ready.
- [x] 4.8 Add worker/unit/provider integration tests for every PDF fixture, output container validity, quality metadata, fallback behavior, malformed responses, timeout, crash, cancellation, and external-provider absence.

## 5. Built-in macOS DOCX-to-PDF provider

- [x] 5.1 Render staged DOCX files in the engine WebView with pinned `docx-preview` options for fonts, images, headers, footers, footnotes, sections, manual breaks, and available last-rendered breaks.
- [x] 5.2 Add isolated export CSS that removes plugin chrome, preserves section dimensions/orientation and explicit breaks, and exposes bounded page rectangles for native WebKit capture.
- [x] 5.3 Signal render readiness only after `renderAsync`, embedded image decoding, and `document.fonts.ready`, returning measured sections/pages and structured resource errors.
- [x] 5.4 Implement the macOS 11+ native export adapter using engine `WKWebView.createPDF`, bounded per-page rectangles/bytes/timeouts, PDFKit page merging, and a Zero-owned temporary output URL without displaying a dialog.
- [x] 5.5 Bound native capture/merge startup and execution, propagate cancellation and native errors, discard partial PDFs, and validate PDF signature/page count before the existing final commit.
- [x] 5.6 Add `zeroFileDocxToPdfMacos` to provider discovery with macOS version/API/engine-readiness checks and built-in-first priority over LibreOffice and Microsoft Word.
- [x] 5.7 Add Rust and engine tests for render readiness, font/image timeout, multi-page output, portrait/landscape sections, malformed DOCX, cancellation, WebView crash, and unsupported macOS versions.
- [ ] 5.8 Run a real packaged macOS smoke on macOS 11+ with LibreOffice and Word absent, covering a Word-authored no-manual-break document, CJK/font fallback, tables/images, multiple pages, cancellation, open, and reveal.

## 6. Capability contracts and ready-state UI

- [x] 6.1 Extend matching Rust/TypeScript provider contracts with built-in provider IDs, origin, engine/package version, platform minimum, supported quality profiles, readiness diagnostics, result profile, and warning keys without `any`.
- [x] 6.2 Replace the hard-coded PDF-to-DOCX unavailable capability with authoritative built-in readiness and keep `engineUnavailable` only for unsupported platforms, incompatible hosts, damaged installs, or runtime recovery.
- [x] 6.3 Update provider selection and queue tests so a verified fresh macOS 11+ install exposes both directions and valid PDF/DOCX jobs enable `Convert all` without LibreOffice, Word, or Python.
- [x] 6.4 Update the provider strip and job details to name the built-in engines, show offline readiness and actual fidelity profile, and remove normal “direction unavailable / missing engine” copy on supported installs.
- [x] 6.5 Add plugin repair/reinstall guidance for integrity/startup failures and compatibility-provider labeling for optional LibreOffice/Word without presenting either as required.
- [x] 6.6 Add Simplified Chinese and English copy for engine readiness, quality profiles, non-editable layout preservation, platform minimums, repair, and structured engine errors.
- [x] 6.7 Add unit/source-contract/accessibility coverage for enabled/disabled `Convert all`, provider status, quality warnings, keyboard flow, live error announcements, reduced motion, and panel remount during a hidden-engine job.

## 7. Policy, packaging, and release gates

- [ ] 7.1 Extend `file-engine-policy.json` with the candidate engine package digest, exact component/license inventory, output profiles, host/platform constraints, confidence policy, size measurements, and benchmark evidence while leaving it unapproved initially.
- [x] 7.2 Upgrade `verify-file-engine-packaging.mjs` and tests to require the signed approved asset set/notices, reject undeclared engines or runtime downloads, and continue proving conversion has no cloud/network fallback.
- [ ] 7.3 Run the complete corpus through the installed engine package, record profile-specific text/layout/page assertions, choose and pin confidence thresholds plus the large-corpus memory ceiling, and fix or explicitly reject every required failure.
- [ ] 7.4 Measure package size, cold readiness, conversion timeout, cancellation latency, memory, temporary-file cleanup, and restart recovery on release baselines and fail policy approval if any budget is exceeded.
- [x] 7.5 Add release workflow steps that reproducibly build the engine bundle, generate digests/notices, sign the manifest without exposing the private key, package Zero File, and verify the installed artifact before publication.
- [ ] 7.6 Approve only the tested package digest/directions/platforms in policy after every automated and manual gate passes, with a rollback switch to existing external providers.
- [x] 7.7 Add Windows CI coverage for the platform-independent PDF-to-DOCX package path and keep Windows DOCX-to-PDF explicitly unavailable pending a separate WebView2 proposal and runtime smoke.

## 8. Final verification and handoff

- [x] 8.1 Run focused File/plugin/package/engine unit and integration tests, then run `pnpm test`, `pnpm build`, `cargo fmt --check`, `cargo check`, `cargo test`, and `git diff --check`.
- [ ] 8.2 Install the signed Zero File artifact into a clean profile and verify both macOS directions work offline immediately after plugin installation with external providers absent.
- [ ] 8.3 Verify upgrade, rollback, uninstall-with-active-job protection, corrupt-package repair, app restart cleanup, panel close/reopen, timeout, cancellation, output collision, open, and reveal in the real packaged app.
- [x] 8.4 Update Zero File maintainer/user documentation with supported platforms, dependency licenses, offline guarantees, fidelity profiles, size impact, repair steps, compatibility providers, and the Windows DOCX-to-PDF boundary.
- [x] 8.5 Reconcile the three remaining `add-file-plugin` engine/manual-verification tasks with this change's evidence, preserve both unarchived changes, and do not archive either change automatically.
