# Zero File offline engines

Installing the signed first-party `zero.file` plugin installs its conversion assets at the same time. Conversion is local and does not require Python, LibreOffice, Microsoft Word, Chromium, an account, a cloud service, or a runtime download.

## Supported directions

| Direction | Built-in implementation | Platform boundary | Result profile |
| --- | --- | --- | --- |
| PDF to DOCX | PDF.js 6.2.108 plus `docx` 9.7.1 | macOS and Windows desktop WebViews | `editableReconstruction` for simple ordered text; otherwise `layoutPreserving` |
| DOCX to PDF | `docx-preview` 0.4.0, bounded `WKWebView.createPDF`, and PDFKit merge | macOS 11+ | `webRenderedPdf` |

`layoutPreserving` puts each rendered PDF page into a matching DOCX page. It preserves visible layout but its text is not editable and no OCR is claimed. `webRenderedPdf` is not a promise of exact proprietary Word pagination. LibreOffice and Microsoft Word remain optional compatibility providers; neither is required for the built-in path. Windows DOCX-to-PDF remains unavailable until a separately tested WebView2 exporter is approved.

## Installation, integrity, and repair

The host grants `document.convert` only to a package whose identity is exactly `zero.file`, whose archive digest is explicitly approved by the embedded release policy, whose Ed25519 manifest signature verifies with Zero's pinned release key, and whose declared engine files, sizes, media types, notices, and SHA-256 digests all match. The host rechecks policy approval and installed assets before issuing a job lease. A generic plugin cannot obtain this trust by copying manifest fields.

Install and update use a versioned staging directory and an atomic registry switch. A failed activation retains the prior version. Running jobs hold their engine version; update cleanup retains leased assets, and uninstall is rejected until active jobs finish. Integrity or startup failures appear as a Zero File repair/reinstall error rather than instructions to install an office suite.

The engine runs in the hidden `zero-file-engine` WebView. Its read-only custom protocol rejects traversal, encoded paths, symlinks, unsupported media types, other WebView callers, and inactive versions. Document bytes use Zero-owned staging plus bounded raw IPC; the engine never receives the final user destination. Rust owns input validation, queueing, cancellation, output validation, collision-safe commit, open, and reveal.

## Size and licenses

The current candidate contains about 4.78 MiB of prepared PDF assets (2.46 MiB total per-file gzip measurement) plus about 1.2 MiB of engine harness code. Release limits are 20 MiB compressed and 45 MiB installed.

- PDF.js 6.2.108 — Apache-2.0
- `docx` 9.7.1 — MIT
- `docx-preview` 0.4.0 — Apache-2.0
- WebKit and PDFKit — macOS system frameworks, not redistributed

The package includes the direct license texts and `THIRD_PARTY_NOTICES.md`. Python, PyMuPDF, OpenCV, LibreOffice, ONLYOFFICE, Chromium, and Office binaries are not redistributed.

## Maintainer release workflow

Prepare and test the reproducible assets:

```bash
pnpm file-engine:build
node scripts/verify-file-engine-packaging.mjs
pnpm test
pnpm build
```

Create the release package only in the protected release environment:

```bash
export ZERO_FILE_ENGINE_SIGNING_KEY='<base64 Ed25519 32-byte release seed>'
pnpm file-engine:package
```

The private key must come from the release secret store and must never be committed or written into generated metadata. Packaging refuses a missing key or a key that does not match the pinned public key. It builds an independent engine entrypoint, generates the signed manifest and per-file digests, includes notices, and emits `build/zero-file-1.0.0.zplugin` plus its reported SHA-256.

Keep `src-tauri/file-engine-policy.json` unapproved until the signed package passes the complete corpus, size/readiness/memory/cancellation measurements, clean-profile offline install, macOS packaged smoke, signing/notarization, rollback, repair, and platform gates. Record the final package digest and only the directions/platforms actually tested.
