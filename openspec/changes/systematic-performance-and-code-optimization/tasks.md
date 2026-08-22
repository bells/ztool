## 1. Freeze Scope and Record Baselines

- [x] 1.1 Reconcile the requirements and unchecked tasks in `reorganize-project-modules`, `add-quick-launcher-plugin`, `optimize-screenshot-toolbar`, `add-file-plugin`, and `bundle-zero-file-offline-engines`; record overlaps and confirm this change will not archive or falsely complete their platform/manual gates.
- [x] 1.2 Add repository-owned performance result types and scripts that capture commit/dirty state, build mode, OS/build, architecture, hardware/memory, power/display context, warm-up, sample count, settle interval, commands, and raw samples without requiring a production runtime dependency.
- [x] 1.3 Configure Vite manifest output and add a build-graph report for initial entry, per-window/per-plugin chunks, gzip size, static imports, File-engine assets, and forbidden heavy dependencies.
- [x] 1.4 Add native/frontend readiness markers and a release-mode runtime collector for cold startup, warm tray/main/launcher reveal, first/repeat plugin activation, aggregate process-tree CPU/RSS, and media-transfer byte counts.
- [x] 1.5 Define ten-cycle reference workflows for Launch, Snap/pin, Awake, Paper, and File, including peak/post-settle RSS and owned resource counts; keep real GUI actions explicit when they cannot be automated safely.
- [ ] 1.6 Run the clean current production build and macOS reference protocol before runtime changes, save raw results plus the current 562.44 kB/166.92 kB gzip entry and 795.06 kB/233.22 kB gzip PDF chunk in `evidence/baseline.md`, and document the noise band and proposed deterministic budgets.
- [x] 1.7 Run the same baseline on a real Windows reference device or mark the Windows runtime baseline pending without treating macOS results or cross-target compilation as equivalent evidence.

## 2. Optimize Native Startup Critical Path

- [x] 2.1 Instrument monotonic phases for migration, managed-state construction, plugin-registry load/write, status-bar creation, Launch/File initialization, first frontend readiness, and settled hidden idle; add deterministic tests for marker ordering and missing/error outcomes.
- [x] 2.2 Add a versioned, recoverable completion marker or equivalent bounded metadata fast path to `migrate_default_home()` so normal startup does not recursively revisit legacy data, while retaining explicit recovery and migration tests.
- [x] 2.3 Refactor `PluginRegistry::load_or_seed()` to normalize in memory and persist atomically only after seeding, recovery, migration, or an effective state change; test unchanged startup performs no write or filesystem synchronization.
- [x] 2.4 Make managed-state constructors cheap and side-effect free by moving filesystem reads, provider probes, stale cleanup, full scans, and watcher startup into explicit joinable initialization owned by each service.
- [x] 2.5 Implement enabled Launch initialization as one single-flight cache-load/refresh/watcher lifecycle after core readiness or on first demand, and make concurrent panel, launcher-window, and command requests join the same result.
- [x] 2.6 Tie Launch initialization and refresh publication to plugin enablement generations; on disable, stop the application watcher, cancel or invalidate replaceable work, and prevent late results from repopulating disabled state.
- [x] 2.7 Construct one File provider registry and initialize provider discovery plus stale-artifact cleanup once before the first capability or conversion workflow; reuse the initialized state until explicit invalidation without prewarming the engine WebView.
- [x] 2.8 Gate File provider registration and construction with target-platform composition so macOS never probes Windows providers and Windows never probes macOS providers; retain structured unsupported-capability results.
- [ ] 2.9 Measure status-bar settings load, icon decode, and composite creation separately; add reusable decoded/composite caching only where matched evidence shows it improves startup without stale settings or unbounded native resources.
- [ ] 2.10 Add Rust unit/integration coverage for migration and registry no-op startup, Launch first-use races and disable teardown, File once-only/platform-gated initialization, and startup phase attribution; verify the cold/repeat path in a real release-mode macOS app and retain Windows runtime as a separate gate.

## 3. Split Window and Plugin Loading Boundaries

- [x] 3.1 Refactor `BundledPluginModule` so manifest/presentation/contributions remain lightweight synchronous metadata while each panel and dedicated surface is exposed through a typed dynamic loader owned by that plugin.
- [x] 3.2 Update all five bundled plugin descriptors and the single composition registry to use lazy panel/surface loaders without introducing core-to-plugin or sibling-plugin imports.
- [x] 3.3 Refactor `src/main.tsx` to resolve the Tauri window label before dynamically importing tray/main, preferences, about, capture, pin, launcher, paper, or `zero-file-engine` top-level code, preserving the safe unknown-label fallback.
- [x] 3.4 Render the selected tray/main plugin through a memoized `React.lazy` boundary with compact loading and import-error states that do not remount the shell or crash sibling navigation.
- [x] 3.5 Add unit and source-contract tests for lazy descriptor validation, one composition root, plugin removal/addition, unknown labels, import failure, and prohibited eager surface/engine dependencies.
- [x] 3.6 Build production assets and assert from the Vite manifest that tray/main startup cannot statically reach Capture, Pin, Paper, Launcher, File Engine, `pdfjs-dist`, `docx`, or `docx-preview`; record first-activation trade-offs and set reviewed chunk budgets.
- [ ] 3.7 Smoke-test every label-routed surface in the real macOS Tauri app, including repeated plugin switches and load fallbacks, while retaining Windows surface verification as a separate gate.

## 4. Add Explicit Surface Activity and Optimize Awake

- [x] 4.1 Define a symmetric surface-activity contract and centralize native show/hide/close notifications in host-owned window helpers without using focus as a visibility substitute.
- [x] 4.2 Implement a frontend surface-activity service/hook that combines the initial native visibility query, typed native events, and `document.visibilityState`, cleans up listeners, and exposes active/hidden/disposed state.
- [x] 4.3 Migrate disposable shell/plugin presentation work to the activity contract while proving accepted native jobs, downloads, watchers, and conversions continue when their UI is hidden.
- [x] 4.4 Update `useCaffeinePlugin` so its one-second display interval exists only while an Awake surface is active and enabled, and refresh the authoritative Rust snapshot before resuming after reveal.
- [x] 4.5 Add fake-clock/unit tests for inactive, enabled-visible, enabled-hidden, reveal, expiry-while-hidden, stale expiry, listener cleanup, and multiple Awake surfaces.
- [ ] 4.6 In a real macOS Tauri session, verify hidden Awake UI has no presentation timer activity while native no-limit/finite keep-awake and backend expiry remain correct; repeat the runtime behavior on Windows before claiming Windows completion.

## 5. Optimize Zero Launch Query and Icon Scheduling

- [x] 5.1 Extend Launch benchmarks to record frontend query count, IPC count, end-to-end p50/p95, index clone bytes or allocations, running-probe count, icon request concurrency, and React icon commit count for deterministic typing fixtures.
- [x] 5.2 Refactor the Rust index/search state to read a stable immutable snapshot without cloning all indexed items per query and without holding a shared lock during expensive matching or platform work.
- [x] 5.3 Move running-process probing fully outside the query path, give it a documented TTL and bounded refresh trigger, and publish a revision/event that lets visible Launch surfaces update without blocking input.
- [x] 5.4 Add measured query coalescing/debounce in `useQuickLauncher`, retain the generation guard, flush the latest query before Enter activation, and cancel/ignore pending presentation work when the surface becomes hidden.
- [x] 5.5 Add a bounded icon batch command or bounded-concurrency scheduler with per-icon and total cache byte/count limits, deterministic eviction, visible-result priority, and batched React state updates.
- [x] 5.6 Add Rust and TypeScript tests for fast typing, stale completion, Enter during debounce, hidden/reveal, stable revision activation, no query-path I/O/probe/icon work, icon overflow/eviction, superseded icon batches, and unsupported platforms.
- [x] 5.7 Run the existing 10,000-entry release benchmark and the new scheduling benchmark, preserving pure-search p95 below 5 ms and recording whether end-to-end p95/request-count budgets improved.
- [ ] 5.8 Manually verify macOS panel and launcher-window typing, Chinese/pinyin/initial/alias queries, keyboard activation, focus/launch, cache-first reopen, application changes, and icon fallback; keep the equivalent Windows `.lnk`/focus/settings/runtime matrix explicit.

## 6. Replace Zero Snap Base64 Media and Close Resource Leaks

- [x] 6.1 Define symmetric screenshot media descriptors, action/upload leases, raw-byte commands, structured errors, byte/dimension/type limits, session deadlines, and terminal cleanup rules without exposing caller-supplied resource paths.
- [x] 6.2 Refactor macOS capture to retain the native PNG in an owner-only session directory, return an opaque media token, serve it as a bounded raw `tauri::ipc::Response`, and clean expired session files safely on initialization.
- [x] 6.3 Add typed commit preparation plus bounded raw PNG upload using `tauri::ipc::Request`; validate token/session/window/action/deadline/signature/dimensions before native copy, save, or pin and revoke failed/expired leases idempotently.
- [x] 6.4 Update CaptureApp to read raw bytes, create/revoke Blob object URLs, clear decoded image references, shrink export canvases after use, and remove the `image_base64`/`png_base64` contracts and duplicate Base64 buffers.
- [x] 6.5 Refactor pin ownership so a live pin retains only its scoped file/token, `pin-*` initialization rejects stale or mismatched tokens, and window destruction removes the Rust map entry and owned file.
- [x] 6.6 Add Rust/TypeScript tests for raw request/response shape, traversal/token rejection, byte/type/dimension limits, copy/save/pin success, upload interruption, cancellation races, expiry, duplicate cleanup, pin destruction, and crash-leftover cleanup.
- [x] 6.7 Run ten automated resource-lifecycle cycles where possible and record token/file/map/canvas/object-URL terminal counts plus IPC byte reduction; do not treat simulated DOM cleanup as a native WebView memory result.
- [ ] 6.8 In real macOS Tauri windows, verify capture permission failure, Retina/multi-display selection, annotations, cropped copy/save, pin lifetime/close, cancel/failure recovery, ten-cycle RSS settle, and no title-bar/fullscreen regression.
- [ ] 6.9 Verify Windows still uses the system screenshot launcher with clear errors and no macOS-only media contract assumption; keep real Windows device behavior pending unless actually executed.

## 7. Bound Zero Paper Preview Work

- [ ] 7.1 Measure current Paper preview source dimensions, Base64/JSON bytes, decode latency, cache hits, concurrent duplicate requests, peak RSS, and ten-selection settle behavior using existing cached fixtures and a real Tauri WebView.
- [ ] 7.2 Extend the Paper cache index with a versioned bounded preview derivative owned beside the full-resolution file, choose dimensions/quality from visual and performance samples, and generate it atomically on a blocking worker.
  - Implementation uses provisional 960×600/JPEG-82 bounds and passes deterministic corruption/size tests; keep this open until real visual and decode/memory samples justify those constants.
- [x] 7.3 Replace preview data URLs with typed opaque descriptors plus bounded raw-byte reads, retain full-resolution files exclusively in native apply/save paths, and migrate Rust/TypeScript contracts atomically.
- [x] 7.4 Add a keyed single-flight preview-generation/read cache with count/byte/expiry limits and release semantics for obsolete callers and replaced frontend Blob/object URLs.
- [x] 7.5 Make the Paper controller activity-aware so import/hidden state does not initiate refresh/decode, while explicit download/apply/save work preserves its native lifecycle and stale preview completions cannot replace the current selection.
- [x] 7.6 Add parser/cache/service/controller tests for derivative corruption/rebuild, atomic writes, bounds, same-ID deduplication, late completion, hidden/reveal, object-URL cleanup, offline cache-first behavior, and full-resolution native apply/save.
- [ ] 7.7 Manually verify online/offline Paper navigation, preview quality, rapid selection, apply/save, cache recovery, two simultaneous visible surfaces, and ten-cycle RSS settle on macOS; repeat on Windows before claiming Windows runtime completion.

## 8. Bound Zero File Discovery and Engine Lifetime

- [ ] 8.1 Compare provider-registry construction count, panel-open capability cost, engine cold/warm startup, adjacent-job reuse, idle engine RSS, large-job peak/post-settle RSS, session count, and temporary artifact count against the pre-change baseline from group 1 and the startup-phase evidence from group 2.
- [x] 8.2 Add an explicit capability-cache generation and invalidation API for bundled engine install/upgrade/repair/removal and supported native-provider changes, preserving cached reads on normal panel open.
- [x] 8.3 Add a batch/session-aware File engine idle controller that retains one compatible WebView across adjacent sequential jobs, schedules teardown only when queue and sessions are empty, and rejects stale teardown generations after new work arrives.
- [x] 8.4 On idle timeout, crash, cancellation, or incompatible engine upgrade, destroy the hidden engine safely, clear readiness/session/controller state, remove owned staging artifacts, and prove a later job can recreate the engine.
- [x] 8.5 Audit PDF and DOCX engine paths for bounded page concurrency, Blob/ArrayBuffer/canvas/document cleanup, output/log/timeout limits, and cancellation checkpoints without increasing queue concurrency beyond one.
- [x] 8.6 Add Rust/TypeScript tests for cached capability reads, each invalidation cause, adjacent-job reuse, no startup prewarm, idle teardown, teardown/new-job race, crash/recreate, cancellation cleanup, and sequential queue invariants.
- [ ] 8.7 Run the File packaging/integrity checks and real hidden-WebView source smokes for both built-in directions, CJK and large fixtures, cancellation, repeated batches, idle teardown, and post-settle memory; preserve fidelity and platform claims from the active File changes.

## 9. Audit Rust Scheduling, State, and Code Quality

- [x] 9.1 Complete the remaining command and startup-hook inventory under `src-tauri/src/commands/`, `services/`, `bundled_plugins.rs`, and `lib.rs` after the group 2 startup work, recording blocking operations, caller thread, input/concurrency bounds, timeout/cancellation behavior, and lock scope without duplicating already verified startup findings.
- [x] 9.2 Move each verified UI-critical blocking filesystem/process/image/clipboard/platform operation behind a bounded blocking worker or owned executor and map task join failures into existing structured error contracts.
- [ ] 9.3 Shorten shared lock scopes, avoid blocking platform calls while locked, use complete immutable snapshot publication where it removes large clones, and add contention/race tests for every changed state owner.
- [x] 9.4 Add or tighten byte/count/age limits and deterministic eviction/cleanup for launcher icons, screenshot sessions/pins, Paper previews, File sessions/artifacts, diagnostics, and any cache found unbounded by the inventory.
- [x] 9.5 Remove unused imports, redundant state transitions, leftover debug logging, and unjustified `dead_code` allowances found in the audited paths; retain only narrowly documented platform/test conditional code.
- [x] 9.6 Run `cargo clippy --all-targets --all-features -- -D warnings`, fix the existing repo-wide findings or record a narrowly scoped platform/toolchain blocker without suppressing new warnings, and keep `cargo fmt --check` clean.

## 10. Final Regression and Performance Gates

- [x] 10.1 Run `pnpm test:unit`, `pnpm test:integration`, `pnpm test`, and `pnpm build`; verify recursive discovery, TypeScript strictness/no-`any`, lazy import boundaries, media contracts, and final chunk budgets.
- [x] 10.2 Run `cargo fmt --check`, `cargo check`, `cargo test`, the ignored release Launch benchmark, File engine/package verification, and `git diff --check` on a clean intended diff.
- [ ] 10.3 Run the complete matched macOS final protocol and write `evidence/final-verification.md` with raw samples, p50/p95, median/noise comparison, bundle graph, process-tree CPU/RSS, IPC bytes, and ten-cycle resource counts.
- [ ] 10.4 Confirm warm reveal p95 is at most 100 ms and hidden-idle RSS improves by at least 15% on the macOS reference environment, or leave the target incomplete until an explicit maintainer-approved revised budget and rationale are recorded.
- [ ] 10.5 Run the matched Windows release/device protocol and platform interaction smokes, or report each Windows runtime/performance item pending without promoting compile/CI results to device evidence.
- [ ] 10.6 Re-run the remaining manual smoke boundaries from every overlapping active change and update only evidence genuinely proven by this run; do not archive any change.
- [ ] 10.7 Run `openspec validate "systematic-performance-and-code-optimization" --type change --strict`, confirm `openspec status --change "systematic-performance-and-code-optimization"` is apply-complete, and review the final diff for accidental generated assets or unrelated edits.
