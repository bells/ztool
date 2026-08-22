## Why

Zero File currently ships a complete conversion workflow but no approved built-in engine, so a fresh installation truthfully reports “no available engine” and cannot perform either direction without LibreOffice or Microsoft Word. The plugin should own a permissively licensed offline engine stack so installing Zero File is sufficient to convert supported files without Python, an office suite, a network request, or a second setup step.

## What Changes

- Package Zero File's conversion runtime and third-party notices as versioned, integrity-checked assets of the first-party plugin, with an explicit minimum Zero host version and supported-platform declaration.
- Replace the PDF-to-DOCX “no approved provider” path with a built-in local pipeline based on maintained permissive components: PDF.js for PDF parsing/rendering and `docx` for DOCX generation.
- Guarantee a valid layout-preserving DOCX for supported non-encrypted PDFs, use editable reconstruction only when the page content passes a documented confidence gate, and surface the resulting fidelity profile instead of claiming universal editability.
- Add a macOS built-in DOCX-to-PDF pipeline based on `docx-preview` rendering in an isolated local WebView, bounded `WKWebView.createPDF` page capture, and native PDFKit merging, with no Microsoft Word or LibreOffice dependency.
- Retain installed LibreOffice and Microsoft Word only as optional, explicitly selected compatibility providers; they are no longer prerequisites for the built-in directions.
- Make capability discovery treat verified plugin assets and the supported native PDF-export bridge as ready providers immediately after installation, while preserving structured errors for corrupt assets, unsupported host/platform versions, encrypted inputs, timeouts, cancellation, and invalid outputs.
- Remove the normal “direction unavailable / missing engine” presentation on supported fresh installs, keep file intake and `Convert all` enabled whenever valid queued jobs exist, and show concise offline/fidelity/provider information instead.
- Add dependency pinning, license attribution, supply-chain integrity, package-size budgets, representative corpus benchmarks, and real packaged-app smoke tests as release gates.

## Capabilities

### New Capabilities

- `zero-file-offline-engines`: Defines plugin-owned offline engine installation, built-in PDF-to-DOCX and macOS DOCX-to-PDF behavior, truthful quality profiles, ready-state UI behavior, and packaging/release gates.

### Modified Capabilities

None. The related `local-file-conversion` and `file-conversion-workflow` capabilities still live in the active, unarchived `add-file-plugin` change rather than the main spec set; this change layers the built-in-engine contract over that implemented baseline without pretending those deltas are already main specs.

## Impact

- Frontend: `src/plugins/file/` gains plugin-owned engine workers/assets, fidelity-aware contracts and copy, and ready-state action logic; the normal supported-platform path no longer renders a missing-engine state.
- Native backend: `src-tauri/src/services/file/` gains built-in provider adapters and a macOS WebView print bridge while preserving the existing Rust-owned queue, validation, cancellation, output commit, and job-scoped result actions.
- Plugin/package model: the first-party Zero File package must declare engine assets, checksums, licenses, platform/architecture support, and minimum host API compatibility; installing or upgrading the plugin is atomic with its engine bundle.
- Dependencies: add pinned `pdfjs-dist` (Apache-2.0), `docx` (MIT), and `docx-preview` (Apache-2.0) distributions plus their reviewed transitive dependencies and notices. Python, PyMuPDF, OpenCV, LibreOffice, ONLYOFFICE, Chromium, and Microsoft Office are not redistributed.
- Platforms: PDF-to-DOCX is intended for supported desktop WebViews; built-in DOCX-to-PDF ships on macOS 11+ first. Windows WebView2 PDF export remains an explicit follow-up gate rather than an implied capability.
- Specs/history: implementation must reconcile the still-active `add-file-plugin` assumptions and update its remaining engine acceptance evidence without archiving either change automatically.
