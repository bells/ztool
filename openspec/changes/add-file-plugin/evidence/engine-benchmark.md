# Zero File provider corpus benchmark

Run date: 2026-08-16  
Host: macOS arm64 test environment  
Corpus: `tests/fixtures/fileConversion/expected.json`  
Decision authority: `engine-decision.md`

## Environment and method

- LibreOffice: Codex test-runtime `LibreOfficeDev 26.8.0.0.alpha0`, commit `2c87e51eeaa2b413ff4ae097b2705eea1995d8e5`.
- `pdf2docx`: isolated `/private/tmp` virtual environment with `pdf2docx 0.5.13`, PyMuPDF 1.28.2, Python 3.12, and the dependency versions recorded in `engine-decision.md`. Nothing was added to Zero's dependencies or package configuration.
- Microsoft Word and ONLYOFFICE were not installed, so no conversion or runtime claim was made for them.
- Every normal conversion used a dedicated provider profile/working/output directory. Output PDF text was inspected with pypdf; DOCX structure/text was inspected with python-docx and ZIP entry checks. Candidate DOCX results were rendered through the same LibreOffice test binary before visual review.
- Visual review covered all rich pages and pages 1, 12, and 24 of each 24-page large result. Text coverage is token-presence coverage, used only as a regression signal; it is not a fidelity score.

The local LibreOffice runtime emitted Fontconfig cache warnings because the sandbox home cache is not writable. The dedicated-profile conversions still completed, and no output used the repository or source directory for temporary files.

## LibreOffice DOCX-to-PDF

Invocation shape:

```text
soffice -env:UserInstallation=file://<job-profile> --headless --convert-to pdf --outdir <job-output> <input.docx>
```

| Case | Result | Editable text / order | Table, image, layout | Classification |
| --- | --- | --- | --- | --- |
| `rich-layout.docx` | exit 0, 0.72s, valid 3-page PDF | 99.4% source-token presence; CJK extractable and visible; column samples 1-8 in order | Table labels and embedded image visible; headers/footers on all three pages; explicit page break retained; no clipping/overlap on all pages | Pass |
| `large-structured.docx` | exit 0, 0.72s, valid 24-page PDF | 100% source-token presence; CJK extractable and visible | 24 pages retained; pages 1/12/24 visually clean; headers/footers repeated | Pass |
| `malformed.docx` | exit 0 and produced a one-page PDF containing `not-a-zip-package` | Not applicable | LibreOffice treated the renamed bytes as plain text instead of rejecting them | Provider invalid-input classification fails; Rust DOCX ZIP preflight is mandatory |
| timeout at 10ms | process-group leader exit `-15` after 12.1ms; no output | Not applicable | SIGTERM was sufficient; no forced SIGKILL | Pass for bounded termination experiment |
| cancellation at 50ms | process-group leader exit `-15` after 57.8ms; no output | Not applicable | SIGTERM was sufficient; no forced SIGKILL | Pass for bounded termination experiment |

LibreOffice provides lifecycle output but no stable percentage protocol for this command, so Zero must report indeterminate progress/stages. Full descendant-process enumeration was unavailable in the sandbox; task 7 process-tree tests and real macOS/Windows smoke remain required.

Decision: approve detection of a user-installed compatible LibreOffice as the preferred DOCX-to-PDF provider. Do not redistribute it. Validate DOCX structure before launch and validate the resulting PDF after exit even when the process returns zero.

## `pdf2docx` 0.5.13 PDF-to-DOCX

Invocation shape:

```text
pdf2docx convert <input.pdf> --docx_file=<job-output.docx>
```

The isolated environment occupied about 268 MiB unpacked. `pdf2docx --version` is not supported and exits with an argument error; version detection would have to resolve the script's Python environment and package metadata rather than trust a normal CLI version probe.

| Case | Result | Editable text / order | Table, image, layout | Classification |
| --- | --- | --- | --- | --- |
| `rich-layout.pdf` | exit 0, 0.70s, valid DOCX container | 100% token presence; column paragraphs 1-8 remain ordered | One table and one image retained, but the rendered DOCX grows from 2 to 3 pages and every CJK run is visually absent even though its XML text exists | Fail: unacceptable CJK/layout fidelity |
| `large-structured.pdf` | exit 0, 0.79s, valid DOCX container | 100% token presence | 24 pages retained, but CJK is visually absent on representative pages | Fail: unacceptable CJK fidelity |
| `image-only-scan.pdf` | exit 0, 0.33s, valid DOCX container | No editable text | Produces one embedded raster image and logs that scanned PDFs are unsupported | Must be rejected as `ocrRequired`; provider exit zero is a false success for Zero's product contract |
| `encrypted.pdf` | exit 1, 0.25s, no output | Not applicable | Detects that a password is required, then exposes an internal `KeyError: raw_exceptions` traceback | Map to `passwordRequired`; bound/redact diagnostics |
| `malformed.pdf` | exit 1, 0.25s, no output | Not applicable | Emits a full PyMuPDF/pdf2docx traceback | Map to `invalidInput`; bound/redact diagnostics |
| timeout at 10ms | process-group leader exit `-15` after 16.6ms; no output | Not applicable | SIGTERM sufficient in this experiment | Pass for bounded termination experiment |
| cancellation at 50ms | process-group leader exit `-15` after 56.0ms; no output | Not applicable | SIGTERM sufficient in this experiment | Pass for bounded termination experiment |

The CLI writes ANSI-colored phase/page logs, not a supported machine-readable progress protocol. Zero must treat it as indeterminate rather than parsing a fabricated percentage. The benchmark did not exercise the password argument because password entry is a declared non-goal.

Decision: reject `pdf2docx` 0.5.13 for both user-installed detection and redistribution in the first shipping build. PDF-to-DOCX remains `engineUnavailable`. A future maintained fork/provider must rerun the same corpus and pass visible CJK, scanned/encrypted classification, version probing, and license gates before this decision changes.

## Microsoft Word and ONLYOFFICE

| Candidate | Availability | Decision |
| --- | --- | --- |
| Microsoft Word macOS/Windows | Not installed on this Mac; Windows environment unavailable | Adapter/probe implementation may proceed as an optional user-installed DOCX-to-PDF fallback, but it cannot be reported verified until tasks 7.6/7.7 exercise activation, permission denial, cancellation, output, and cleanup on the real platforms. Never redistribute Word. |
| ONLYOFFICE Desktop Editors 9.4.0 | Not installed | Do not detect or redistribute in this change. No unattended CLI, fidelity, process, or offline behavior was benchmarked. |

## Accepted provider set after benchmark

- Detected and executable: compatible user-installed LibreOffice for DOCX-to-PDF.
- Optional adapter pending real-platform smoke: user-installed Microsoft Word for DOCX-to-PDF.
- Explicitly unavailable: PDF-to-DOCX; `pdf2docx` 0.5.13 failed the accepted quality gate.
- Excluded: ONLYOFFICE and every bundled/downloaded conversion engine.

These decisions keep source files local and preserve an honest unavailable state instead of treating a container, zero exit code, or extractable-but-invisible text as conversion success.
