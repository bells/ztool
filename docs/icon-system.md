# Zero Icon System

Zero uses a Modern Terminal Minimalist icon language: monochrome geometry, decisive terminal-inspired forms, and no decorative detail that disappears in a menu bar.

## Canonical identities

| Product | Source | Meaning |
| --- | --- | --- |
| Zero | `src/assets/icons/zero.svg` | A solid duck egg, narrow above and full below, cut by a precise negative 45° slash: zero, continuity, and the `Ø` identity. |
| Zero Launch | `src/assets/icons/zero-launch.svg` | A sparse rocket aimed toward the upper right: search, switch, and immediate launch. |
| Zero Snap | `src/assets/icons/zero-snap.svg` | Four open viewfinder corners: the capture region before content is selected. |
| Zero Awake | `src/assets/icons/zero-awake.svg` | An enlarged steam-free coffee cup, handle, and saucer: caffeine and persistent wakefulness. |
| Zero Paper | `src/assets/icons/zero-paper.svg` | A framed mountain and sun: a daily wallpaper as a view into a landscape. |

`zero-awake-active.svg` is a state derivative, not a sixth identity. It adds one liquid-level stroke while retaining the steam-free base cup's outer geometry.

## Geometry

- Status-compatible sources use `viewBox="0 0 24 24"`.
- The canvas is transparent and drawing color comes from `currentColor`.
- The shared optical stroke is `2`, with round caps and joins.
- Primary geometry uses whole or half coordinates and keeps about two view-box units of clear space.
- The Zero foreground uses one centered solid Bézier egg, enlarged to a fuller "fat duck egg" envelope while remaining narrower above and heavier below, with a rounded transparent bottom-left to top-right 45° cut. The same normalized compound path is used by status, React, and application masters.
- Launch follows that diagonal direction with a sparse body, two fin cues, and one exhaust stroke; Awake reallocates the former steam area to its cup body.
- Required review sizes are 16px, 18px, 22px, and 24px. The 18px rendering is the native tray derivative.
- `zero-app-icon.svg` is the 512×512 application master. It keeps the same `Ø` proportions on a `#111318` container.

The SVG files are the source of truth. Native tray PNGs, the React mask map, the main-window and About marks, the contact sheet, and application bundle icons must be regenerated or rendered from these sources rather than redrawn. React brand surfaces use the canonical `zero.svg` CSS mask and never substitute a text `Z`.

`extension.svg` is the trusted generic fallback for third-party plugins. It follows the same geometry contract but is not part of the five first-party identities.

## Commands

```sh
pnpm icons:validate
pnpm icons:contact-sheet
pnpm icons:generate-tray
pnpm icons:generate-app
```

- `icons:validate` performs dependency-free structural checks.
- `icons:contact-sheet` writes `docs/assets/zero-icon-contact-sheet.svg`.
- `icons:generate-tray` uses the existing Tauri CLI renderer and writes transparent RGBA derivatives under `src-tauri/icons/tray/`.
- `icons:generate-app` uses the same Tauri CLI to regenerate the tracked bundle assets and writes `src-tauri/icons/app-icon.png`.

After generation, review the contact sheet at 1× and 2×, then verify the real macOS status items in both appearances. A browser preview or successful build does not establish Template Image behavior.

## Verification record

- macOS dark and light appearances: verified in a real Tauri session with all five Zero family status items visible, unclipped, and rendered with the system-selected template foreground.
- React fallback action row: verified with Windows-shaped host data in a real browser render. The glyph mask inherits its parent foreground through `color: inherit` and `background-color: currentColor`; both dark-on-light and light-on-dark foregrounds were checked.
- macOS Zero Awake menu-bar interaction: verified through the real `SystemUIServer` status item. Direct on/off clicks changed only the bounded liquid-level state, with no clipping, cell movement, or hit-target regression.
- macOS Zero Awake timed expiry: verified with a real five-minute session. The active cup returned to the inactive cup automatically at expiry while retaining the same outer bounds and status-item position.
- Revised solid Zero egg: source, generated-asset, and light/dark contact-sheet review passed at 16px, 18px, 22px, 24px, 128px, and 512px. Fresh Dock, main-window, and `SystemUIServer` review of this final asymmetric refinement is delegated to the user.
- Windows runtime rendering remains unrun; generated Windows assets, React fallback masks, and cross-platform builds are covered by automated checks only.
