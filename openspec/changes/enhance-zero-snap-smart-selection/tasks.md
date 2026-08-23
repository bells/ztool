## 1. Contracts, Dependency Gate, and RED Tests

- [x] 1.1 Add failing Rust tests for `ScreenshotTargetCandidate` camelCase serialization, opaque ids, front-to-back ordering, and the absence of application title/name, pid, z, and global-coordinate fields in `CaptureSessionPayload`.
- [x] 1.2 Add failing Rust pure-model tests for native-window filtering, current-process exclusion, minimized/invalid/desktop candidates, partial monitor intersection, negative origins, outside rounding, 1x/2x coordinate conversion, and empty-list degradation.
- [x] 1.3 Add failing TypeScript tests for topmost candidate hit resolution, complete-image fallback, pointer exit, stable click commit, the 4 CSS-pixel drag threshold, free-selection takeover, pointer cancellation, and source-pixel bounds.
- [x] 1.4 Add failing TypeScript tests for `SelectionGeometry` normalization, fixed-upper-left width/height edits, minimum/maximum validation, invalid draft rollback, radius limits, and radius reclamping after resize.
- [x] 1.5 Add failing geometry-control positioning and keyboard-guard tests covering above/inside/clamped placement, small selections, dimension inputs, IME composition, slider arrow input, and canvas/hotkey isolation.
- [x] 1.6 Add failing export tests that sample rectangular and rounded PNG corner alpha, clip annotations with the same radius, preserve output dimensions, and prove capture chrome is not rendered.
- [x] 1.7 Add failing source-contract tests for symmetric Rust/TypeScript target fields, macOS-only candidate provider/dependency scope, initial null selection, horizontal/vertical guides, accessible width/height/radius controls, and no shadow/OCR/aspect-ratio additions.

## 2. Native macOS Window Candidate Adapter

- [x] 2.1 Add `xcap` 0.9.8 only to the macOS Cargo target, update the lockfile, confirm Apache-2.0 licensing, and inspect `cargo tree`/bundle-relevant duplicate dependencies so the proposal's maturity choice is verified in the actual build.
- [x] 2.2 Create a screenshot-internal `capture_targets` adapter and pure `NativeWindowSnapshot`/`NativeCaptureGeometry` models so no `xcap::Window` type crosses into session, command, or test-facing layers.
- [x] 2.3 Enumerate windows in native z order after Zero transient surfaces are hidden; read each fallible metadata field defensively and exclude the Zero process, minimized/invisible, invalid-small, identifiable desktop, and non-intersecting windows.
- [x] 2.4 Convert accepted global window rectangles to the current capture image's source pixels with explicit monitor origin/physical size, outside rounding, clipping, stable opaque ids, and deterministic ordering.
- [x] 2.5 Make provider-wide or per-window enumeration failures produce bounded diagnostics and an empty/partial candidate list without failing the existing PNG capture, permission handling, cleanup, or shell restoration path.

## 3. Rust-TypeScript Session Integration

- [x] 3.1 Add serializable Rust `ScreenshotSourceBounds`, `ScreenshotTargetKind`, and `ScreenshotTargetCandidate` types and store the candidate snapshot inside `ScreenshotSession` beside the matching PNG media.
- [x] 3.2 Extend `CaptureSessionPayload` with required `targets`, populate it from the session snapshot, and update Rust fixtures/tests to use explicit empty arrays rather than a compatibility-weak optional field.
- [x] 3.3 Add exactly symmetric TypeScript target interfaces/unions to `captureTypes.ts`, update capture-session fixtures and invoke consumers, and keep `any` out of all narrowing and fallback paths.
- [x] 3.4 Preserve the current session id, media token, read scope, upload lease, Copy/Save/Pin input, cancellation, and restore-window contracts with focused regression tests.
- [x] 3.5 Confirm non-macOS compilation does not instantiate or require the custom candidate provider and that Windows/Linux launcher/error behavior remains unchanged.

## 4. Smart Targeting and Pointer Guides

- [x] 4.1 Add pure target normalization and `resolveScreenshotTargetAtPoint` helpers that respect native ordering, validate source bounds, and synthesize the complete-image fallback without representing it as a native window.
- [x] 4.2 Replace `createFullImageSelection` startup in `CaptureApp` with an explicit targeting phase whose hover preview is separate from the committed selection and cannot activate adjustment, annotations, toolbar, or export.
- [x] 4.3 Extend the typed selection pointer union with `pending-target`, retain one pointer id owner, and implement stable-candidate click commit plus 4 CSS-pixel transition into existing source-pixel free selection creation.
- [x] 4.4 Preserve pointer capture, normalization, minimum size, cancellation, lost-capture, Escape rollback, selection resize, annotation hit priority, and later empty-canvas reselection while integrating the new initial phase.
- [x] 4.5 Render a non-interactive candidate boundary that switches across overlapping/application windows and clears when the pointer leaves the actual contained image area.
- [x] 4.6 Render one viewport-mapped horizontal and one vertical guide through the pointer during targeting/free creation, hide both after selection commit or incompatible editing, and keep them out of Canvas export.
- [x] 4.7 Add screenshot-plugin Chinese/English accessibility labels and concise hints for target selection and guides without copying the recording's unrelated shortcut/help overlay.

## 5. Selection Geometry Controls and Rounded Rendering

- [x] 5.1 Introduce a typed `SelectionGeometry { bounds, cornerRadius }` model, keep existing bounds helpers reusable, and normalize radius after create, resize, nudge, target commit, and numeric dimension edits.
- [x] 5.2 Implement pure fixed-upper-left width/height commits with integer parsing, minimum size, image-edge maximums, invalid-state preservation, and Escape rollback inputs.
- [x] 5.3 Implement a pure geometry-control position resolver that prefers the selection's upper-left outside edge, falls back inside/alternate vertical placement, and clamps the complete control to the capture viewport.
- [x] 5.4 Build a focused `SelectionGeometryControls` component with two labelled numeric inputs, independent string drafts, Enter/Tab/blur commit, Escape restore, visible invalid state, and a labelled source-pixel radius slider.
- [x] 5.5 Ensure input/slider focus, pointer events, Arrow keys, IME composition, Tab, Enter, and Escape do not leak into selection nudge, annotation hotkeys, Delete, Canvas creation, or global screenshot cancellation.
- [x] 5.6 Replace the rectangular box-shadow selection dimming with SVG even-odd rounded selection chrome while preserving eight resize handles, source-pixel size feedback, active draft rendering, and toolbar anchoring.
- [x] 5.7 Extend the shared capture export pipeline to accept selection radius and apply one rounded `destination-in` alpha mask after drawing the base image and annotations; keep radius-zero export on the current rectangular path.
- [x] 5.8 Route Copy, Save, and Pin through the same normalized geometry/export result, preserve output width/height and upload limits, and verify no shadow, extra margin, background, OCR, or aspect lock is introduced.
- [x] 5.9 Add compact Zero-specific styles for target preview, thin crosshair guides, numeric inputs, slider, invalid/focus states, and viewport fallbacks with practical pointer/touch hit areas and reduced-motion compatibility.

## 6. Automated Verification

- [x] 6.1 Run the focused screenshot selection, Canvas, export, hotkey, toolbar, reducer, media lifecycle, capture-window, session-contract, and new targeting/geometry tests through the `/private/tmp/zero-tests` preparation flow.
- [x] 6.2 Run `pnpm test`, `pnpm build`, and `git diff --check`, then audit the diff for plugin ownership, Rust/TypeScript contract symmetry, lockfile scope, and absence of generated artifacts.
- [x] 6.3 Run `cargo fmt --check`, `cargo check`, and `cargo test` from `src-tauri`, including macOS target-provider tests and non-macOS cfg/source-contract coverage available on the current host.
- [x] 6.4 Run `openspec validate enhance-zero-snap-smart-selection --type change --strict` and reconcile every spec scenario with an automated test or an explicitly manual runtime task below.
- [x] 6.5 Add source-contract tests that keep `open_capture_window` hidden, require decoded-and-committed frontend content before reveal, scope reveal to the active capture session, and preserve the Windows system-launcher-only path.
- [x] 6.6 Split macOS capture preparation from reveal; make reveal idempotent, configure the native window on the AppKit main thread above menu bar/Dock without application-wide fullscreen state, and restore shell/session resources on every failure.
- [x] 6.7 Invoke reveal from `CaptureApp` only after session media decode and React DOM commit, guard StrictMode duplicates, and cancel the hidden session when initialization or reveal fails.
- [x] 6.8 Run focused reveal/media/capture-window tests plus `pnpm test`, `pnpm build`, Rust format/check/tests, strict OpenSpec validation, and `git diff --check`.
- [x] 6.9 Add a RED capture-window contract for size-before-position, fix the hidden window frame preparation order, and rerun focused plus complete frontend/Rust/OpenSpec gates.

## 7. Real macOS and Platform-Boundary Verification

- [ ] 7.1 In a real `pnpm tauri dev` session with Screen Recording permission, verify screenshot startup has no committed full-image selection, window candidates follow Finder/browser/Tauri windows in visible z order, single click snaps exactly, and drag immediately becomes free selection.
- [ ] 7.2 Verify the horizontal/vertical guides follow the pointer across the captured image, candidate and full-image fallback boundaries switch correctly, and all targeting chrome disappears after commit and from copied/saved/pinned PNGs.
- [ ] 7.3 Verify width/height typing, Enter/Tab/blur, invalid input, Escape rollback, native input/slider keyboard behavior, radius live preview, resize/nudge interaction, small selections, and control placement at every viewport edge.
- [ ] 7.4 Inspect saved PNG alpha at radius zero, a small radius, and the maximum radius; verify annotations are clipped at rounded corners and Copy, Save, and Pin show the same geometry without shadow or margin.
- [ ] 7.5 Verify Retina and available multi-display cases including 1x/2x mapping, negative display origins, windows partially crossing the captured display, overlapping windows, full-screen windows, and candidate-enumeration fallback; record limits caused by the existing single-capture-display model.
- [x] 7.6 Record Windows system launcher and Linux/other unsupported behavior as unchanged and not runtime-proven by macOS tests; do not claim cross-platform custom-overlay support from source checks.
- [ ] 7.7 In a real macOS capture, verify no blank/white first frame and exactly one frozen menu bar/Dock; record Windows as system-launcher-only by source unless a Windows-device smoke test is available.
- [x] 7.8 In a real macOS capture, verify the overlay fills the complete Retina display without upper-half displacement; runtime inspection confirmed `(0, 0, 1440×900)` at native layer 1000 and clean Escape teardown. No negative-origin or vertically stacked display was available, so those layouts remain explicitly covered by 7.5 rather than inferred from this single-display smoke.
