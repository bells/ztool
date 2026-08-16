## 1. Baseline and engine acceptance evidence

- [x] 1.1 Record the current bundled-plugin inventory and run the existing descriptor/source-boundary tests, `pnpm test`, `pnpm build`, `cargo fmt --check`, `cargo check`, and `cargo test` before changing File-related composition or native registration.
- [x] 1.2 Add a non-sensitive conversion corpus covering Latin and CJK text, headings, fonts, tables, images, headers/footers, page breaks, columns, scanned pages, encrypted PDFs, malformed containers, and representative large documents, with expected outcome metadata rather than private user files.
- [x] 1.3 Create a durable engine decision record that pins the evaluated LibreOffice, Microsoft Word automation, `pdf2docx`/PyMuPDF, and ONLYOFFICE versions; records upstream maintenance, full redistribution licenses, package-size impact, offline behavior, security/process behavior, and supported platform/direction claims.
- [x] 1.4 Run available candidate providers against the corpus, record editable-text, reading-order, table/image, layout, invalid-input, timeout, and cancellation results, and decide which providers may be detected and which, if any, may be redistributed.
- [x] 1.5 Add a build/package guard proving no unapproved Python, PyMuPDF, OpenCV, LibreOffice, ONLYOFFICE, or other conversion binary is silently included or downloaded.

## 2. Plugin identity, composition, and contracts

- [x] 2.1 Add matching `zero.file` canonical identity constants and tests in frontend and Rust brand modules.
- [x] 2.2 Create the self-contained `src/plugins/file/` module with `plugin.tsx`, local presentation/i18n ownership, `accent-file`, the main view contribution, no default status-bar contribution, and one registration in `src/appShell/bundledPluginModules.ts`.
- [x] 2.3 Add the matching bundled Rust manifest/record, state registration, and composition metadata without exposing File commands or filesystem/process access through the sandboxed third-party Extension API.
- [x] 2.4 Define symmetric Rust and TypeScript capability, provider, candidate, enqueue request, direction, stage, progress, job-state, result, and structured-error contracts with no TypeScript `any` and stable camelCase serialization.
- [x] 2.5 Update descriptor, expected-plugin inventory, manifest uniqueness, handler-registration, and module-boundary tests so File is build-time pluggable and neither core nor a sibling plugin imports it.

## 3. Native validation, output safety, and queue model

- [x] 3.1 Implement and unit-test canonical source inspection for readable regular files, supported extensions, PDF headers, required DOCX ZIP entries, Office lock files, missing paths, directories, and duplicate active sources.
- [x] 3.2 Implement and unit-test direction detection plus collision-free `<stem>-converted.<ext>` output reservation, incremented fallback names, optional writable output-directory validation, and no-overwrite behavior.
- [x] 3.3 Implement owner-only per-job temporary directories, provider-output validation, safe final commit, stale temporary cleanup, and preservation of source/existing output files on every failure path.
- [x] 3.4 Implement the Rust-owned session queue and pure state transitions for queued, preparing, running, completed, failed, and cancelled jobs, including one-active-job scheduling and continuation after an item fails.
- [x] 3.5 Define the `FileConversionProvider` probe/convert interface, cancellation/progress sinks, deterministic direction-specific provider selection, and fake providers for success, failure, invalid output, timeout, percentage, and indeterminate tests.

## 4. Local conversion providers

- [x] 4.1 Implement provider discovery with explicit executable resolution, approved version ranges, capability reasons, probe caching/invalidation, and direct argument-array execution rather than shell strings.
- [x] 4.2 Implement and integration-test the LibreOffice DOCX-to-PDF provider using a dedicated profile/working directory, `--headless --convert-to pdf`, bounded diagnostics, validated output, timeout, and cancellation.
- [x] 4.3 Implement the optional macOS Microsoft Word DOCX-to-PDF adapter with Automation-permission, application/activation, cancellation, and user-visible-window limitations classified separately from missing-engine errors.
- [x] 4.4 Implement the optional Windows Microsoft Word DOCX-to-PDF COM adapter with application/activation, apartment/process cleanup, timeout, and permission failures classified separately from missing-engine errors.
- [x] 4.5 Implement detection and guarded execution for an approved user-installed PDF-to-DOCX provider, beginning with a compatible `pdf2docx` CLI only if its version and dependency/license decision from task 1 are accepted.
- [x] 4.6 Apply the recorded redistribution gate: integrate and checksum/sign/package an approved PDF-to-DOCX sidecar only if every gate passes, otherwise verify the shipping build exposes the direction as unavailable until a compatible user-installed provider is detected.
- [x] 4.7 Classify encrypted, image-only/OCR-required, malformed, unsupported-feature, provider-exit, permission, timeout, cancellation, and invalid-output outcomes into stable safe error codes without logging document contents or unredacted full paths.

## 5. Tauri commands, events, and result actions

- [x] 5.1 Add thin commands for capability snapshots, native multi-file selection, enqueue/list/start/cancel/remove/retry operations, and register them with the managed File state in the bundled composition root.
- [x] 5.2 Run the sequential worker independently of panel lifecycle, emit typed `zero://file-conversion/job-updated` snapshots, and prove list-on-mount reconciles events missed while the tray or main panel was closed.
- [x] 5.3 Implement bounded stdout/stderr capture, configurable provider deadlines, process-tree cancellation, application-shutdown cleanup, and queue continuation after provider failure.
- [x] 5.4 Implement `open_file_conversion_output` and `reveal_file_conversion_output` using completed job IDs, immediate result revalidation, and direct macOS/Windows operations without a general arbitrary-path launcher.
- [x] 5.5 Add Rust command/service tests for malformed payloads, unknown IDs, forbidden job states, missing/moved outputs, mixed batches, failure isolation, shutdown cleanup, and event/snapshot consistency.

## 6. Zero File panel and interaction model

- [x] 6.1 Implement `fileService.ts` and `useFileConversion.ts` as the typed IPC/event bridge with mount refresh, event cleanup, action-specific busy state, and error narrowing.
- [x] 6.2 Implement pure frontend queue helpers for direction labels, candidate reconciliation, actionable states, retry/remove/clear eligibility, provider guidance, and tray/main row summaries with focused unit tests.
- [x] 6.3 Add complete Simplified Chinese and English File strings for plugin presentation, file intake, directions, stages, states, provider guidance, actions, local-processing claims, and structured recovery messages.
- [x] 6.4 Build the adaptive `FilePanel` empty and intake states with desktop drag-and-drop, keyboard/native-picker activation, mixed-file validation feedback, source/target review, and explicit `Convert all` start behavior.
- [x] 6.5 Build queue rows and batch controls for queued, preparing, percentage/indeterminate running, completed, failed, cancelled, and engine-unavailable states, including cancel, remove, retry, clear, open, and reveal actions.
- [x] 6.6 Add compact tray and roomy main-window layouts using the source-to-output transfer rail, existing typography/theme/radius system, one cool-blue File accent, a single bounded queue scroll region, and explicit narrow-width stacking.
- [x] 6.7 Add queue-shaped loading, directional empty, inline alert/status, accessible progress, non-color state cues, visible focus, 44px compact/touch targets where practical, and reduced-motion behavior; audit every visible Chinese and English string for precise local/provider/fidelity claims.
- [x] 6.8 Add frontend tests for drag/pick intake, mixed valid/invalid batches, no auto-start on drop, truthful progress, job actions, event reconciliation, language changes, reduced motion, accessibility semantics, and tray/main responsive structure.

## 7. Integration, packaging, and verification

- [x] 7.1 Add integration fixtures with fake executables to verify argument escaping, paths containing spaces/non-ASCII characters, dedicated working directories, bounded output, timeout, cancellation, invalid provider results, and zero network fallback.
- [x] 7.2 Run focused File frontend/Rust tests, plugin descriptor/manifest/import-boundary tests, `pnpm test`, and `pnpm build`, confirming the recursive test inventory includes all new nested tests exactly once.
- [ ] 7.3 Run `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`, separating environment-only office/GUI failures from source regressions.
- [x] 7.4 Audit shipped artifacts, notices, hashes, sidecar allowlists, Tauri capabilities, binary size, and provider discovery paths against the accepted engine decision; verify no unapproved engine or broad third-party filesystem permission entered the build.
- [x] 7.5 Run `openspec validate add-file-plugin --type change --strict` and `git diff --check`, then inspect the final diff for symmetric IPC, one frontend/Rust composition entry, no sibling coupling, and no undocumented platform claim.
- [ ] 7.6 Manually smoke-test macOS with the actual tray/main UI, drag-and-drop, native picker, LibreOffice and available Word/PDF providers, Automation denial, background queue, cancellation, output collisions, open, Finder reveal, CJK paths, offline conversion, and app restart cleanup.
- [ ] 7.7 Manually smoke-test Windows with the actual main/tray UI, Explorer paths, LibreOffice and available Word/PDF providers, COM/activation denial, process-tree cancellation, output collisions, open, Explorer reveal, CJK paths, offline conversion, and app restart cleanup.
