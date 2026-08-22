# Zero File deterministic optimization evidence

Date: 2026-08-22

This report covers deterministic code, build, and lifecycle evidence for tasks 8.2–8.6. It is not a real WKWebView RSS, cold/warm engine latency, packaging, fidelity, or device smoke result; those measurements remain in tasks 8.1 and 8.7.

## Capability lifecycle

- `FileConversionState` owns a generation-keyed capability snapshot. Repeated reads in one generation return the cached value and do not reconstruct or probe the provider list.
- Explicit invalidation causes cover engine install, upgrade, repair, removal, supported native-provider changes, and a lifecycle reset.
- Built-in providers re-read the trusted active engine only after invalidation. LibreOffice drops its fingerprinted probe, while the platform Word adapter refreshes its detected installation/runtime state.
- Engine package commands invalidate the File snapshot; a separate `refresh_file_conversion_capabilities` command performs supported native-provider invalidation on a blocking worker.
- Focused Rust tests prove one provider-registry initialization, one snapshot refresh for repeated reads, and exactly one on-demand rebuild after every supported invalidation cause.

## Engine lifetime and queue boundary

- The Rust queue remains one-active-job sequential. No CPU-count or parallel conversion worker was added.
- One compatible hidden engine remains ready across adjacent jobs. Only the worker's queue-empty path may schedule teardown, and the bridge additionally requires an empty session map.
- Idle retention is 30 seconds. Every new start/work item advances the lifecycle generation and cancels the pending generation.
- The idle generation is checked before teardown. If new work intersects a teardown already being claimed, readiness retry observes the destroyed window and recreates it instead of accepting the stale engine.
- Cancellation, conversion/startup timeout, unexpected window destruction, app shutdown, plugin disable/removal/repair, and incompatible engine upgrade clear readiness/session/controller state and destroy the hidden surface through the host-owned destroy helper.
- The existing Rust worker removes the Zero-owned job directory after every provider result, including cancellation/failure. A later task reacquires and recreates the engine when readiness is absent.

## Browser-engine resource bounds

- PDF analysis acquires and cleans one `PDFPageProxy` at a time. Layout rendering reacquires one page at a time, keeps one canvas active, cancels the PDF.js render task on abort, shrinks the canvas to 1×1, and calls page cleanup in `finally`.
- DOCX image decoding is bounded to four workers. The rendered document DOM and export class are removed on every terminal path.
- Engine input and generated output `Uint8Array` references are zeroed after terminal IPC when their buffers remain attached.
- Existing native limits remain unchanged: 512 MiB input, 768 MiB output, 120-second job deadline, 2-second readiness deadline, bounded diagnostics/warnings/page geometry, raw-byte IPC, and sequential conversion.

## Executed checks

- `cargo test --manifest-path src-tauri/Cargo.toml services::file::runtime::tests::` — 7 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml services::file::engine_bridge::tests::` — 8 passed.
- `cargo check --manifest-path src-tauri/Cargo.toml` — passed.
- `pnpm build` — passed; entry 267.12 kB / 81.85 kB gzip, File engine 179.18 kB / 53.08 kB gzip, PDF-to-DOCX 795.34 kB / 233.30 kB gzip.
- `node --test tests/integration/sourceContracts/fileEnginePackaging.test.mjs` — 10 passed before the final inert-startup assertion was added; the complete integration suite is rerun by final verification.
- Refreshed test fixtures plus `node --test tests/unit/plugins/fileService.test.mjs` — 2 passed.

## Pending matched evidence

Task 8.1 remains open because no matched real-WebView pre/post engine startup, RSS, session/artifact count, or large-job settle samples were collected in this apply slice. Task 8.7 remains open for installed/signed package checks plus real hidden-WebView conversions, fidelity fixtures, cancellation, repeated batches, idle teardown observation, and post-settle memory.
