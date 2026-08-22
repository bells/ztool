# Active Change Overlap

Recorded against `5ad4a6d92f4afc9be5c308eeeef23c0bf4c52fb9` on 2026-08-22.

| Active change | Reused by this change | Gates that remain owned by the active change |
| --- | --- | --- |
| `reorganize-project-modules` | Bundled descriptors, one frontend/native composition root, and structural import tests | Real macOS tray/main/preferences/about and plugin entry smoke; Windows runtime remains separate |
| `add-quick-launcher-plugin` | Cache, watcher, search, activation, icon, IPC, and benchmark tests | Watcher degradation/event-storm coverage, remaining IPC/controller tests, full macOS interaction matrix, and real Windows `.lnk`/focus/runtime matrix |
| `optimize-screenshot-toolbar` | Capture selection/editor/export behavior and source-contract tests | Full real macOS overlay/editor/Retina/toolbar/pin smoke |
| `add-file-plugin` | Sequential Rust queue, provider contracts, result actions, engine isolation, and fixtures | Clippy gate plus real macOS and Windows provider/UI/conversion smokes |
| `bundle-zero-file-offline-engines` | Signed engine package, raw engine IPC, page cleanup, and packaging checks | Any unchecked package/runtime/fidelity/device evidence remains in that change |

This performance change does not archive another change and does not mark any of the manual or platform-specific items above complete. Automated source, unit, build, or cross-target checks are recorded separately from real AppKit/WebView2 behavior.
