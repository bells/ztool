# Reference Performance Workflows

All runtime workflows use a packaged or release-mode build from the recorded commit and dirty state. Cold startup uses at least 10 samples; warm reveal uses at least 30 samples. Hidden-idle and post-workflow snapshots use a 60-second settle interval and aggregate the complete Zero process tree.

## Common capture

Record native phase events, first frontend readiness, per-surface reveal request/readiness, process-tree CPU/RSS, IPC payload bytes, owned native resource counts, and frontend Blob/object-URL/canvas counts where applicable. Report raw samples plus p50/p95 latency and median CPU/RSS. A browser-only simulation cannot close a native WebView resource or memory gate.

## Ten-cycle workflows

1. **Zero Launch:** reveal panel or launcher, type a deterministic Chinese/pinyin/initial/alias sequence, wait for visible icons, activate or dismiss, hide, repeat. Record query/IPC/icon request counts, first/repeat latency, watcher count, peak RSS, and post-settle RSS.
2. **Zero Snap and pin:** start capture, create/edit a selection, then alternate copy, save, pin/close, and cancel paths. Record session/token/file/pin-map/object-URL/canvas counts and IPC bytes. Real macOS capture permission, Retina, multi-display, AppKit window, and clipboard behavior remain manual.
3. **Zero Awake:** enable finite and unlimited sessions, hide/reveal the containing window, toggle off, and include an expiry while hidden. Record presentation timer count, native keep-awake ownership, CPU, and authoritative snapshots. Native macOS and Windows assertions require device evidence.
4. **Zero Paper:** navigate across ten cached selections with rapid replacements, include one refresh and one apply/save, then hide. Record cache/preview generation count, full/preview bytes, object URLs, peak RSS, and post-settle RSS. Repeat online and offline where network evidence is required.
5. **Zero File:** enqueue representative PDF-to-DOCX and DOCX-to-PDF fixtures sequentially, include cancellation and an adjacent compatible job, then wait past engine idle teardown. Record provider discovery count, engine cold/warm latency, session/job/artifact counts, peak RSS, and post-settle RSS. Provider availability and fidelity are platform-specific.

## Manual boundaries

- macOS: tray/AppKit status item, overlay/pin window levels, native capture/clipboard, Word/LibreOffice automation, Finder actions, real WKWebView process memory.
- Windows: `.lnk` discovery, window focus, system capture, WebView2 process memory, Office/LibreOffice automation, Explorer actions.
- Cross-target compilation and Node/browser tests never substitute for those runtime checks.
