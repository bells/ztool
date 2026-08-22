# Surface Activity and Awake Evidence

## Implemented contract

- Rust owns the `zero://surface-activity` event and the symmetric `active | hidden | disposed` payload.
- All native Rust WebView `show`, `hide`, `close`, and `destroy` transitions now pass through the host helper; a source-contract test rejects direct transitions outside that helper.
- Frontend state subscribes before requesting its initial native visibility snapshot, filters events by window label, combines native visibility with `document.visibilityState`, refreshes the native snapshot after document reveal, and removes listeners on disposal.
- Frontend Paper hide and Pin close actions invoke host commands rather than bypassing the activity event.
- Quick Launch registers its disposable blur-dismiss listener only while its dedicated surface is active. The surface contract does not cancel native application watchers or an accepted activation.

## Awake behavior

Awake now waits for an authoritative Rust snapshot after a surface becomes active. Its one-second presentation interval starts only when the surface is active, the refreshed snapshot is ready, and native state is enabled. Hiding or disposing the surface clears the interval. Hidden expiry remains backend-owned; no hidden frontend clock attempts to disable or extend the native session.

Fake-scheduler tests cover inactive, visible, hidden, reveal-before-snapshot, expiry while hidden, stale disabled expiry, cleanup, and two independently visible Awake surfaces. Rust/TypeScript contract and native transition ownership are covered by source-contract tests.

## Verification boundary

`pnpm test:unit`, `pnpm test:integration`, `pnpm build`, and `cargo check` pass. These checks prove scheduling and source ownership, but do not prove WKWebView/WebView2 visibility event delivery or native macOS/Windows keep-awake behavior. Task 4.6 remains pending a real release-mode session on each platform.
