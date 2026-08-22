## Context

Zero is a tray-first Tauri 2 application with one shared React entry routing the tray, main, preferences, about, capture, pin, launcher, paper, and hidden File-engine WebViews by window label. Bundled plugins are structurally isolated, but their descriptors contain concrete React renderers and dedicated-surface components, so importing `src/appShell/bundledPluginModules.ts` also imports every bundled UI. `src/main.tsx` additionally imports `FileEngineApp` statically.

A production build at commit `5ad4a6d` on 2026-08-22 transformed 1,910 modules and produced:

| Asset | Minified | Gzip | Current meaning |
| --- | ---: | ---: | --- |
| `index-*.js` | 562.44 kB | 166.92 kB | Shared entry for every normal app/window surface |
| `pdfToDocx-*.js` | 795.06 kB | 233.22 kB | Dynamically imported only after a PDF-to-DOCX engine job |
| `index-*.css` | 54.80 kB | 10.79 kB | Shared styles |
| File engine installed assets | 4,780,883 bytes | 2,458,867 bytes | Versioned engine package prepared before the main build |

Vite reports a large-chunk warning. The dynamic `pdfToDocx` boundary works inside `FileEngineApp`, but the engine app itself and all other plugin surfaces still participate in the shared entry graph.

The native startup path also performs work before any frontend readiness marker exists. `run()` synchronously invokes the legacy-home migration, constructs the plugin registry, creates all bundled managed states, refreshes the native status bar, starts the Quick Launcher watcher plus a full background refresh, and initializes File conversion. The current details include:

- a persistent legacy root causes `migrate_legacy_data` to recursively inspect it on every launch because there is no versioned completion marker;
- `PluginRegistry::load_or_seed` reads and parses the registry, then saves and `sync_all`s it even when schema and records are unchanged;
- `QuickLauncherState::default` synchronously loads and parses index and usage JSON before Tauri setup, after which `start_quick_launcher` unconditionally registers a watcher and schedules a full refresh, including when the plugin is disabled;
- `FileConversionState::default` constructs a provider registry, `initialize_with_engine` replaces it with a second registry, and `default_provider_registry` instantiates both macOS and Windows Word providers, whose defaults perform platform discovery even on the inapplicable platform;
- `refresh_status_bar` is required for tray availability but also loads settings and decodes/composes embedded icons synchronously, so its measured cost and cache behavior belong to the startup budget.

The local CodeGraph 1.0.1 index covered 255 files, 3,631 symbols, and 11,119 edges with no parse errors or unresolved references. It is used as an impact/navigation aid, then checked against live source: Rust macros and chained `.manage(...)` calls are not fully connected, same-named methods can be misresolved (for example a Launcher `refresh` edge was attributed to Paper), and `codegraph affected` did not discover the repository's source-contract tests. CodeGraph output therefore cannot by itself prove a call target or safely narrow verification scope.

The plugin implementations already contain important performance mechanisms that this change must preserve:

- Launch has a versioned disk cache, in-memory Nucleo search, a filesystem watcher, `spawn_blocking` refresh, a 24-result bound, a 128-entry icon cache, and a release benchmark for 10,000 fixtures. Its current query path still clones the full index, can refresh running-process state, invokes on every input change, and resolves icons through individual calls/state writes.
- Snap's macOS editor needs the captured pixels in the WebView, but Rust currently reads the native temporary PNG into Base64, stores it in the session, sends it through JSON IPC, and receives another Base64 PNG on copy/save/pin. Pin data is retained in a Rust map without a window-destroy cleanup path.
- Awake already activates native keep-awake only while enabled and owns expiry in Rust. Its React hook runs a one-second display timer whenever enabled, including while the containing window is hidden.
- Paper already uses cache-first loading, atomic disk files, single-flight refresh, and two concurrent downloads. A selected full-resolution image is nevertheless read, Base64-encoded, serialized, and retained as a data URL for preview.
- File already uses a sequential queue, a cached provider registry, blocking-worker isolation, raw binary Tauri IPC for engine input/output, per-page canvas shrinking, and reuse of a compatible hidden engine WebView. The reusable engine currently has no idle teardown policy, and capability invalidation is not an explicit public lifecycle.

Several related OpenSpec changes remain active because of real macOS/Windows smoke or release-evidence tasks. This change layers measurable performance requirements over their implemented behavior; it does not rewrite their history or mark their platform gates complete.

## Goals / Non-Goals

**Goals:**

- Establish reproducible, versioned performance evidence before optimizing and compare final results under the same environment and protocol.
- Shorten the native process-to-ready path by making repeated migrations/persistence no-ops, keeping state construction cheap, and deferring plugin-specific discovery/background work until core readiness or first demand.
- Reduce code and resource work on the common tray/main startup and warm-show paths by loading only the active window surface and selected plugin UI.
- Bound large-media copies and lifetime across Rust, IPC, and WebViews.
- Remove avoidable query-path, hidden-window, preview, and engine-idle work without changing feature semantics.
- Keep native commands thin, blocking work off the event-loop/main thread, lock scopes explicit, and Rust/TypeScript contracts symmetric.
- Add automated structural and deterministic performance gates, plus explicit real macOS and Windows runtime gates where automation cannot prove behavior.

**Non-Goals:**

- Replacing React, Tauri, the bundled-plugin model, Nucleo, the File engine stack, native capture, or platform keep-awake implementations.
- Introducing a global state library, a general streaming framework, a virtual-list dependency for a list already bounded to 24 rows, or speculative micro-optimizations without measured evidence.
- Running File conversions at `CPU cores - 1`; the sequential queue remains intentional because Office automation, WebView rendering, and large documents can contend for memory and provider-global state.
- Claiming a universal 15-30% memory improvement or zero CPU usage across machines before collecting stable baselines.
- Treating browser tests, macOS runs, or cross-target compilation as proof of Windows runtime behavior.
- Archiving any OpenSpec change as part of this work.

## Decisions

### 1. Performance evidence is a versioned release artifact, not an ad-hoc screenshot

Add a small repository-owned harness with machine-readable output and two durable reports under this change's `evidence/` directory. Every run records commit, dirty state, release/debug mode, OS/build, architecture, hardware, memory, power mode, display setup, sample count, warm-up, settle interval, command, and raw samples.

The protocol measures:

- build graph: initial entry, per-surface and per-plugin chunks, gzip sizes, and forbidden eager dependencies from the Vite manifest;
- cold process-to-ready time for a packaged/release app;
- warm tray/main/launcher reveal from native show request to a frontend `requestAnimationFrame` readiness acknowledgement;
- selected-plugin first activation and repeat activation;
- aggregate Zero process-tree RSS and CPU after a 60-second hidden-idle settle;
- byte counts and copy stages for screenshot and wallpaper media handoffs;
- memory before, peak, and after settle for ten Snap, Paper, Awake, Launch, and File lifecycle repetitions appropriate to each tool.

Use at least 10 cold samples and 30 warm samples, report p50/p95 for latency, and use medians for idle CPU/RSS. Runtime comparisons use the same machine, OS, power state, packaged/release build, and workflow. CI enforces deterministic build/fixture budgets; real-device reports own RSS/CPU/window timings because shared CI hosts are too noisy.

The desired first-round outcomes remain warm-reveal p95 at or below 100 ms and at least 15% lower hidden-idle process-tree RSS from the recorded baseline. If noise, platform runtime, or fixed WebView cost prevents either target, the final report must show the samples and the change remains incomplete until the maintainer explicitly accepts a narrower budget. Percentages are never inferred from one sample or different environments.

Alternative: commit fixed absolute RAM numbers immediately. Rejected because WebKit/WebView2 process models, OS versions, architecture, and loaded plugins materially change the baseline.

### 2. Keep native startup cheap, single-flight, and plugin-gated

Instrument process entry through first frontend readiness as named phases: legacy migration, managed-state construction, plugin-registry load/persist, Tauri setup, status-bar creation, Launcher cache/scan/watcher initialization, File initialization, WebView creation, and frontend ready acknowledgement. The release evidence reports each phase so moving work after the ready marker cannot disguise a CPU/RSS spike; time-to-settled-idle remains a separate measurement.

`migrate_default_home` remains before registry loading because compatibility must be resolved before canonical data is consumed. Add a versioned completion marker only after the migration finishes without a blocking diagnostic. A matching marker and migration schema take a constant-time fast path on later launches; missing, corrupt, older, or incomplete markers rerun the safe idempotent migration. The marker never causes deletion of the legacy tree.

`PluginRegistryState` remains available during setup because plugin enablement and status-bar contributions depend on it, but `load_or_seed` tracks whether seeding, canonicalization, bundled-record migration, or schema migration actually changed the disk model. It writes and syncs only after a real change. A normal unchanged launch performs one bounded read/parse and no registry write.

Bundled service `Default` implementations allocate only cheap in-memory state. In particular:

- Launch cache/usage loading moves to an idempotent single-flight initializer on a blocking worker. When Launch is enabled, initialization begins after the minimum native shell is ready or at first Launcher use, whichever comes first; a first-use race joins the same initialization and can show a truthful loading snapshot. Its watcher starts only after initialization and is dropped when the plugin is disabled. Full refresh remains coalesced and must not delay core readiness.
- File state starts without constructing compatibility providers twice. Stale-job cleanup, built-in engine binding, and provider discovery initialize once before the first File capability/workflow request. Provider composition is platform-gated so macOS does not detect Windows Word/PowerShell and Windows does not inspect macOS Word. Install/upgrade/repair/removal invalidates the initialized generation rather than rebuilding it on panel open.
- Paper, Snap, and Awake keep cheap managed state and do not gain eager work.

The status bar is part of the minimum tray shell, so it is not deferred wholesale. Measure its settings load, icon decode/composition, menu construction, and native item creation separately; cache immutable decoded embedded icons and keyed composites only where the baseline proves repeated decode/composition material. Correct enabled contributions and Awake state take precedence over reusing a stale composite.

Plugin enable/disable transitions own native lifecycle. Enabling a plugin permits its deferred initializer; disabling Launch stops its watcher and prevents refresh publication, and disabling any plugin prevents new plugin-owned background work without cancelling an already accepted operation whose existing contract requires completion or explicit cancellation.

Alternative: move all persistent state loading after the first WebView frame. Rejected because migration and plugin enablement are prerequisites for correct registry, shortcut, and status-bar behavior. Alternative: leave all initialization unconditional but wrap it in `spawn_blocking`. Rejected because disabled plugins would still consume I/O, CPU, watcher handles, and memory after the ready marker.

### 3. Split lightweight plugin metadata from lazy render loaders

Keep each plugin independently pluggable, but change its descriptor so manifest, contributions, presentation, kind, and accent metadata are synchronously cheap while UI code is behind typed loaders. The composition root remains the only bundled-plugin registry.

Conceptually:

```ts
interface BundledPluginModule {
  manifest: PluginManifest;
  presentation: Record<ResolvedLanguage, PluginPresentation>;
  loadPanel: () => Promise<{ default: ComponentType<PanelProps> }>;
  surfaces?: Partial<Record<BundledPluginSurface, () => Promise<{ default: ComponentType }>>>;
}
```

`src/main.tsx` first resolves the Tauri label, then imports only the corresponding top-level app. Tray/main load the shell and the selected plugin panel through `React.lazy`/`Suspense`; capture, pin, launcher, paper, preferences, about, and `zero-file-engine` load direct dedicated chunks. Lightweight manifest registration must not evaluate a panel, capture editor, wallpaper controller, launcher view, or File engine.

The build test parses Vite's manifest/import graph. In particular, tray/main startup must not statically reach `CaptureApp`, `PinApp`, `PaperApp`, `QuickLauncherApp`, `FileEngineApp`, `pdfjs-dist`, `docx`, or `docx-preview`. Loading and error fallbacks stay compact and recoverable, and an import failure must not crash the entire shell.

Alternative: only configure `manualChunks`. Rejected because it can create separate files while eager imports still download/evaluate them. The ownership boundary must be dynamic import, with `manualChunks` used only if measurement later shows stable vendor grouping is beneficial.

### 4. Window activity is an explicit host lifecycle contract

Create a host-owned surface-activity service/hook. Native window helpers emit a typed visibility event after successful show/hide/close transitions; the frontend combines that event with an initial `isVisible()` read and `document.visibilityState`. Consumers receive `active`, `hidden`, or `disposed` rather than inferring lifecycle from focus alone.

When hidden, a surface cancels presentation-only intervals, animation frames, delayed preview requests, and disposable UI subscriptions. When shown, it rereads authoritative native state before resuming. Native caffeine expiry, an accepted File conversion, filesystem watching, and a user-requested Paper download continue because they are not presentation work. Blur alone does not suspend a visible launcher or editor.

Alternative: rely only on the Page Visibility API. Rejected until verified on WKWebView and WebView2 because a natively hidden WebView is not guaranteed to produce identical browser events across platforms.

### 5. Large native media uses scoped raw resources, not Base64 JSON

Reuse the raw Tauri IPC pattern already proven by Zero File (`tauri::ipc::Response` for bytes and `tauri::ipc::Request` for bounded upload bodies), but put a small typed control contract around it.

Snap keeps the native PNG in an owner-only session directory and returns metadata plus an opaque, session-bound media token. The WebView reads the bytes once, creates a Blob/object URL, and revokes it on replacement, cancel, commit, or unmount. A commit first creates a typed action/upload lease, then sends bounded raw PNG bytes under that lease. Rust validates session, MIME/signature, byte limit, dimensions, action, deadline, and optional save destination before copy/save/pin. Pin windows read from their own opaque lease or retained session file; destroying a pin window removes its Rust record and owned file. Startup/session cleanup removes expired artifacts.

Paper creates and atomically caches a UI-sized preview derivative when an image is downloaded or first requested. Snapshots contain metadata and cache state only; preview returns an opaque media descriptor and raw bytes rather than a full-resolution data URL. Concurrent requests for the same wallpaper share one generation, and replaced tokens/object URLs are released. Applying or saving continues to use the validated full-resolution native cache file without sending it to React.

Small launcher icons remain bounded IPC payloads in the first round, but requests are batched for visible results and the cache has byte/count limits. File continues using its existing raw binary engine bridge; it is the reference contract, not a new generic permission for third-party plugins.

Alternative: expose broad temporary directories through the asset protocol. Rejected because scope drift could reveal unrelated files. Alternative: Base64 inside Tauri events. Rejected because it retains the JSON serialization and roughly 33% encoding expansion.

### 6. Each plugin receives only evidence-backed scheduling changes

**Zero Launch**

- Debounce/coalesce query dispatch over a short measured interval while preserving immediate keyboard feedback and flushing the latest query before Enter activation.
- Keep the existing stale-generation guard. A search request reads a stable index snapshot without cloning every item and never probes processes, reads icons, or touches disk.
- Refresh running-state snapshots outside the query critical path with a bounded TTL and publish a revision/event when results need repainting.
- Fetch visible application icons in one bounded batch or a bounded-concurrency scheduler, update React state once per batch, and cancel/ignore work when hidden or superseded.
- Retain 24 results; add virtualization only if a later change raises the bound and profiling proves DOM cost.

**Zero Awake**

- Rust remains the owner of native assertion/process state and finite-session expiry.
- The one-second React display clock runs only while an Awake panel is visible and enabled. On reveal it fetches one fresh snapshot before showing elapsed/remaining values.
- Inactive mode has no recurring frontend or backend poll.

**Zero File**

- Preserve the existing sequential queue, cached provider registry, raw binary bridge, compatible engine-window reuse, cancellation, timeouts, and per-page canvas shrink.
- Add explicit provider-cache invalidation for plugin engine install/upgrade/repair and relevant platform-provider changes instead of probing on every panel open.
- Retain a compatible engine WebView across adjacent queued jobs, then destroy it after a measured idle timeout when there are no sessions/jobs; cancellation or crash revokes sessions and removes temporary artifacts. A later job recreates it safely.
- Do not prewarm the engine during Zero startup. Peak and post-settle memory for representative large jobs are recorded separately from idle tray RSS.

Snap and Paper scheduling follows Decision 4. All five plugins use events only where state changes are discrete; a timer is retained when it represents visible elapsed time. “Replace all polling” is not a design rule by itself.

### 7. Rust audits distinguish blocking work, lock contention, and harmless synchronous state

Inventory every registered command and startup hook. For each file/network/process/image/clipboard/platform call, record whether it can block, its caller thread, input bound, timeout/cancellation behavior, and lock scope. Move genuinely blocking work to `tauri::async_runtime::spawn_blocking` or an owned worker, but do not wrap cheap mutex-protected snapshots in tasks just to make them syntactically async.

No blocking platform call may run while holding a shared state lock. Cache publication uses short lock windows and whole immutable snapshots (`Arc` where it avoids large clones). Task joins map panic/cancellation to structured errors. Background jobs have explicit concurrency, queue, byte, timeout, and teardown bounds.

Alternative: convert every command to `async fn`. Rejected because an async signature does not make blocking filesystem/process work non-blocking and can obscure the real boundary.

### 8. Code-quality cleanup is scoped by evidence and enforced by gates

Keep TypeScript strict/no-`any` and the existing module-boundary tests. Add source-contract tests for lazy loader direction and large-media contracts. Run formatter, TypeScript build, complete Node suites, Rust tests, and Clippy with warnings denied. Remove unused imports, debug logging, redundant transitions, and unjustified dead-code allowances found by these gates; platform/test-only code may use a narrowly documented conditional allowance when compilation proves it is necessary.

Do not perform unrelated naming, styling, state-library, or directory refactors during a performance task. A code cleanup must either simplify a measured hot/lifecycle path or satisfy an explicit quality gate.

### 9. Active OpenSpec work is a sequencing constraint

Before editing a plugin, reconcile its active spec and remaining tasks. This change may reuse their tests and evidence, but does not check their manual items merely because performance automation passes. In particular:

- module lazy loaders must preserve the pluggability/source-boundary contract;
- Snap optimization must preserve the pending real macOS overlay/editor smoke and Windows system path;
- Launch optimization must preserve unfinished watcher, cache, IPC, macOS, and Windows validation;
- File optimization follows the offline-engine package/runtime contracts and does not claim Windows DOCX-to-PDF;
- no related change is archived automatically.

Implementation is staged plugin-by-plugin so a regression can be reverted independently. IPC changes are migrated atomically across Rust commands, TypeScript interfaces/services, capabilities, and tests; no compatibility shim keeps duplicate Base64 and raw-media paths after the new path is verified.

## Risks / Trade-offs

- [Dynamic imports make the first visit to a plugin slower] → Measure first activation separately, provide compact Suspense feedback, and optionally prefetch only the next likely plugin after the active shell is interactive and visible.
- [Deferring native plugin initialization makes first use slower] → Start enabled Launch initialization after minimum core readiness, make first use join the same single-flight state, measure both cold startup and first activation, and keep File truthful while provider discovery is pending.
- [A migration marker incorrectly skips required compatibility work] → Version the marker, write it only after a complete idempotent migration, rerun on missing/corrupt/older markers, and retain migration fixtures for interrupted and upgraded states.
- [Stopping a watcher on disable races with an in-flight refresh] → Bind publication to an enablement/lifecycle generation, drop the watcher handle, and ignore results from stale generations without corrupting the last valid cache.
- [A hidden-window event is missed] → Combine native events with an initial/current visibility query and resynchronize authoritative state on every reveal.
- [Raw-media tokens create a file-read capability] → Use random opaque tokens bound to one plugin/session/window, fixed roots, byte/type limits, expiry, one-purpose leases, and cleanup on terminal/window events; never accept a caller path.
- [Screenshot upload becomes a two-phase failure surface] → Make leases short-lived and idempotently revocable, keep the old session valid until commit succeeds, and cover prepare/upload/cancel/timeout races.
- [Thumbnail generation adds CPU/disk work] → Generate once in a blocking worker, cap dimensions/quality/count, atomically cache it, and compare its cost against current Base64 encoding in the evidence report.
- [Launcher debounce harms perceived responsiveness] → Tune against end-to-end p95, flush on activation, and retain stale-request protection; remove the debounce if batching/backend changes make it unnecessary.
- [Destroying the File engine after idle increases the next conversion's latency] → Use a measured retention timeout, retain across a running batch, and report both warm-job and post-idle behavior.
- [A fixed 15% RSS goal is dominated by WebView runtime variance] → Aggregate the full process tree, use repeated medians and noise bands, keep the goal visible, and require explicit maintainer acceptance for any revised budget.
- [Cross-platform results diverge] → Keep macOS and Windows evidence separate and block only the platform claim whose real-device gate failed.
- [Broad cleanup destabilizes feature work] → Limit changes to audited paths, preserve structural tests, and use small plugin-scoped task groups with rollback points.

## Migration Plan

1. Record clean-build baseline evidence and freeze the measurement protocol before changing runtime code.
2. Add native startup phase instrumentation, then implement migration/registry no-op paths, cheap state construction, plugin-gated single-flight initialization, platform-gated File providers, and measured status-bar caching.
3. Introduce bundle performance scripts, Vite manifest output, and non-regression tests without changing product behavior.
4. Split metadata/render loaders and window-label entry chunks; verify all existing surfaces before plugin-specific work.
5. Add the surface-activity contract, then migrate Awake and disposable UI work.
6. Implement Snap/Paper raw-media lifecycle behind new symmetric contracts and remove Base64 fields after automated and real-window verification.
7. Optimize Launch query/running/icon scheduling against the existing release benchmark and add end-to-end request evidence.
8. Add File provider invalidation and batch-aware engine idle teardown without changing queue concurrency or quality/provider selection.
9. Complete the remaining Rust blocking/lock/cache audit, warning cleanup, full automated gates, and matched final measurements on macOS and Windows.

Rollback is staged: the old migration algorithm remains the fallback when its marker is absent; registry no-op persistence can return to the last verified save behavior; deferred plugin initializers can temporarily run after setup without reintroducing cross-platform providers; lazy loaders can return to the last verified eager descriptor without changing manifests; activity consumers can resume their former visible timers; media contracts roll back as one Rust/TypeScript pair while temporary files remain cleanup-safe; Launch scheduling and File idle teardown can be disabled independently. Baseline/final evidence and measurement scripts remain useful even if an optimization is reverted.

## Open Questions

- The exact packaged-app process-tree collection commands and stable noise threshold for the Windows reference device must be fixed in `evidence/baseline.md` before numeric Windows comparison; this does not change the architecture.
- The post-core-ready delay for proactive Launch initialization and any status-bar composite cache budget will be selected from named startup/settled-idle phase samples; first use remains the deterministic fallback.
- The initial File engine idle timeout and Paper preview dimensions will be chosen from baseline memory/latency and visual-quality samples, then recorded as explicit constants and budgets rather than hidden magic numbers.
