# Zero File local conversion engine decision

Decision date: 2026-08-16  
Scope: Zero File 1.0 on macOS and Windows desktop  
Status: accepted for implementation; redistribution remains denied unless this record is amended with release-owner approval

## Decision

Zero will ship provider adapters and capability detection, not a document engine. The first provider order is:

1. DOCX-to-PDF: a compatible user-installed LibreOffice, then a compatible user-installed Microsoft Word.
2. PDF-to-DOCX: unavailable in the first shipping build. `pdf2docx` 0.5.13 failed the task 1.4 CJK/layout/OCR/error-quality gate and is not an approved detected provider.
3. ONLYOFFICE: neither detected nor redistributed in this change because its unattended desktop conversion contract has not been accepted.

No provider, Python runtime, office suite, model, or installer may be downloaded at runtime. Provider probing and conversion must use direct executable paths and argument arrays. Zero's UI must name the selected external provider and may claim only that Zero itself does not upload the document.

## Evaluated version pins and product claims

| Candidate | Pinned evidence | Maintenance | Allowed platform/direction claim | Detection | Redistribution |
| --- | --- | --- | --- | --- | --- |
| LibreOffice | Local test binary `LibreOfficeDev 26.8.0.0.alpha0`, commit `2c87e51eeaa2b413ff4ae097b2705eea1995d8e5`; upstream stable directories `25.8.7` and `26.2.5` | Active; upstream repository updated on 2026-08-16 | macOS/Windows DOCX-to-PDF through `soffice --headless --convert-to pdf`; production detection accepts stable releases `>=25.8.0` and `<27.0.0` and rejects Development, alpha, beta, and release-candidate builds | Yes, preferred; the local alpha is benchmark evidence, not an accepted production provider | No |
| Microsoft Word automation | macOS update baseline `16.112 (26081010)`; Windows Microsoft 365 baseline `2607 (Build 20228.20190)`; no Word binary installed on this test Mac | Active proprietary Microsoft 365 product | Optional macOS DOCX-to-PDF through Apple Events for stable `16.80 <= version < 17`; Windows DOCX-to-PDF through Word COM requires Office major version `16` and an activated Microsoft 365/Office installation; availability and cleanup claims remain subject to platform smoke tests | Yes, fallback after implementation; runtime claims remain unverified until platform smoke tests | No; user supplies and licenses Word |
| `pdf2docx` / PyMuPDF | `pdf2docx 0.5.13` wheel SHA-256 `a293e9e78d89b12a4a43fcefba1346de220681c3daf20b8a7d3e1fce77f0fe97`; PyMuPDF `1.28.2` | `pdf2docx` repository is not archived and released 0.5.13 on 2026-05-01, but its own package description says Artifex no longer actively maintains it; PyMuPDF remains active | No shipping claim; task 1.4 found invisible CJK after rendering, rich-layout pagination drift, false success for image-only input, and unsafe internal traces for expected errors | No | Denied: quality gate failed, PyMuPDF requires AGPL-3.0 compliance or an Artifex commercial license, and no commercial license was supplied |
| ONLYOFFICE Desktop Editors | `9.4.0`, released 2026-05-19; arm64 DMG SHA-256 `e965be2222609add6b5a70baa2a8cdb599402491fb2925825d9039dcb154beb4`, x86_64 DMG SHA-256 `43ac517493c0c316f268ce4b7dc3810b77a7aefe83c0edc1655476d9f21681d2` | Active | No product claim in this change; common office/PDF editing support does not prove a stable unattended PDF-to-DOCX or DOCX-to-PDF desktop CLI | No | Denied pending a separate CLI, fidelity, license, signing, and size review |

Version pins record what was inspected, not a promise to accept every future update. Provider discovery must reject an executable outside its explicit approved range as `engineVersionUnsupported`; changing a range requires updating this record and rerunning the corpus.

## Redistribution license review

### LibreOffice

LibreOffice source uses MPL-2.0 with an LGPL-3.0-or-later secondary license and includes third-party notices. Its license permits distribution when obligations are satisfied, but permission alone does not justify adding a full office suite to Zero. Zero will execute only a user-installed copy. No LibreOffice executable, framework, resource, license bundle, or updater is approved for the Zero package.

### Microsoft Word

Microsoft Word is proprietary software governed by the user's Microsoft license. Automation does not grant Zero redistribution rights. Zero will not copy, install, patch, or package Word and will treat activation, Automation consent, Protected View, and application startup failures separately from a missing engine.

### `pdf2docx` resolved candidate graph

The 0.5.13 package declares `PyMuPDF>=1.26.7`, `python-docx>=0.8.10`, `fonttools>=4.24.0`, `numpy>=1.17.2`, `opencv-python-headless>=4.5`, and `fire>=0.3.0`. The versions below were the current PyPI resolution inputs inspected on 2026-08-16; this is a license inventory, not an approved lock file.

| Package | Inspected version | Redistribution license |
| --- | --- | --- |
| `pdf2docx` | 0.5.13 | MIT |
| PyMuPDF | 1.28.2 | AGPL-3.0 or Artifex commercial license |
| `python-docx` | 1.2.0 | MIT |
| FontTools | 4.63.0 | MIT |
| NumPy | 2.5.2 | BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0 as reported by PyPI |
| `opencv-python-headless` | 5.0.0.93 | Apache-2.0 for the package; binary third-party notices/codecs still require artifact-level review |
| Fire | 0.7.1 | Apache-2.0 |
| lxml | 6.1.1 | BSD-3-Clause |
| `typing-extensions` | 4.16.0 | PSF-2.0 |
| termcolor | 3.3.0 | MIT |

Because PyMuPDF is not permissively licensed for unrestricted proprietary redistribution, the graph is not approved for bundling. A future commercial-license proposal must also pin every platform wheel, hash the full resolved graph, collect all notices, review native libraries/codecs, and include the Python runtime license and size.

### ONLYOFFICE Desktop Editors

The upstream repository reports AGPL-3.0 and ships extensive third-party components. Zero does not approve a combined or separate-app redistribution theory in this change. Any future proposal must obtain legal review for the exact distribution mode and preserve the complete corresponding-source/notices obligations applicable to the chosen artifact.

## Package-size impact

- Current Zero impact: `0 bytes`; every provider is external and no provider artifact is referenced by Tauri bundling.
- The local LibreOfficeDev app used for evaluation occupies about `410 MiB` unpacked. This is Codex test infrastructure, not a Zero asset.
- The inspected ONLYOFFICE 9.4.0 downloads are `542,306,356` bytes for arm64 macOS and `555,347,165` bytes for x86_64 macOS before installation.
- The largest current wheel per package in the inspected `pdf2docx` graph totals about `114.7 MiB` compressed. That figure is deliberately conservative and excludes the Python runtime, unpacking, platform duplication, signing, and notices.
- Word size is not measured because Zero neither distributes nor installs it; user-installed detection adds only adapter code.

These figures exceed an acceptable implicit utility-plugin cost. Any future sidecar needs an explicit compressed/uncompressed delta against a release build and release-owner approval.

## Offline, security, and process behavior

| Candidate | Offline behavior controlled by Zero | Security/process constraints and known limitations |
| --- | --- | --- |
| LibreOffice | Zero passes local paths only and makes no network request | Launch `soffice` directly with a per-job working directory and dedicated user profile; bound stdout/stderr and deadline; kill the process tree on cancellation. The external app may retain its own preferences or update settings outside Zero's control. |
| Word on macOS | Zero uses local Apple Events only | macOS may prompt for Automation permission; Word may activate or show UI. Cancellation and shutdown must close only the Zero-owned document/operation where possible, not terminate unrelated user work. |
| Word on Windows | Zero uses local COM only | Requires correct COM apartment setup, installed/activated Word, and deterministic document/application release. Protected View, activation, policy, and permission failures are not `engineUnavailable`. |
| `pdf2docx` | Zero would invoke an already installed local CLI and make no network request | PDF parsing and image processing are CPU/memory intensive and process untrusted containers. Use a direct executable path, dedicated directory, output validation, bounded diagnostics, timeout, cancellation, and no implicit `pip`/engine download. Image-only and encrypted inputs are rejected rather than presented as successful conversions. |
| ONLYOFFICE | Not executed in this change | Large GUI/native attack surface and unattended lifecycle are not yet characterized; no probe or conversion path is approved. |

Zero cannot guarantee that an independently installed provider never performs its own license/update checks. Product copy therefore says “Zero does not upload files” and identifies the provider; it does not say “100% private,” “perfect formatting,” or “works with every document.”

## Acceptance gate

A provider may be redistributed only when all of the following are committed for the exact platform artifact:

1. exact version, download URL, SHA-256, signing/notarization or Authenticode evidence, and reproducible package contents;
2. complete direct/transitive license and notice set, including native libraries and runtime;
3. compressed/uncompressed Zero release delta and startup/probe measurements within an explicitly approved budget;
4. corpus results for editable text, reading order, tables, images, layout, malformed/encrypted/scanned input, large files, timeout, cancellation, and cleanup;
5. proof that probing/conversion performs no Zero network fallback or runtime download;
6. release-owner approval recorded in this file.

Failing any gate means the shipping build excludes the artifact. Detection of a user-installed provider is a separate, narrower decision and still requires an approved version range and corpus evidence.

## Sources checked on 2026-08-16

- LibreOffice official downloads: <https://download.documentfoundation.org/libreoffice/stable/>
- LibreOffice licensing: <https://www.libreoffice.org/about-us/licenses/>
- LibreOffice source repository: <https://github.com/LibreOffice/core>
- LibreOffice command-line parameters: <https://help.libreoffice.org/latest/en-US/text/shared/guide/start_parameters.html>
- Microsoft Office for Mac update history: <https://learn.microsoft.com/en-us/officeupdates/update-history-office-for-mac>
- Microsoft 365 Apps update history: <https://learn.microsoft.com/en-us/officeupdates/update-history-microsoft365-apps-by-date>
- Word VBA PDF export contract: <https://learn.microsoft.com/en-us/office/vba/api/word.document.exportasfixedformat>
- `pdf2docx` repository/release: <https://github.com/ArtifexSoftware/pdf2docx/releases/tag/v0.5.13>
- `pdf2docx` package metadata: <https://pypi.org/project/pdf2docx/0.5.13/>
- PyMuPDF licensing/package metadata: <https://pypi.org/project/PyMuPDF/1.28.2/>
- ONLYOFFICE Desktop Editors release: <https://github.com/ONLYOFFICE/DesktopEditors/releases/tag/v9.4.0>
- ONLYOFFICE Desktop Editors source/license: <https://github.com/ONLYOFFICE/DesktopEditors>
