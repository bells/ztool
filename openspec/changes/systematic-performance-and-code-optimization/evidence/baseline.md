# Baseline Evidence

## Snapshot

- Commit: `5ad4a6d92f4afc9be5c308eeeef23c0bf4c52fb9`
- Dirty state: yes; only the untracked `systematic-performance-and-code-optimization` planning/evidence directory existed before production build output was collected
- Recorded: 2026-08-22, Asia/Shanghai
- Build mode and command: production frontend, `pnpm build`
- Operating system: macOS 26.5.2 (25F84), arm64
- Hardware: MacBook Pro with Apple M2, 8 CPU cores, 16 GB memory
- Power: AC power, internal battery charged
- Display setup: not available to the non-GUI collector; this field must be filled before matched native latency/RSS comparison
- Warm-up/sample/settle: one build sample; native cold/warm and 60-second settled-idle protocols are pending startup instrumentation

## Production build baseline

The baseline build transformed 1,910 modules and completed successfully in 2.13 seconds.

| Asset | Minified | Gzip |
| --- | ---: | ---: |
| Shared `index-*.js` entry | 562.44 kB | 166.92 kB |
| Dynamic `pdfToDocx-*.js` chunk | 795.06 kB | 233.22 kB |
| Shared `index-*.css` | 54.80 kB | 10.79 kB |
| Prepared File engine assets | 4,780,883 bytes | 2,458,867 bytes |

Vite emitted its chunk-size warning. These values are pre-runtime-change reference values, not final budgets.

## Initial deterministic budgets

- Initial static JavaScript before the lazy boundary: at most 580,000 bytes and 175,000 gzip bytes.
- Largest JavaScript chunk: at most 820,000 bytes to catch accidental engine growth while allowing the measured baseline PDF converter.
- Launch pure matching: 10,000 deterministic entries, release p95 below 5 ms.
- Visible Launch results: at most 24; icon cache count remains at most 128 until a byte budget is measured.

## Lazy-loading result and reviewed budgets

The post-split production manifest records an initial entry of 267,015 bytes / 81,805 gzip bytes, down 295,425 bytes (52.5%) / 85,115 bytes (51.0%) from the baseline. The app shell is a separate 51,870-byte / 13,194-byte gzip chunk, while Capture, Pin, Paper, Launcher, each plugin panel, and the 178,760-byte File Engine remain dynamically imported. The 795,061-byte PDF-to-DOCX converter remains behind the File Engine's own dynamic boundary.

`pnpm performance:bundle:check` reports no forbidden eager import from the initial graph to Capture, Pin, Paper, Launcher, File Engine, `pdfjs-dist`, `docx`, or `docx-preview`. Based on that graph, the reviewed deterministic budgets are:

- Initial static JavaScript: at most 285,000 bytes and 90,000 gzip bytes. This leaves 17,985 bytes / 8,195 gzip bytes of bundler headroom without permitting the former eager graph.
- Largest JavaScript chunk: unchanged at 820,000 bytes because the current 795,061-byte PDF converter is intentionally isolated and has not yet been optimized.

The trade-off is a first-activation fetch/evaluation for an unloaded shell, panel, or dedicated surface. React Suspense keeps the shell responsive with a compact loading state, loader identity is cached per plugin, and import failure is contained to the selected panel. Real packaged-app first/repeat activation latency and every label-routed macOS surface remain runtime smoke gates; the bundle result alone does not prove WebView interaction quality.

## Runtime baseline status

Cold process-to-ready, warm tray/main/launcher reveal, aggregate process-tree CPU/RSS, media IPC byte counts, and ten-cycle resource recovery are not inferable from the frontend build. The pre-change binary does not emit the required phase/readiness markers, so these values remain explicitly pending rather than estimated. Task 1.6 remains open until the instrumented collector captures a comparable pre-optimization or retained baseline binary run.

Windows production/runtime baseline is pending a real Windows reference device. macOS build or cross-target results must not be reused as Windows runtime evidence.
