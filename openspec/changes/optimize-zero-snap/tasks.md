## 1. Baseline Models and RED Tests

- [x] 1.1 Add failing TypeScript unit tests for all four corner and four edge resize handles, fixed opposite edges, minimum size, image-boundary clamping, pointer cancellation inputs, and source-pixel coordinates.
- [x] 1.2 Add failing TypeScript unit tests for one-pixel selection movement in all directions, held-key repetition inputs, full-axis and edge clamping, dimension preservation, and modified/input-context key guards.
- [x] 1.3 Add failing Rust tests for Snap native activation routing, exact grouped-cell anchor propagation, repeated-click toggle behavior, compact window options, and peer transient-window coordination.
- [x] 1.4 Move the existing Paper position cases onto a shared anchored-window geometry contract, then add failing cases for Snap window dimensions, left/right edges, negative coordinates, vertically arranged displays, invalid geometry, and safe fallback selection.
- [x] 1.5 Add failing source/routing contract tests for the `snap-menu` app surface, screenshot plugin ownership, capability allowlist, typed Screenshot-only action list, localization, focus/Escape behavior, and restricted screenshot handoff command.

## 2. Host-Owned Snap Menu Window

- [x] 2.1 Extract Paper's physical anchor, target-monitor work-area clamping, and tray-relative fallback into a host-owned reusable window geometry helper without changing verified Paper behavior.
- [x] 2.2 Add `SnapMenu` to `ToolWindowKind` and update transient coordination so tray, Launch, Paper, and Snap hide peers deterministically without changing persisted status-bar settings.
- [x] 2.3 Implement a lazily created singleton `snap-menu` Tauri window with compact fixed sizing, hidden-first creation, no decorations, transparency, always-on-top, taskbar exclusion, focus, delayed blur dismissal, and Escape-compatible surface hiding.
- [x] 2.4 Add Snap to native macOS status-bar activation, derive the exact Snap cell rectangle through the existing grouped-item calculation, and toggle the anchored Snap window while preserving Ø, right-click, collapse/expand, and visibility behavior.
- [x] 2.5 Keep the non-macOS fallback action row and global screenshot shortcut on their existing direct screenshot paths, with regression assertions that no macOS menu geometry is required.

## 3. Plugin-Owned Snap Menu Surface

- [x] 3.1 Add `snap-menu` to `AppSurface`, `BundledPluginSurface`, plugin surface routing, screenshot plugin registration, and `src-tauri/capabilities/default.json`, with routing and module-boundary tests.
- [x] 3.2 Create a small typed Snap menu action model whose initial list contains only the stable Screenshot action and can accept future supported media actions without a second window or host dispatch path.
- [x] 3.3 Implement `SnapMenuApp` with plugin-local Chinese/English labels, Snap icon semantics, first-action autofocus, mouse/Enter/Space activation, Escape dismissal, busy/error feedback, and no unavailable recording placeholders.
- [x] 3.4 Add compact Zero-specific Snap menu styles with practical hit targets, visible hover/active/focus/disabled states, restrained rounding/shadow, and no horizontal overflow at the fixed window size.
- [x] 3.5 Add and register a caller-scoped `start_snap_menu_screenshot` Rust command that accepts only the `snap-menu` window, hides it before capture, delegates to the existing copy-oriented screenshot service exactly once, and restores/focuses the menu on preparation failure.
- [x] 3.6 Wire the Screenshot row through `invoke<ScreenshotStartResult>("start_snap_menu_screenshot")` and verify a successful handoff keeps the menu hidden after capture completes or is cancelled.

## 4. Pure Selection Adjustment Model

- [x] 4.1 Add an explicit eight-value `SelectionResizeHandle` union and pure resize geometry that moves only the active edges, holds the opposite edge or corner fixed, enforces the existing minimum size, and clamps to source-image bounds.
- [x] 4.2 Add a pure `moveSelectionBy` helper that preserves selection width/height and moves/clamps x/y by an image-pixel delta.
- [x] 4.3 Export shared handle descriptors and cursor/axis metadata without dynamic string lookup or `any`, and make the model tests from tasks 1.1 and 1.2 pass.

## 5. Capture Editor Interaction Integration

- [x] 5.1 Replace the loose selection drag refs in `CaptureApp` with one typed `SelectionPointerInteraction` union for create and resize gestures, including starting bounds, pointer id, active handle, draft, commit, and rollback cleanup.
- [x] 5.2 Make all eight rendered controls pointer-interactive with handle-first event routing and pointer capture; commit valid drafts on pointer up and restore prior selection on pointer cancel, lost capture, or Escape.
- [x] 5.3 Update selection CSS so four corners are circular, four edges are visually distinct, every control has a larger transparent hit region, and diagonal/vertical/horizontal resize cursors match the active axes without blocking ordinary canvas input.
- [x] 5.4 Extend capture key handling so unmodified arrow keys move the current selection one source pixel only while Select is active and no text, IME, input control, pointer selection gesture, or conflicting modifier is active; preserve native key repeat.
- [x] 5.5 Keep `activeSelection` driving the frame, outside mask, dimension badge, and toolbar anchor during resize drafts, while the committed selection remains the sole Copy/Save/Pin crop source.
- [x] 5.6 Preserve annotation hit priority outside handles, empty-canvas selection creation, rectangle/ellipse and other annotation drawing, text entry, undo/redo, Delete/Backspace, crop export, and existing toolbar behavior with focused regression tests.

## 6. Automated Verification

- [x] 6.1 Run the focused screenshot selection, hotkey, toolbar, export, reducer, routing, source-contract, status-bar, tool-window, and anchored-geometry tests using the repository's `/private/tmp/zero-tests` preparation flow.
- [x] 6.2 Run `pnpm test`, `pnpm build`, and inspect dependency/lockfile output to confirm no new frontend package is required for the menu or resize behavior.
- [x] 6.3 Run `cargo fmt --check`, `cargo check`, and `cargo test` from `src-tauri`, including the restricted command caller and Paper/Snap window regression cases.
- [x] 6.4 Run `openspec validate optimize-zero-snap --type change --strict` and `git diff --check`, then audit the diff so only this change and its intended implementation files are included.

## 7. Real Desktop Verification and Platform Boundaries

- [ ] 7.1 In a real macOS `pnpm tauri dev` session, verify exact Snap glyph targeting, Screenshot-only menu content, autofocus and keyboard activation, repeated click, Escape, blur, right-click, collapse/expand, peer-window switching, and that the menu never appears in the captured PNG.
- [ ] 7.2 Verify Snap menu placement on primary and non-primary displays including left/right edges, negative coordinates, vertical arrangements, mixed DPI/Retina, Dock work-area changes, and geometry fallback; record any behavior that cannot be automated through `SystemUIServer`.
- [ ] 7.3 In the real capture overlay, verify all four corner and four edge drags, minimum size, every image edge, fast pointer escape/cancel, live size/mask/toolbar feedback, and cropped Copy/Save/Pin output.
- [ ] 7.4 Verify single presses and held arrow keys move the selection by original-image pixels without changing size, stop at every boundary, and do not interfere with text/IME input, modified shortcuts, annotation selection, or existing editor hotkeys.
- [x] 7.5 Record Windows system screenshot and Linux unsupported/error behavior as unchanged and unverified by macOS menu/capture smoke tests; do not claim cross-platform runtime coverage from source tests.
