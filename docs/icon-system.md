# Zero Icon System

Zero uses a Modern Terminal Minimalist icon language: monochrome geometry, decisive terminal-inspired forms, and no decorative detail that disappears in a menu bar.

## Canonical identities

| Product | Source | Meaning |
| --- | --- | --- |
| Zero | `src/assets/icons/zero.svg` | A zero ring crossed by a precise 45° slash: reset, continuity, and the `Ø` identity. |
| Zero Launch | `src/assets/icons/zero-launch.svg` | A terminal `>` prompt and `_` cursor: search, switch, and immediate execution. |
| Zero Snap | `src/assets/icons/zero-snap.svg` | Four open viewfinder corners: the capture region before content is selected. |
| Zero Awake | `src/assets/icons/zero-awake.svg` | A coffee cup with one steam stroke: caffeine and persistent wakefulness. |
| Zero Paper | `src/assets/icons/zero-paper.svg` | A framed mountain and sun: a daily wallpaper as a view into a landscape. |

`zero-awake-active.svg` is a state derivative, not a sixth identity. It adds one liquid-level stroke while retaining the base cup's outer geometry.

## Geometry

- Status-compatible sources use `viewBox="0 0 24 24"`.
- The canvas is transparent and drawing color comes from `currentColor`.
- The shared optical stroke is `2`, with round caps and joins.
- Primary geometry uses whole or half coordinates and keeps about two view-box units of clear space.
- Required review sizes are 16px, 18px, 22px, and 24px. The 18px rendering is the native tray derivative.
- `zero-app-icon.svg` is the 512×512 application master. It keeps the same `Ø` proportions on a `#111318` container.

The SVG files are the source of truth. Native tray PNGs, the React mask map, the contact sheet, and application bundle icons must be regenerated from these sources rather than redrawn.

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
- macOS Zero Awake menu-bar click and timed expiry: pending a status-item interaction path that can target `SystemUIServer`; automated state mapping and derivative-bound tests do not replace this smoke check.
