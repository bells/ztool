# Runtime performance instrumentation

Date: 2026-08-22

This evidence covers the instrumentation and collector required by task 1.4. It does not provide the missing pre-change release baseline or matched final runtime samples; tasks 1.6, 10.3, and 10.4 remain open.

## Markers

- Native startup events use one monotonic origin and retain sequence, phase, outcome, start offset, and duration. First frontend readiness records process-origin-to-next-frame duration for the actual calling window.
- Every host-owned `show_surface` begins a label-scoped pending `surface_reveal:*` marker. The current WebView listens for the typed active event and acknowledges on the next animation frame; failed native show/event publication cancels the pending marker, and duplicate/late acknowledgements are ignored.
- A lazy bundled panel records activation only after its component has resolved Suspense and mounted. Each WebView distinguishes the first activation of a plugin from a later remount and reports a bounded duration through a typed command; the frontend cannot choose an arbitrary native phase name.
- Snap screenshot reads/uploads, Paper preview reads, and File engine reads/writes publish accepted raw byte counts as `media_transfer:*` measurements. The runtime event schema keeps measurement value/unit separate from latency duration.

## Collector

- `pnpm performance:runtime -- --input <ZERO_PERF-log> --process-samples <json> --output <result>` consumes release-mode structured events and documented process-tree samples.
- Latency phases retain raw samples and report min/median/p95/max. Byte measurements retain raw values and totals by channel. Aggregate process-tree RSS and CPU retain their own raw samples and summaries.
- Coverage flags independently report cold startup, warm reveal, first plugin activation, repeat plugin activation, media bytes, and process-tree CPU/RSS. Missing runtime evidence remains visible rather than becoming a zero sample.
- Run metadata records commit/dirty state, build mode, OS/build, architecture, hardware/memory, power/display context, command, warm-up count, sample count, and settle interval through the shared performance contract.

## Deterministic checks

- Rust performance tests cover monotonic ordering, error outcomes, pending reveal single-consumption/missing acknowledgements, and byte value/unit retention.
- Runtime collector unit tests cover structured-log parsing, latency summaries, media byte totals, and all six coverage categories.
- Source-contract tests bind native reveal start to next-frame frontend acknowledgement, activation to a resolved lazy panel, and media/process fields to the collector.
- `cargo check --manifest-path src-tauri/Cargo.toml` and `pnpm build` passed after instrumentation. The production initial entry remained inside its reviewed budget at 267.79 kB / 82.17 kB gzip.

## Runtime boundary

No packaged release app was launched for this evidence. Therefore it makes no cold-start, warm-reveal, process-tree RSS/CPU, plugin activation latency, or native media latency claim. Those values require the matched release/device protocols retained in the remaining OpenSpec tasks.
