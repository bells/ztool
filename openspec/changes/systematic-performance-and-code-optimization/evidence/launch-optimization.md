# Zero Launch Optimization Evidence

## Query and running-state path

The Rust index now publishes `Arc`-owned immutable item and usage snapshots. Search clones only the two `Arc` handles under a short index read lock, then performs matching through a thread-local Nucleo matcher with no shared application-state lock. The query function contains no filesystem, process probe, icon decode, or platform call, and running state is overlaid from an immutable cache without mutating the indexed items.

Running-process probing has a two-second TTL, a single-flight native refresh gate, and a typed `zero://quick-launcher/running-state-updated` event. Visible Launch surfaces request refresh at the TTL boundary and reschedule their current query when the revision event arrives. Hidden/disposed surfaces remove the timer/listener and ignore late results.

## Frontend scheduling and icons

- Query coalescing interval: 40 ms.
- The scheduler retains only the latest pending query, permits at most one search IPC at a time, retains the generation guard, cancels pending work while hidden, and flushes before Enter.
- Activation resolves the item ID and index revision from the flushed result; Rust rejects stale revisions before any platform action.
- Visible icons use one typed batch of at most 16 requested rows within the native maximum of 24.
- Native icon decoding is serialized to maximum concurrency one. Cache limits are 512 KiB per data URL, 128 entries, and 8 MiB total with deterministic LRU/key tie-breaking.
- A completed, current batch produces one bounded React state update; superseded/hidden batches are ignored.

## Deterministic scheduling benchmark

`pnpm performance:launch` ran 30 six-keystroke `v` → `vscode` fixtures:

- frontend query changes: 180
- search IPC executions: 30
- superseded requests: 150
- request-count reduction versus immediate dispatch: 83.3%
- maximum search concurrency: 1
- scheduler/bridge-stub p50: 1.667 µs
- scheduler/bridge-stub p95: 10.375 µs
- index clone bytes per query: 0 (source-contract asserted)
- running probes per query: 0 (source-contract asserted)
- native icon-load concurrency: 1
- React commits per completed icon batch: 1

This deterministic result measures coalescing and contract overhead, not real Tauri IPC or WebView latency. There was no comparable pre-change real-device end-to-end sample, so a real end-to-end improvement is not claimed here; request count is directly comparable to the former one-invoke-per-input implementation.

## Release pure-search benchmark

Command:

`cargo test --release --manifest-path src-tauri/Cargo.toml --test quick_launcher_benchmark -- --ignored --nocapture`

Result on the baseline M2 reference environment, 10,000 mixed English/Chinese/alias entries and 80 samples:

- reused explicit Nucleo matcher: p50 3.850 ms, p95 4.256 ms
- production thread-local matcher path: p50 3.967 ms, p95 4.307 ms
- comparison fuzzy-matcher: p50 2.645 ms, p95 3.455 ms
- production pure-search p95 gate: pass, 4.307 ms < 5 ms

Final automated-gate rerun after the Rust scheduling audit: reused Nucleo p50/p95 4.003/4.376 ms, production thread-local p50/p95 4.027/4.416 ms, comparison fuzzy-matcher p50/p95 2.639/3.565 ms, `index_clone_bytes=0`, and `running_probe_count=0`. The production p95 remains below the 5 ms gate.

The benchmark retains Nucleo because this change does not reopen the prior match-quality/library selection decision. Real macOS panel/launcher interaction, focus/launch, filesystem changes, and Windows behavior remain Task 5.8 gates.
