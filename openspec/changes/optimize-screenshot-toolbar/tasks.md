## 1. Positioning Model and Tests

- [x] 1.1 Add RED Node tests for full-image default selection, normalized/clamped drag selection, minimum-size preservation, image-to-viewport mapping with scale/letterbox offsets, and independence from rectangle annotations.
- [x] 1.2 Add RED Node tests for toolbar placement below a normal selection, inside a full-screen/bottom-edge selection, above a short selection, horizontally clamped at both viewport edges, and recalculated for changed toolbar or viewport dimensions.
- [x] 1.3 Add typed `captureSelectionModel.ts` and `captureToolbarModel.ts` pure helpers with shared geometry tokens and observable placement results.

## 2. Real Selection, Crop, and Toolbar Integration

- [x] 2.1 Initialize a full-image screenshot selection, let Select drags on annotation-free areas create a valid draft/selection, and preserve existing annotation selection and drawing behavior.
- [x] 2.2 Render the mapped selection frame, dimensions, handles, and outside dimming as non-exported UI, and crop Copy/Save output to the real selection through the existing IPC payload.
- [x] 2.3 Measure the toolbar with a ref and `ResizeObserver`, recompute on selection and viewport changes, and apply the model's `left`/`top` result without a fixed-bottom flash during initial layout.
- [x] 2.4 Update screenshot selection/toolbar CSS to remove fixed toolbar centering, preserve layering above the canvas, and keep every action pointer-accessible.

## 3. Graphical Toolbar Controls

- [x] 3.1 Add `lucide-react` with the project package manager and verify the lockfile contains only the intended icon dependency change.
- [x] 3.2 Create explicit strongly typed tool/action descriptors for Select, Rectangle, Arrow, Pen, Text, Mosaic, Pin, Undo, Redo, Delete, Cancel, Save, and Copy using named Lucide icon imports and the existing functional order.
- [x] 3.3 Reuse the existing stored-preference language resolution pattern in the capture window, add Chinese and English toolbar translation keys, and give every icon button a localized tooltip and `aria-label`.
- [x] 3.4 Replace visible English button text with icon components, add `aria-pressed` to selectable tools, and preserve current click handlers, native disabled behavior, hotkeys, and commit guards.

## 4. Visual States and Accessibility

- [x] 4.1 Refine the toolbar into tool, history/delete, and completion groups with consistent 44×44 hit targets, 18–20px icons, restrained dividers, and the existing dark surface/green accent vocabulary.
- [x] 4.2 Implement and verify default, hover, active, selected, focus-visible, disabled, danger, and confirm states with non-color cues for selected and destructive/confirmation actions.
- [x] 4.3 Add source or DOM-level regression coverage for icon-only faces, localized accessible names, `aria-pressed`, disabled commit actions, and stable keyboard order.

## 5. Verification

- [x] 5.1 Run focused screenshot selection, toolbar model, hotkey, reducer, canvas, export, serialization, i18n, and icon-control tests, then run the complete Node test suite using the repository's documented TypeScript fixture build steps.
- [x] 5.2 Run `pnpm build`, `cargo fmt --check`, `cargo check`, `cargo test`, `openspec validate optimize-screenshot-toolbar --type change --strict`, and `git diff --check`.
- [ ] 5.3 In a real macOS `pnpm tauri dev` session, verify the capture overlay opens with no system title bar or native fullscreen Space transition, full-screen default selection, Select drag, rectangle-annotation independence, cropped copy/save, normal/bottom/left/right/narrow/short toolbar positions, Retina/display scaling, live repositioning, every icon tooltip, keyboard focus/order, hotkeys, multiline Text input with Enter, left-click text completion, Ellipse drawing/export, pin, cancel, and no toolbar flash or clipping.
- [x] 5.4 Record Windows and Linux as unchanged platform boundaries; do not claim the macOS custom-editor checks verify their system screenshot paths.
- [x] 5.5 Replace the failing macOS AppleScript PNG clipboard transport with native `NSPasteboard` data and verify it through an isolated pasteboard round-trip test that does not overwrite the user's clipboard.
- [x] 5.6 Keep the Text tool input focused after the canvas pointer sequence, avoid text-mode pointer capture, preserve IME composition on Enter, and add source regression coverage.
- [x] 5.7 Make Enter insert text line breaks, commit text by left-clicking outside without duplicate drafts, add Ellipse immediately after Rectangle, and cover multiline bounds/rendering, typed ellipse integration, icon order, and localization with focused tests.
- [x] 5.8 Replace macOS native fullscreen capture-window creation with a hidden borderless window sized to the primary monitor's physical bounds, close it on preparation failure, and add source regression coverage for the no-title-bar contract.

> Platform boundary: this change only modifies the macOS custom capture editor. Windows continues to use its system screenshot path, Linux remains on its existing unsupported/error path, and neither platform is verified by the macOS acceptance checklist in 5.3.
