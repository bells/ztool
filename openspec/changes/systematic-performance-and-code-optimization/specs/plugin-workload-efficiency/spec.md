## ADDED Requirements

### Requirement: Bundled plugin work starts on demand
The system SHALL load and initialize a bundled plugin's panel or dedicated surface only when that plugin or surface is selected, shown, or explicitly prefetched after the current visible surface is interactive. Loading one plugin MUST NOT initialize a sibling plugin's UI, network refresh, preview, presentation timer, or engine WebView.

#### Scenario: Tray opens on one selected plugin
- **WHEN** the tray shell becomes visible with one bundled plugin selected
- **THEN** the shell and selected panel may initialize while unselected sibling panels and their dedicated surfaces remain unloaded and inactive

#### Scenario: User selects a different plugin
- **WHEN** the user selects a bundled plugin whose panel has not been loaded
- **THEN** the system shows a compact loading state, loads that panel once, and preserves shell interaction if loading succeeds or fails

### Requirement: Disabled plugins do not start native workloads
Native plugin services SHALL consult the persisted enablement state before starting plugin-owned watchers, full scans, provider discovery, stale-artifact cleanup, or hidden engine WebViews. A disabled plugin SHALL retain only cheap inert state needed for typed command registration and MUST NOT publish results from work that belonged to an earlier enabled generation.

#### Scenario: Zero starts with a plugin disabled
- **WHEN** Launch or File is disabled in the persisted plugin registry
- **THEN** Zero does not start that plugin's filesystem watcher, full application scan, provider discovery, stale-artifact cleanup, or hidden engine WebView during startup

#### Scenario: A plugin is disabled while work is active
- **WHEN** an enabled plugin is disabled while replaceable initialization or background refresh is running
- **THEN** the service advances its lifecycle generation, tears down owned watchers or disposable resources, and ignores any stale completion from the prior generation

### Requirement: Zero Launch initialization is single-flight and lifecycle-owned
Zero Launch SHALL initialize its cached index, full refresh, and application-directory watcher through one joinable single-flight operation after core readiness or on first Launch demand. Concurrent startup, panel, launcher-window, or command demand SHALL join the same operation, and disabling Launch SHALL stop its watcher and prevent stale index publication.

#### Scenario: Launch is enabled at startup
- **WHEN** core readiness completes and the configured startup policy initializes enabled Launch
- **THEN** cache loading, any required full refresh, and watcher startup occur once outside the core startup critical path

#### Scenario: First use races deferred initialization
- **WHEN** two Launch surfaces or commands request Launch before initialization completes
- **THEN** both callers join the same initialization result without duplicating cache loads, full scans, or filesystem watchers

#### Scenario: Launch is disabled after initialization
- **WHEN** the persisted enablement state changes from enabled to disabled
- **THEN** the application-directory watcher is stopped, replaceable refresh work is cancelled or invalidated, and late results cannot replace the disabled state

### Requirement: Zero File initializes discovery once for the current platform
Zero File SHALL create one provider registry and run provider discovery plus stale-artifact cleanup at most once before the first capability or conversion workflow for the active lifecycle generation. The registry SHALL construct only providers supported by the current target platform and SHALL reuse the same initialized state until an explicit capability invalidation or lifecycle reset.

#### Scenario: File is first opened
- **WHEN** the first File surface requests capabilities
- **THEN** the service creates one provider registry, performs one discovery and cleanup initialization, and shares the resulting snapshot with later panel opens

#### Scenario: File workflow races capability discovery
- **WHEN** a conversion command arrives while first capability discovery is still running
- **THEN** the command joins the same initialization instead of constructing another registry or probing providers again

#### Scenario: File initializes on a supported platform
- **WHEN** File initializes on macOS or Windows
- **THEN** it constructs and probes only providers compiled for that platform and does not execute discovery constructors for the other platform

### Requirement: Hidden surfaces suspend presentation-only work
The system SHALL expose an explicit active/hidden/disposed surface state, SHALL stop presentation-only intervals, animation frames, delayed previews, and disposable UI work while hidden, and SHALL resynchronize authoritative native state before resuming after reveal.

#### Scenario: A native window is hidden
- **WHEN** the host successfully hides a WebView window
- **THEN** its frontend receives or resolves hidden state and stops disposable presentation work without cancelling an accepted native job

#### Scenario: A hidden window is shown again
- **WHEN** the host shows a previously hidden surface
- **THEN** the surface refreshes authoritative state before restarting only the presentation work required for the visible UI

### Requirement: Zero Launch keeps input-path work bounded
Zero Launch SHALL coalesce superseded query requests, SHALL prevent stale results from replacing newer results, SHALL search a stable in-memory index without application-directory I/O, icon decoding, full-index cloning, or process probing on the query path, and SHALL return at most 24 results.

#### Scenario: User types several characters quickly
- **WHEN** multiple query changes occur before the configured coalescing interval completes
- **THEN** the system searches the latest query without allowing an older response to replace it and without issuing unbounded concurrent IPC requests

#### Scenario: User activates during a pending query
- **WHEN** the user presses Enter while a newer query is waiting to run
- **THEN** Zero Launch flushes or resolves the latest query before activating a stable item ID and revision

#### Scenario: Search benchmark runs
- **WHEN** the existing release benchmark searches at least 10,000 deterministic mixed-language entries
- **THEN** the pure matching phase retains its p95 below 5 ms and the added scheduling path reports its own request counts and latency separately

### Requirement: Zero Launch icon and running-state refreshes are bounded
Zero Launch SHALL resolve icons only for visible bounded results using a batched or bounded-concurrency scheduler, SHALL update UI icon state in bounded batches, and SHALL refresh application running state outside the query critical path with a documented cache lifetime.

#### Scenario: Search results change rapidly
- **WHEN** visible results are superseded before their icon work completes
- **THEN** obsolete icon results are ignored or cancelled and no more than the configured bounded work remains active

#### Scenario: Running-state cache expires
- **WHEN** the running-state lifetime expires while Launch remains active
- **THEN** the system refreshes the process snapshot outside search matching and publishes the new state without blocking typed input on a platform process scan

### Requirement: Zero Paper avoids duplicate hidden work
Zero Paper SHALL retain cache-first behavior, SHALL share concurrent refresh or preview generation for the same resource, and SHALL not start a remote refresh or preview decode merely because an unselected or hidden Paper panel was imported.

#### Scenario: Tray and Paper window request the same preview
- **WHEN** two visible Paper surfaces request the same cached wallpaper preview concurrently
- **THEN** the native service performs at most one preview-generation operation and both callers receive the same bounded result or structured error

#### Scenario: Paper surface becomes hidden
- **WHEN** a Paper surface is hidden during replaceable preview work
- **THEN** its obsolete UI request is cancelled or ignored while an explicitly requested native download/apply/save operation keeps its defined lifecycle

### Requirement: Zero File preserves bounded sequential execution
Zero File SHALL execute at most one conversion job at a time, SHALL reuse one compatible isolated engine WebView across adjacent jobs in an active batch, SHALL not prewarm that engine during Zero startup, and SHALL destroy it after a documented idle interval when no job or engine session remains.

#### Scenario: Multiple files are queued
- **WHEN** the user starts a batch containing multiple valid files
- **THEN** jobs run sequentially and the compatible engine may remain ready between adjacent jobs without creating one engine WebView per file

#### Scenario: File engine becomes idle
- **WHEN** the queue and session store remain empty for the configured idle interval
- **THEN** the hidden engine WebView is destroyed, its readiness state and disposable resources are cleared, and a future job can recreate it safely

#### Scenario: Provider availability changes
- **WHEN** a bundled engine is installed, upgraded, repaired, or removed or a supported platform provider changes
- **THEN** Zero File explicitly invalidates and rebuilds its cached capability snapshot without probing providers on every panel open

### Requirement: Native blocking work does not occupy UI-critical threads
The system SHALL execute potentially blocking filesystem traversal, process execution, image transformation, native automation, and platform probes on an owned blocking worker or bounded background executor, and MUST NOT hold shared application-state locks while performing such work.

#### Scenario: A blocking operation fails or panics
- **WHEN** a spawned blocking task cannot complete normally
- **THEN** its command or background workflow releases held resources and reports a bounded structured failure instead of hanging the UI or exposing a task-join error directly
