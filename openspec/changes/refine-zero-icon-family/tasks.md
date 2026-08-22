## 1. Geometry Contracts and Regression Tests

- [x] 1.1 Update icon-system tests to require a centered vertical ellipse and exact 45-degree slash in `zero.svg` and optically equivalent foreground geometry in `zero-app-icon.svg`.
- [x] 1.2 Add source-contract assertions that both Awake sources contain no steam path, share identical outer cup/handle/saucer geometry, and differ only by the bounded active-state liquid mark.
- [x] 1.3 Add source-contract assertions that Launch contains the reviewed sparse rocket body, fin, and exhaust cues and no legacy `>_` paths.
- [x] 1.4 Add React source-contract coverage that the main-window header and About surface render the canonical Zero source without literal `Z` foregrounds or duplicate SVG path data.
- [x] 1.5 Run the focused icon/source-contract tests and confirm they fail only for the intended pre-change geometry.

## 2. Canonical SVG Refinement

- [x] 2.1 Replace the circle in `zero.svg` with the reviewed centered duck-egg ellipse while retaining the exact 45-degree slash, shared two-unit stroke, and transparent `currentColor` contract.
- [x] 2.2 Replace the application-master circle with an optically equivalent vertical ellipse while retaining the current 512×512 container, contrast, inset, and slash direction.
- [x] 2.3 Redraw `zero-awake.svg` as an enlarged steam-free cup with right handle and saucer inside the shared clear-space envelope.
- [x] 2.4 Update `zero-awake-active.svg` to reuse the exact new Awake outer geometry and add only the reviewed internal liquid-level state mark.
- [x] 2.5 Replace the Zero Launch terminal prompt with the reviewed upper-right 45-degree rocket, removing any optional detail that merges at 16px or 18px.
- [x] 2.6 Run `pnpm icons:validate` and the focused icon tests, correcting only canonical source geometry until all structural contracts pass.

## 3. Shared React Brand Surfaces and Documentation

- [x] 3.1 Replace the main-window header's text `Z` with the canonical Zero CSS-mask glyph while preserving its existing container dimensions, alignment, drag region, and accessible heading.
- [x] 3.2 Replace the About panel's text `Z` with the same canonical Zero glyph while preserving its existing container dimensions and layout.
- [x] 3.3 Refactor shared mark rendering only as needed to avoid duplicate foreground geometry and keep typed icon identifiers without TypeScript `any`.
- [x] 3.4 Update `docs/icon-system.md` with the duck-egg Zero, rocket Launch, steam-free enlarged Awake, React brand-surface reuse, and unchanged Template Image rules.
- [x] 3.5 Run the focused React/source-contract tests and inspect the main-window and About marks at their real rendered sizes.

## 4. Contact Sheet and Generated Assets

- [x] 4.1 Regenerate the icon contact sheet and review all identities on light and dark backgrounds at 16px, 18px, 22px, and 24px plus the application master previews at 128px and 512px.
- [x] 4.2 Compare Zero, Launch, and both Awake states at 1× and 2×, then apply at most half-grid optical adjustments to the canonical SVGs when clipping, blur, merging, or weight imbalance is visible.
- [x] 4.3 Re-run the contact-sheet and structural checks after optical adjustment and confirm Zero Snap, Zero Paper, and extension canonical sources are unchanged.
- [x] 4.4 Regenerate all transparent 18×18 tray PNG derivatives from the approved masters and verify RGBA dimensions, alpha, monochrome pixels, and stable base/active Awake bounds.
- [x] 4.5 Regenerate the Tauri PNG, ICNS, ICO, Windows tile, Android, and iOS application assets from the approved `zero-app-icon.svg` master and verify every configured bundle path.
- [x] 4.6 Review generated diffs together with their SVG sources and confirm no manually redrawn or untracked derivative was introduced.

## 5. Automated and Native Verification

- [x] 5.1 Run `pnpm test:icons`, the relevant status-bar and brand tests, and the full `pnpm test` suite.
- [x] 5.2 Run `pnpm build` and confirm the React main-window, About, status preview, and fallback masks compile without duplicated icon geometry.
- [x] 5.3 Run focused Rust status-bar tests followed by `cargo fmt --check`, `cargo check`, and `cargo test` under `src-tauri/` to verify embedded generated assets and grouped composition.
- [x] 5.4 Run `openspec validate refine-zero-icon-family --type change --strict` and `git diff --check`.
- [x] 5.5 In a real macOS Tauri session, inspect the main-window and About marks, then verify the grouped Zero, Launch, and Awake status icons in light and dark appearances with native Template Image recoloring.
- [x] 5.6 Toggle Zero Awake on and off from the real menu bar and exercise timed expiry, confirming the enlarged base/active cup changes without cell movement, clipping, action regression, or hit-target change.
- [x] 5.7 Record platform verification honestly, including any unrun Windows runtime or macOS `SystemUIServer` path, and retain both OpenSpec changes unarchived until the base `zero-icon-system` capability can be synchronized before this follow-up.

## 6. Asymmetric Solid Zero Refinement

- [x] 6.1 Update source contracts to require one shared top-narrow, bottom-full solid egg path with a negative 45-degree slash and reject symmetric circle or ellipse foregrounds.
- [x] 6.2 Replace the status and application Zero masters with the shared normalized compound path while preserving transparent monochrome Template Image behavior and the existing application container.
- [x] 6.3 Regenerate the contact sheet, tray derivatives, and complete application bundle asset set, then review the solid egg at every required size.
- [x] 6.4 Re-run icon, frontend, Rust, build, OpenSpec, and diff checks; record any native runtime review left for the user.
- [x] 6.5 Enlarge the shared solid egg foreground by approximately 10 percent vertically and 20 percent horizontally for a fuller "fat duck egg" within the existing status and application canvases, regenerate derivatives, and re-run focused verification.
