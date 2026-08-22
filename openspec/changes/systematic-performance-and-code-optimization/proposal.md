## Why

Zero's feature set has grown from a small tray shell into five bundled tools and several dedicated WebViews, but performance work has remained feature-local: the current production build eagerly connects every bundled panel/surface to one 562.44 kB main JavaScript chunk (166.92 kB gzip), native startup performs repeated synchronous migration/registry/provider work and unconditionally starts plugin background services, large screenshots and wallpaper previews still cross IPC as Base64 strings, and there is no reproducible whole-app startup, warm-reveal, hidden-idle, memory, or bundle regression gate. A measured first optimization pass is needed now so future plugin growth does not silently increase launch latency, resident memory, background work, or native/WebView copies.

## What Changes

- Add a repeatable performance evidence workflow for macOS and Windows that records release-build bundle/chunk composition, process/WebView RSS, hidden-idle CPU, cold startup, warm window reveal, plugin activation, IPC payload size, and repeated-workflow memory recovery on documented reference environments.
- Establish baseline-derived budgets and regression gates instead of treating unmeasured goals as facts: retain the desired warm-reveal p95 of 100 ms and 15% resident-memory reduction as first-round targets, but require raw samples, measurement noise, comparison method, and an explicit decision when a target cannot be met honestly.
- Make the native startup critical path explicit and cheap: measure migration, managed-state construction, plugin-registry load/persist, status-bar creation, Launch initialization, File initialization, and first frontend readiness separately; add a versioned no-op path for completed migration, persist the plugin registry only when its content changes, and keep managed-state constructors free of avoidable disk/process discovery.
- Gate native plugin background work by enablement and demand: disabled plugins must not start watchers, full scans, provider discovery, or hidden engine WebViews; enabled Launch cache/scan/watcher initialization runs single-flight after the core is ready or on first use, while File provider discovery and stale-artifact cleanup occur lazily before the first File workflow.
- Split the React entry by Tauri window label and bundled-plugin surface, separate lightweight plugin metadata from render loaders, and load a plugin panel or dedicated surface only when selected; the Zero File engine and PDF/DOCX dependencies must not enter tray/main/launcher/capture/paper startup paths.
- Make WebView work visibility-aware: hidden windows suspend presentation-only clocks, animation frames, preview work, and other disposable UI activity while native state, expiry, conversion, and user-requested background operations remain correct.
- Optimize Zero Launch's existing Rust cache/search/watcher implementation rather than replacing it: coalesce input requests, prevent stale results, avoid per-query full-index cloning and repeated running-process probes, batch or lazily resolve visible icons, and preserve the existing 24-result bound and release search benchmark.
- Replace Zero Snap's full-resolution Base64 round trips and unbounded pinned-image retention with session-scoped native files or another bounded Tauri 2 resource handoff; release export canvases, object URLs/resource handles, Rust session bytes, and pin records at deterministic lifecycle boundaries while preserving the macOS editor and Windows system-capture paths.
- Keep Zero Awake's existing native keep-awake implementation and backend-owned expiry, but stop its one-second presentation clock while no caffeine UI is visible and resynchronize from the authoritative snapshot when a surface becomes visible again.
- Keep Zero Paper's existing cache-first, disk-backed, two-download pipeline, but serve a bounded thumbnail/preview through a local resource reference rather than a full-resolution Base64 snapshot, deduplicate preview requests, and release replaced preview resources.
- Preserve Zero File's intentionally sequential Rust-owned job queue and isolated blocking conversion worker; cache capability/provider discovery with explicit invalidation, avoid recreating the hidden engine WebView between compatible jobs, and enforce bounded cleanup for job artifacts and large page canvases. Do not introduce CPU-count concurrency that would violate Office automation and memory-safety constraints.
- Audit native commands and services for main-thread blocking, lock scope, redundant serialization, unbounded caches, process/resource cleanup, and duplicated state transitions; move verified blocking work behind bounded `spawn_blocking`/worker boundaries and enforce TypeScript/Rust warning gates without broad architecture rewrites or speculative dependencies.
- Preserve plugin independence, typed Rust/TypeScript IPC symmetry, offline/security guarantees, platform-specific behavior, and all unfinished manual acceptance boundaries in the related active OpenSpec changes.

## Capabilities

### New Capabilities

- `runtime-performance-budgets`: Defines reproducible baseline evidence, release-mode startup/reveal/idle/memory/bundle measurements, numeric budget ownership, and regression handling.
- `plugin-workload-efficiency`: Defines lazy plugin/surface activation and bounded, visibility-aware work for Zero Launch, Zero Snap, Zero Awake, Zero Paper, and Zero File.
- `native-media-lifecycle`: Defines path/resource-based transfer, ownership, cleanup, and repeated-workflow memory recovery for screenshots, pinned images, wallpaper previews, icons, and conversion canvases.

### Modified Capabilities

- `main-window-shell`: Requires Tauri window-label routing and plugin selection to load only the code needed by the active shell/surface while preserving safe fallback and existing window behavior.
- `caffeine-duration`: Requires presentation timers to follow surface visibility without changing backend-owned native keep-awake state or expiry correctness.

## Impact

- Frontend shell and composition: `src/main.tsx`, `src/App.tsx`, `src/appShell/`, bundled plugin descriptors/loaders, build configuration, and source-boundary tests.
- Plugin frontend code: Quick Launcher request/icon scheduling; Screenshot capture/export/pin lifecycle; Caffeine visibility-aware display timing; Bing preview loading; File panel/engine lifecycle and canvas cleanup.
- Rust/Tauri: `src-tauri/src/lib.rs`, `bundled_plugins.rs`, migration, plugin registry, status-bar startup, platform-gated provider composition, and thin commands/services for Quick Launcher, Screenshot, Bing Wallpaper, Caffeine, File, window lifecycle, managed caches, temporary-resource cleanup, and any typed IPC/event/resource-reference contracts changed by the measured design.
- Tooling and evidence: new benchmark scripts, release-mode fixtures, build-manifest/bundle assertions, platform measurement notes, repeated-workflow smokes, and CI-safe regression checks; real macOS and Windows runtime measurements remain separate platform gates.
- Related active changes: implementation must reconcile `reorganize-project-modules`, `add-quick-launcher-plugin`, `optimize-screenshot-toolbar`, `add-file-plugin`, and `bundle-zero-file-offline-engines` rather than duplicating, weakening, or prematurely archiving their remaining tasks.
- Dependencies: no new runtime library is assumed. Any virtual-list, debounce, media transport, or profiling dependency must first beat the simpler platform/native implementation on maintenance, bundle, security, and measured-value grounds.
