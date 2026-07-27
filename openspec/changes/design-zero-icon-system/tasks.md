## 1. Canonical SVG Sources

- [x] 1.1 Add the five reviewed 24×24 `currentColor` SVG masters for Zero, Zero Launch, Zero Snap, Zero Awake, and Zero Paper under `src/assets/icons/`.
- [x] 1.2 Add the Zero Awake active-state derivative with identical outer bounds and the documented liquid-level mark.
- [x] 1.3 Add the 512×512 `zero-app-icon.svg` master with the dark charcoal container and optically equivalent white `Ø`.
- [x] 1.4 Add concise icon-system documentation covering metaphor, grid, stroke, clear space, supported sizes, source-of-truth rules, and the deterministic generation command.

## 2. Asset Validation and Review Outputs

- [x] 2.1 Add a dependency-free structural validation script that checks icon inventory, XML structure, exact 24×24 view boxes, `currentColor`, transparency, and prohibited font/text/filter/external/raster content.
- [x] 2.2 Add automated tests for the validator and each canonical source, including the Zero slash geometry and the Awake base/active outer-bound contract.
- [x] 2.3 Add a deterministic contact-sheet workflow that renders the icon family at 16px, 18px, 22px, 24px, 128px, and 512px on light and dark review backgrounds.
- [x] 2.4 Review the generated contact sheet at 1× and 2×, correcting only the canonical SVG geometry when clipping, blur, merged detail, or inconsistent optical weight is found.

## 3. Native Status-Bar Assets

- [x] 3.1 Generate transparent monochrome RGBA tray derivatives from the approved SVG masters under `src-tauri/icons/tray/` and document their source mapping.
- [x] 3.2 Extend Rust `StatusBarIconId` handling additively for Zero Launch and Zero Paper while preserving all existing serialized icon identifiers.
- [x] 3.3 Replace procedural `IconCanvas` drawing with bundled canonical tray derivatives and retain `.icon_as_template(true)` for macOS status items.
- [x] 3.4 Map `screenshot`, `caffeine-empty`, and `caffeine-full` to the Zero Snap, Zero Awake base, and Zero Awake active derivatives without changing status-item actions.
- [x] 3.5 Add Rust tests for complete icon-ID resolution, valid RGBA dimensions/transparency, backward-compatible deserialization, and Awake state mapping.

## 4. React Icon Surfaces

- [x] 4.1 Add a typed frontend icon-source map covering existing icon identifiers plus additive `launch` and `paper` identifiers without using `any`.
- [x] 4.2 Refactor `StatusBarGlyph` to render trusted canonical SVG assets through a `currentColor` CSS mask.
- [x] 4.3 Remove the duplicated first-party CSS pseudo-element drawings after the canonical SVG mapping covers status previews and fallback action rows.
- [x] 4.4 Add frontend tests that verify every first-party status icon identifier resolves to the intended Zero family source and that existing identifiers remain compatible.

## 5. Application Bundle Icon

- [x] 5.1 Render and review a 512×512 `app-icon.png` from `zero-app-icon.svg`, preserving the specified background, inset, `Ø` proportions, and RGBA format.
- [x] 5.2 Use the existing Tauri icon workflow to regenerate the tracked PNG, ICNS, ICO, Windows tile, Android, and iOS assets from the approved application master.
- [x] 5.3 Verify every icon path referenced by `src-tauri/tauri.conf.json` exists, decodes successfully, and is accepted by the Tauri build context.

## 6. Verification

- [x] 6.1 Run the icon structural tests, status-bar frontend tests, full Node test suite, and production frontend build.
- [x] 6.2 Run focused Rust status-bar tests followed by `cargo check` and `cargo test`.
- [x] 6.3 Run strict OpenSpec validation and `git diff --check`.
- [x] 6.4 In a real macOS Tauri session, verify the Zero and plugin status icons in both light and dark appearances and confirm automatic Template Image recoloring.
- [ ] 6.5 Toggle and expire Zero Awake from the real menu bar, confirming the base/active derivative changes without position or click-target movement.
- [x] 6.6 Inspect the Windows/fallback React action row with inherited light/dark colors and record separately any platform behavior not covered by the macOS native smoke pass.
