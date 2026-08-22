## Context

Zero already has a deterministic icon pipeline. Canonical `currentColor` SVGs live under `src/assets/icons/`; `scripts/icon-system.mjs` validates them, builds a light/dark contact sheet, rasterizes 18×18 RGBA tray derivatives, and regenerates the application bundle assets. Rust embeds the tray PNGs and marks both individual and grouped macOS status items as template images. React status previews consume the same SVG filenames through CSS masks.

Three visible identities need refinement. `zero.svg` and `zero-app-icon.svg` use a circle even though the intended Zero metaphor is a duck egg. The main-window header and About panel bypass the canonical artwork and render a text `Z`. Zero Awake spends almost one third of its vertical canvas on steam, leaving a small cup. Zero Launch uses `>_`, which communicates a terminal more strongly than launching.

The active `design-zero-icon-system` change established the source contract but has not yet been archived because one real Awake menu-bar interaction check remains open. This follow-up keeps that architecture, changes only reviewed geometry and consumers, and records the ordering dependency instead of folding new work into unrelated behavior changes.

## Goals / Non-Goals

**Goals:**

- Make the canonical Zero mark a solid top-narrow, bottom-full duck egg with a negative 45-degree slash on every branded surface.
- Reuse canonical Zero artwork in the main-window header and About panel instead of text glyphs.
- Enlarge the Awake cup by removing steam while preserving its saucer, handle, active-state distinction, and stable bounds.
- Make Launch read as a rocket at 16px, 18px, 22px, and 24px without losing the shared monochrome geometric language.
- Preserve deterministic generation, typed React mapping, native theme adaptation, and real-system verification boundaries.

**Non-Goals:**

- Redesigning Zero Snap, Zero Paper, the generic extension mark, the surrounding main-window header, or About layout.
- Changing status-item order, grouped-cell sizing, actions, hit testing, menus, IPC, serialized icon identifiers, plugin behavior, or window behavior.
- Adding animation, multicolor tray artwork, a runtime SVG renderer, or an icon-library dependency.
- Treating a generated bitmap concept as canonical source artwork.

## Decisions

### 1. Keep code-native SVG masters and refine only three identities

The existing SVG pipeline remains the source of truth. The implementation changes `zero.svg`, `zero-app-icon.svg`, `zero-awake.svg`, `zero-awake-active.svg`, and `zero-launch.svg`, then regenerates all dependent outputs. Snap, Paper, and extension sources remain byte-for-byte unchanged.

An external icon library can provide metaphor references, but it will not be added as a dependency or copied as the delivered identity. A bespoke rocket is justified because Zero must preserve its exact 24×24 grid, two-unit optical stroke, clear space, Template Image derivatives, and stable source filenames. This also avoids mixing one library glyph into an otherwise owned brand family.

Alternatives considered:

- AI-generated bitmap concepts are not deterministic, do not provide exact vector geometry, and conflict with the repository's SVG source-of-truth workflow.
- A library rocket would be quick to adopt but would import a foreign optical grid or require maintaining an isolated copied shape.
- Parallel CSS-drawn marks would recreate the drift the icon system was introduced to remove.

### 2. Define the Zero foreground as one solid asymmetric egg with a negative diagonal

The 24×24 status master uses a centered Bézier silhouette that is narrower in its upper half and fuller toward its base. The reviewed final envelope expands the egg about 10 percent vertically and 20 percent horizontally for a larger, deliberately fat duck-egg presence while retaining about two units of outer clear space. A rounded transparent cut runs bottom-left to top-right at exactly 45 degrees. The compound `evenodd` path creates figure-ground depth while remaining one monochrome Template Image source with no gradient or shadow.

The 512×512 application master keeps the current near-black rounded container and scales the same normalized compound egg path. The main-window and About containers likewise keep their current size and layout, replace literal `Z` text with the canonical `zero.svg` mask, and size the mark through CSS rather than embedding another path.

Alternatives considered:

- A symmetric vertical oval reads as the numeral `0` and does not express the reviewed duck-egg identity strongly enough.
- A stroked outline plus a positive slash has less visual mass and can look like two unrelated pieces rather than one object.
- Removing the application-icon container would reduce dock and launcher presence and broaden the requested scope into platform icon-mask behavior.

### 3. Reallocate Awake's steam space to the cup

Both Awake sources remove the steam path. The cup body moves upward and grows to occupy the available vertical span while retaining the handle on the right and the horizontal saucer below. The base and active sources share identical outer geometry; the active source adds only one liquid-level stroke inside the cup. That state stroke remains away from the outer contour so it cannot change the raster bounds, grouped status-item position, or hit target.

The final cup should occupy roughly 70 percent of the 24×24 width and reach into the upper clear-space region previously reserved for steam, while retaining at least two units around the complete handle/saucer silhouette.

Alternatives considered:

- Keeping a shorter or straighter steam mark does not solve the user's request or materially enlarge the cup.
- A filled cup makes state contrast strong but departs from the shared open-stroke family.
- Removing the saucer would create more space but discards a requested part of the established metaphor.

### 4. Use a sparse 45-degree rocket, not a detailed spacecraft

Launch uses a rocket aimed toward the upper right so its overall motion follows the family's existing 45-degree geometry. The mark consists of a pointed body outline, minimal fin cues, and one short exhaust stroke. A porthole is optional only if it survives 16px raster review without merging; flame texture, multiple exhaust trails, stars, and filled gradients are prohibited.

The body is visually centered despite its diagonal direction, uses the same two-unit rounded stroke as the family, and stays inside the same clear-space envelope as the other status icons. Recognition is evaluated from the smallest raster first, not from a zoomed SVG.

Alternatives considered:

- A play triangle communicates media playback rather than application launch.
- A lightning bolt communicates speed or power but not launching.
- A detailed rocket is literal at large size but turns into an indistinct blob at 16px to 18px.

### 5. Reuse current asset identifiers and native Template Image composition

No enum, manifest, IPC, or frontend icon identifier changes. Existing `zero`, `launch`, `caffeine-empty`, and `caffeine-full` mappings retain their filenames. Rust continues to load embedded 18×18 transparent RGBA derivatives and calls `icon_as_template(true)` or `set_icon_with_as_template(..., true)` for the grouped item. React continues using `currentColor` masks, including the new main-window and About consumers.

Automated tests assert source inventory, the shared asymmetric solid egg and negative-slash structure, absence of Awake steam, base/active geometry equivalence, Launch rocket cues, generated formats, unchanged mappings, and removal of textual `Z` brand marks. The contact sheet remains the optical review surface. Real macOS verification covers light/dark appearances, grouped item composition, and Awake base/active switching because builds and browser previews cannot establish `SystemUIServer` behavior.

## Risks / Trade-offs

- [Risk] The egg becomes too literal or the solid halves merge at status size. → Mitigation: keep the silhouette geometric, preserve the wide negative slash, and compare status, header, About, and bundle foregrounds together from 16px upward.
- [Risk] Rocket fins or exhaust merge at 16px. → Mitigation: start from the 16px raster, remove optional detail before changing the shared stroke width, and require clear recognition at every target size.
- [Risk] The enlarged Awake handle or saucer clips in the 18×18 derivative. → Mitigation: preserve the two-unit envelope, validate alpha bounds, and review base and active overlays at 1× and 2×.
- [Risk] SVG and generated PNG, ICNS, ICO, mobile, or Windows tile assets drift. → Mitigation: regenerate through the existing commands and validate all tracked Tauri paths in one change.
- [Risk] A monochrome PNG is not treated as a macOS template after grouped composition. → Mitigation: retain explicit template flags and verify the real status bar in light and dark appearances.
- [Risk] The new delta is archived before its active base capability exists in main specs. → Mitigation: keep both changes active during implementation and synchronize or archive `design-zero-icon-system` before archiving this follow-up.

## Migration Plan

1. Add failing structural/source-contract tests for the three new geometries and canonical main-surface reuse.
2. Refine the three 24×24 status masters and the 512×512 application master within the existing filenames.
3. Replace main-window and About text marks with the canonical Zero mask without changing their layout containers.
4. Generate and review the contact sheet at all target sizes; adjust only canonical SVG geometry.
5. Regenerate tray and application bundle assets through the existing scripts.
6. Run focused and full frontend/Rust/build verification, then perform the real macOS light/dark and Awake state smoke pass.
7. Before eventual archive, synchronize the base `zero-icon-system` capability from `design-zero-icon-system`; do not archive either change as part of this proposal.

Rollback restores the previous five SVG masters and regenerated outputs together. Since identifiers and runtime contracts do not change, rollback requires no data or preference migration.

## Open Questions

None. Half-grid optical adjustments to the proposed egg, cup, or rocket are implementation review details only and may not change their specified metaphors, clear-space contract, or small-size acceptance criteria.
