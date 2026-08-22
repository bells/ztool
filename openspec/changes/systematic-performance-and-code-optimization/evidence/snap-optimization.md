# Zero Snap deterministic optimization evidence

Date: 2026-08-22

This report covers deterministic contract, ownership, and lifecycle evidence for tasks 6.1–6.7. It does not claim a real WKWebView/AppKit memory result, Retina or multi-display correctness, capture-permission behavior, or Windows device behavior; tasks 6.8 and 6.9 retain those platform gates.

## Raw media and two-phase commit

- macOS capture writes the PNG directly into an owner-only `screenshot-media/session-*` directory. The JSON initialization payload contains only an opaque token, MIME type, byte length, dimensions, and expiry; it exposes no filesystem path or Base64 field.
- `read_screenshot_media` returns a bounded raw `tauri::ipc::Response`. Tokens are limited to URL-safe opaque characters and resolve only through the active capture session or the calling `pin-*` window's Rust-owned record.
- Copy, save, and pin use `prepare_screenshot_commit` followed by one raw `tauri::ipc::Request`. The preparation result is a 30-second, one-action, one-session upload lease capped at 100 MiB. At most four pending leases exist.
- Rust consumes a lease before validating its window/session/action/body, so a failed, interrupted, expired, or mismatched upload cannot replay it. Upload validation checks the PNG signature, MIME/format, a 32,768-pixel per-axis bound, and a 268,435,456-pixel total bound without decoding the full image.
- Save destinations come only from native `rfd::AsyncFileDialog`; the frontend contract contains no path. Copy continues through the native macOS pasteboard. Pin uploads are atomically staged into a separate owner-only directory before their window is created.

## Ownership and terminal cleanup

- Capture session files and all leases are removed on successful copy/save, cancel, session expiry, capture-window destruction, replacement, or process shutdown. An old capture-window destruction callback is session-keyed and cannot clear a newer session.
- A pin owns one token and one PNG file. `init_pin_window` resolves only the current window label; window destruction removes the Rust map entry and file. Failed pin creation/show also removes both.
- Startup/first capture removes only expired `session-*` and `pin-*` directories under Zero's fixed cache root. Unknown sibling files/directories remain untouched.
- Capture and Pin create Blob object URLs from raw `ArrayBuffer` values. Replacement, errors, mid-load disposal, and unmount revoke the URL; decoded image sources are cleared.
- Screenshot export uses `canvas.toBlob`. Crop canvases and full-resolution source canvases shrink to 1×1 in `finally`, and raw received/upload byte arrays are zeroed on terminal paths.

## Ten-cycle deterministic resource result

- Rust ten-cycle lifecycle test: after every session create/clear cycle, active sessions = 0, upload leases = 0, pins = 0, and the owned session directory no longer exists.
- Frontend ten-cycle helper test: after every simulated export lifecycle, live object URLs = 0, decoded image source is empty, and canvas backing size is 1×1. Across ten distinct URLs, exactly ten revocations occur.
- These counts prove deterministic application-owned cleanup only. They are not a WKWebView RSS/decode-memory result and do not complete task 6.8.

## IPC byte effect

For a PNG of `N` bytes, the former capture/read plus commit round trip carried approximately `2 × 4 × ceil(N / 3)` Base64 characters in JSON, excluding JSON keys and temporary string copies. The new path carries `2 × N` raw bytes. For normal large payloads this removes approximately 25% of the wire bytes relative to the former Base64 payload (equivalently, Base64 was approximately 33% larger than raw), plus the duplicate Rust and JavaScript Base64 strings. This is a contract-derived byte result, not a measured native IPC latency or RSS claim.

## Executed checks

- `cargo test --manifest-path src-tauri/Cargo.toml services::screenshot::tests -- --skip native_png_clipboard_round_trips_on_private_pasteboard` — 13 deterministic tests passed.
- The isolated macOS private-pasteboard test passed outside the filesystem/process sandbox. Its earlier sandboxed attempt reported the system pasteboard unavailable, which is retained as an environment boundary rather than a product result.
- `pnpm test:unit` — 148 passed, including raw preparation/header serialization and ten-cycle frontend resource cleanup.
- `pnpm test:integration` — 68 passed, including raw request/response source shape, removal of Snap Base64 fields, token/window scoping, terminal cleanup, and retention of the Windows system-launcher path.
- `pnpm build` — passed; Capture chunk 20.31 kB / 6.96 kB gzip and Pin chunk 1.39 kB / 0.75 kB gzip.
- `cargo check --manifest-path src-tauri/Cargo.toml` passed before the final deterministic helper extraction; the final Rust test compile covers that extraction and the final aggregate gates rerun it.

## Pending real-platform evidence

Task 6.8 remains open for real macOS capture permission, Retina/multi-display selection, annotations, cropped copy/save, live pin lifetime, cancellation/failure recovery, title-bar/fullscreen behavior, and ten-cycle process-tree RSS settle. Task 6.9 remains open for a real Windows `ms-screenclip:`/Snipping Tool device run; source-contract coverage and macOS behavior are not promoted to Windows runtime evidence.
