# Rust scheduling, state, and cache audit

Date: 2026-08-22

Scope: the 70 commands registered in `src-tauri/src/lib.rs`, setup/shortcut/exit hooks, `src-tauri/src/bundled_plugins.rs`, command modules, and their direct services. The refreshed local CodeGraph index (255 files, 3,631 nodes, 11,119 edges) was used only for navigation; every finding below was confirmed in live source because Rust macros and Tauri state composition are not completely represented by the graph.

## Startup hooks

| Hook | Caller and work | Bound / scheduling decision |
| --- | --- | --- |
| `run()` migration | Process startup before Tauri event processing; bounded marker fast path, otherwise compatibility filesystem migration | Remains synchronous because canonical data must exist before registry construction; versioned completion marker makes the normal path bounded. |
| Managed-state and registry construction | Process startup; cheap plugin state plus one bounded registry read/normalize | Constructors are side-effect free except the required registry load; unchanged registry performs no write. |
| Tauri setup / status bar | Tauri setup callback; settings read, icon decode/composition, native tray construction | Remains on setup because the tray is Zero's minimum shell. Named sub-phases expose its cost; task 2.9 remains open until matched samples justify a cache. |
| Enabled Launch initialization | Post-core async task | Full cache load/scan/watcher work runs single-flight in `spawn_blocking`; disabled generations do not publish. |
| Global shortcuts | Native callback | Window-only Launch dispatch stays direct; Snap capture/process/image work is handed to `spawn_blocking`. |
| Debug File smoke | Debug-only, environment-gated | Not a release path; the conversion worker remains async/sequential and smoke diagnostics are intentionally prefixed. |
| Exit cleanup | Tauri exit callback | Bounded owned-session cleanup only; no arbitrary paths are accepted. |

## Registered command inventory

| Command family | Blocking work and input bounds | Worker / lock outcome |
| --- | --- | --- |
| App/surface/window (11) | Tauri window lifecycle, small validated performance fields (plugin id <=128 bytes, duration <=10 minutes) | Direct host operations; no filesystem traversal, process, image, or clipboard work. |
| Plugin market/lifecycle (9) | HTTPS is async and bounded by existing native-resource limits; package validation/install/remove and registry persistence touch disk | File/package/status-bar lifecycle runs on the blocking pool and maps join failure to a stable command error. Small cached list snapshots remain direct. Registry mutation retains its exclusive transactional lock; further lock-architecture work is tracked by open task 9.3. |
| Status bar (4) | Settings <= bounded JSON model, embedded icon decode/composition, native tray updates | All frontend-invoked operations use one blocking worker helper. Required startup creation remains synchronous and separately instrumented. Settings persistence occurs after cloning state and outside the settings mutex. |
| Shortcuts/Awake (3) | Shortcut snapshot is in-memory; Awake can spawn/stop the native keep-awake process and rebuild status items | Snapshot remains direct. Toggle plus status rebuild runs in `spawn_blocking`; expiry is backend-owned on a joinable generation and does not poll. |
| File panel/queue (13) | Picker, path canonicalization/metadata, max 100 enqueue items, sequential queue, provider/process/output actions with existing time/byte limits | Capabilities, picker, inspect, enqueue, and result process actions run on blocking workers. Enqueue serializes reservation work with a dedicated gate, snapshots runtime state briefly, performs filesystem inspection without the runtime mutex, then publishes atomically. |
| File engine bridge (6) | One engine window/session; input <=512 MiB, output <=768 MiB, protocol/job/version/deadline binding | Ready asset validation and raw input/output filesystem I/O run in blocking workers. Runtime mutex is released before read/write. Progress/completion messages only validate bounded fields and send to the owned session. |
| Paper (7) | Cache index <=1 MiB, image <=25 MiB, preview <=2 MiB; refresh/download count already bounded | Network stays async. Preview reads, image decode/validation/derivative generation, apply, and save filesystem/platform work run in blocking workers with structured join errors. Per-ID generation, refresh, lease count/bytes, and history remain bounded. |
| Launch (9) | Query <=128 chars and 24 visible results; icon batch 24, item 512 KiB, cache 128 entries/8 MiB; usage 1,000 records | Initialization/search/platform/icon/activation operations use the blocking pool. Search reads immutable snapshots. Usage I/O is serialized by a dedicated gate and occurs outside the index `RwLock`; diagnostics are capped at 64 entries and 512 bytes each. |
| Snap (8) | One active capture; PNG <=100 MiB, dimension <=32,768, pixels <=268,435,456; four upload leases and 16 live pins | Capture/process/image, raw read, commit/clipboard/save/pin, and cancellation cleanup run in blocking workers. Raw request size is rejected before cloning. Store locks are released before file removal, image validation, clipboard, and window creation. |

## Lock and cache findings

- Launch no longer holds the index `RwLock` while writing and syncing usage JSON. Concurrent activation persistence is serialized by an operation gate, then a complete `Arc<UsageMap>` is published under a short write lock.
- File enqueue no longer holds `RuntimeState` while canonicalizing input or reserving output names on disk. Concurrent enqueues use a dedicated gate and publish queue/reservation state in one short runtime lock scope.
- File raw engine reads/writes validate and clone the scoped path while locked, release the mutex, and then perform filesystem I/O.
- Snap has one active session, four expiring upload leases, and now at most 16 live pins. Reaching the pin bound returns `screenshot.pin_limit` instead of evicting a visible window or growing indefinitely.
- Paper preview leases remain limited to 16 entries/8 MiB; preview-generation locks are now limited to the ten-item history bound and evict only idle entries.
- Launch icons remain limited to 128 entries/8 MiB with deterministic LRU-key eviction; diagnostics now have deterministic count and UTF-8-safe byte bounds.
- File queue/session/artifact state remains sequential, job batches are capped at 100, engine sessions are deadline scoped, diagnostics are string-bounded, and owned stale artifacts are cleaned on initialization, cancellation, crash/reset, idle teardown, and exit.

## Quality gate and intentional diagnostics

`cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check` pass after removing the reported redundant folds/comparisons, unnecessary returns/conversions, misplaced test module, legacy C-string construction, and test-only boolean assertions. No `dead_code` allowance, `dbg!`, `println!`, frontend `console.log`, or `console.debug` remains in the audited production paths. The remaining `eprintln!` calls are intentionally scoped to `ZERO_PERF`, environment-gated `ZERO_FILE_ENGINE_SMOKE`, or actionable startup migration diagnostics.

## Verification boundary

Focused unit tests cover diagnostic byte/count limits, live-pin rejection, preview-lock saturation/idle eviction, and concurrent File enqueues. These checks prove deterministic scheduling/state behavior, not AppKit, WKWebView, WebView2, native clipboard, Office automation, or real release-mode CPU/RSS behavior. Those platform gates remain open.
