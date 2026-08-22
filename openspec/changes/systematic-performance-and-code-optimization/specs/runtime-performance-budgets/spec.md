## ADDED Requirements

### Requirement: Performance comparisons are reproducible
The project SHALL record performance baselines and final measurements from packaged or release-mode builds using a documented protocol that identifies the commit and dirty state, operating system and build, architecture, hardware and memory, power mode, display setup, warm-up, sample count, settle interval, commands, and raw samples.

#### Scenario: Baseline is captured before optimization
- **WHEN** implementation begins changing a measured runtime or bundle path
- **THEN** the corresponding baseline evidence exists and contains enough environment and protocol data for the same machine to repeat the measurement

#### Scenario: Final result is compared with baseline
- **WHEN** an optimization is presented as an improvement
- **THEN** its final measurement uses the same platform, reference environment, build mode, workflow, sample count, and aggregation method as its baseline or explicitly reports why the results are not comparable

### Requirement: The performance report covers the whole Zero runtime
The performance evidence SHALL report initial and per-surface bundle composition, cold startup, warm tray/main/launcher reveal, first and repeated plugin activation, hidden-idle aggregate process-tree CPU and RSS, large-media IPC bytes, and peak/post-settle memory for ten representative lifecycle repetitions of each affected tool.

#### Scenario: Release candidate evidence is reviewed
- **WHEN** the first-round optimization reaches final verification
- **THEN** the report contains raw samples plus p50 and p95 for latency and medians for hidden-idle CPU/RSS rather than a single observation

#### Scenario: Multi-process WebView runtime is measured
- **WHEN** Zero uses child WebView or helper processes on the measured platform
- **THEN** idle and workflow memory evidence aggregates the documented Zero process tree instead of reporting only the Rust parent process

### Requirement: Native startup phases are measured independently
The project SHALL record monotonic timing markers for migration, managed-state construction, plugin-registry load and persistence, status-bar creation, Zero Launch initialization, Zero File initialization, and first frontend readiness. Settled hidden-idle CPU and RSS SHALL be measured after the documented settle interval and MUST NOT be presented as part of frontend readiness latency.

#### Scenario: Cold startup evidence is captured
- **WHEN** the release-mode startup protocol runs
- **THEN** the evidence identifies the duration and outcome of every named native startup phase, first frontend readiness, and the later settled-idle sample

#### Scenario: Plugin initialization is deferred
- **WHEN** Launch or File initialization occurs after first frontend readiness or on first demand
- **THEN** the report attributes that work to its named phase without adding it to the core-readiness duration or omitting it from first-activation evidence

### Requirement: Repeated startup avoids unchanged persistence work
After migration has completed successfully, normal startup SHALL use a bounded completion fast path instead of recursively revisiting legacy data. Loading an unchanged plugin registry SHALL NOT rewrite or synchronize its backing file; persistence SHALL occur only when seeding, migration, recovery, or an effective registry change produces different durable state.

#### Scenario: Completed migration runs again
- **WHEN** Zero starts with a valid migration completion marker and no migration recovery condition
- **THEN** startup validates the marker through bounded metadata work and does not recursively scan or copy legacy application data

#### Scenario: Registry state is unchanged
- **WHEN** the persisted plugin registry is valid and normalization or default seeding produces no effective change
- **THEN** startup performs no registry write, temporary-file replacement, or filesystem synchronization

#### Scenario: Durable startup state changes
- **WHEN** registry seeding, migration, or recovery changes the durable state
- **THEN** Zero persists the new state atomically and records the write in the corresponding startup phase evidence

### Requirement: Numeric goals are honest release gates
The first-round optimization SHALL target warm window reveal p95 at or below 100 ms and at least 15% lower hidden-idle aggregate RSS than the recorded baseline on each declared reference platform. A missed or statistically unstable target MUST remain visible in the final evidence and MUST NOT be reported as achieved unless the maintainer explicitly accepts and records a revised budget with rationale.

#### Scenario: Both targets are met
- **WHEN** matched final samples show warm reveal p95 no greater than 100 ms and median hidden-idle RSS at least 15% below baseline outside the documented noise band
- **THEN** the report marks both first-round goals achieved for that reference platform

#### Scenario: A target is not met
- **WHEN** final samples miss a target or overlap the documented measurement-noise band
- **THEN** the change remains incomplete for that target or records an explicit maintainer-approved revised budget without manipulating samples, environments, or process scope

### Requirement: Deterministic regressions fail automated gates
The project SHALL provide deterministic automated checks for bundle import boundaries, chunk budgets, bounded fixture workloads, resource-count limits, and performance-sensitive pure functions, and SHALL fail the relevant check when a committed limit is exceeded.

#### Scenario: A heavy engine enters the shell graph
- **WHEN** the production build manifest shows the tray/main startup graph statically importing the File engine, PDF.js, `docx`, or `docx-preview`
- **THEN** the bundle regression check fails

#### Scenario: A budget change is intentional
- **WHEN** a maintainer accepts a new deterministic limit after reviewing measured evidence
- **THEN** the budget and rationale are updated together rather than silencing the tool warning or weakening discovery

### Requirement: Platform claims match executed evidence
The project SHALL report macOS and Windows performance and real-window validation independently, and MUST NOT use a browser test, another platform's runtime result, or cross-target compilation as proof of unexecuted device behavior.

#### Scenario: Only macOS runtime measurement ran
- **WHEN** automated checks and the macOS reference workflow pass but no Windows device workflow ran
- **THEN** the final evidence reports the Windows runtime gate as pending while retaining any valid Windows compile/test result separately
