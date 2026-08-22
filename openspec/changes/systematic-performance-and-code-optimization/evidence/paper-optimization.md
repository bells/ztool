# Zero Paper deterministic optimization evidence

Date: 2026-08-22

This report covers deterministic implementation and automated checks for tasks 7.2–7.6. It does not claim real WKWebView decode latency, visual-quality acceptance, online behavior, RSS recovery, or a matched baseline; tasks 7.1 and 7.7 remain open for those measurements.

## Preview derivative and transport

- Cache schema 2 adds an optional versioned `*-preview-v1.jpg` derivative while accepting and lazily migrating schema 1 indexes.
- A derivative is generated atomically on a blocking worker at first preview request, with a 960×600 maximum, JPEG quality 82, and a 2 MiB byte ceiling. Corrupt, missing, wrong-format, or out-of-bound derivatives are rebuilt from the validated full-resolution cache file.
- Apply and save continue to use only the validated full-resolution cache image. The derivative path never crosses IPC.
- The preview control response contains an opaque token, MIME type, bytes, dimensions, and expiry. A separate command returns at most 2 MiB as raw `tauri::ipc::Response`; the JSON contract no longer contains `dataUrl`/`data_url`.

## Single-flight, cache, and ownership

- Per-wallpaper async locks deduplicate concurrent derivative generation. Concurrent visible surfaces reuse the same lease token and increment a reference count.
- The lease cache is bounded to 16 entries, 8 MiB of described bytes, and a five-minute lifetime, with deterministic access-order/token eviction.
- Each frontend owner creates one Blob URL and releases both the URL and native lease on replacement, hidden/disposed activity, failure, or unmount. One owner's release does not revoke a token still referenced by another surface.
- Obsolete index-owned full-resolution and preview files are cleaned together; unknown files remain untouched.

## Surface activity

- Initial cache/remote refresh and selected preview work start only while the Paper surface is `active` according to the host-owned surface activity contract.
- Hiding a surface disposes its presentation request gate and preview resources. Already accepted native refresh/download/apply/save work retains its existing lifecycle, and stale completions cannot update hidden/disposed React state.

## Executed checks

- `cargo test --manifest-path src-tauri/Cargo.toml --test bing_wallpaper` — 17 passed, including concurrent token sharing, raw-byte bounds, reference-counted release, corrupt derivative rebuild, full-resolution preservation, cache-first/offline behavior, retention, and native apply/save.
- `cargo test --manifest-path src-tauri/Cargo.toml preview_cache_tests` — 2 passed for deterministic eviction, expiry, and count bounds.
- Bing Wallpaper Node unit tests — 13 passed, including hidden/disposed gating, stale completion, descriptor byte matching, and symmetric service commands.
- Surface activity integration tests — 4 passed, including Blob URL/native lease cleanup and removal of Base64 preview contracts.
- `cargo check --manifest-path src-tauri/Cargo.toml`, `pnpm build`, and `git diff --check` — passed. Production entry remained 267.14 kB / 81.86 kB gzip.

## Pending matched evidence

Task 7.1 remains open because current full-resolution dimensions, Base64/JSON byte counts, decode latency, cache hits, real-WebView duplicate requests, peak RSS, and ten-selection settle samples were not captured before this runtime change. Task 7.7 remains open for real online/offline navigation, visual preview quality, rapid selection, apply/save, cache recovery, two visible surfaces, ten-cycle macOS memory settle, and the separate Windows device run.
