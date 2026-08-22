# Source engine smoke — 2026-08-22

Environment: macOS 26.5.2, Tauri development build, hidden `zero-file-engine` WebView, no conversion request routed through Python, LibreOffice, Microsoft Word, or a network service.

The maintainer also completed a conversion through the visible Zero File UI with source-tree development assets enabled and confirmed the generated document quality was good. This verifies the real file-selection, queued-job, `Convert all`, and output path beyond the hidden-engine harness, while remaining below the signed packaged-app release gate.

## PDF to DOCX

| Source | Result | Validation |
| --- | --- | --- |
| `large-structured.pdf` | `editableReconstruction` | Completed; DOCX ZIP passed `unzip -t` and contained `[Content_Types].xml`, `_rels/.rels`, and `word/document.xml`. |
| `rich-layout.pdf` | `layoutPreserving` | Completed; two-page DOCX contained two PNG page images and passed `unzip -t`. |
| `image-only-scan.pdf` | `layoutPreserving` | Completed; one JPEG page facsimile, no OCR claim, DOCX passed `unzip -t`. |
| `encrypted.pdf` | `passwordRequired` | Rejected by Rust preflight; no final output or engine staging file remained. |

Runtime fixes proven by these smokes:

- hidden-WebView creation is explicitly dispatched to the Tauri main thread;
- engine listeners are registered before the ready handshake, including React StrictMode remount;
- older WebKit receives `Promise.withResolvers`, `URL.parse`, and `ReadableStream` async-iterator polyfills;
- PDF.js uses print intent so hidden-WebView rasterization does not wait on throttled `requestAnimationFrame`;
- the pinned vector threshold routes the rich table fixture to the truthful layout-preserving profile.

## DOCX to PDF

The initial `printOperationWithPrintInfo:`/`NSPrintSaveJob` prototype was rejected: the two-page rich fixture produced an unbounded 100+ MiB spool before completion. Every generated Zero-owned staging file from those failed smokes was explicitly removed; source fixtures were untouched.

The replacement uses bounded `WKWebView.createPDF` page rectangles and PDFKit merging:

| Source | Result | Validation |
| --- | --- | --- |
| `rich-layout.docx` | `webRenderedPdf` | Re-run after geometry and engine-version binding changes: completed through the real queue/hidden WebView; valid 57,031-byte PDF with 2 pages at 612×792 pt Letter (PDF 1.3). The earlier 612×1056 measurement is resolved. |
| `large-structured.docx` | `webRenderedPdf` | Completed; `pdfinfo` reported 24 pages, 612×792 pt Letter, 193 KiB, PDF 1.3. |

The native bridge rejects more than 512 rectangles, non-finite/out-of-range geometry, duplicate or missing capture callbacks, per-page captures above 32 MiB, total captures above 128 MiB, and capture timeout before PDFKit merge and the existing Rust-owned final commit.

This is source-tree evidence, not the signed clean-profile packaged-app gate required by tasks 5.8, 7.3–7.6, and 8.2–8.3.

## Automated regression coverage

The source smoke is backed by focused Rust and source-contract tests that now reject stale job and engine-version bindings, unknown completion fields, missing success metadata, unbounded warning metadata, expired capabilities, cancellation, conversion timeout, and simulated hidden-WebView crashes. Native export tests cover mixed portrait/landscape page rectangles, out-of-order callbacks, duplicate/missing/out-of-range pages, page/document byte ceilings, PDFKit page-count preservation, and the macOS/WebKit/PDFKit availability gate.

Frontend source-contract coverage requires DOCX rendering to complete before image and font readiness waits, pins resource-timeout diagnostics and section-preserving `docx-preview` options, and proves the visible File panel rehydrates from the Rust queue without owning the hidden engine lifetime. Accessibility coverage locks native keyboard controls, enabled/disabled `Convert all`, live status/error announcements, all quality-profile labels, and reduced-motion CSS.
