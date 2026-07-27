## Context

Zero is a tray-first Tauri 2 application whose visible identity spans very different rendering paths:

- Tauri bundle icons are raster/native files under `src-tauri/icons/`.
- Native status items currently generate 18×18 RGBA pixels in Rust and opt into Tauri's template-icon behavior.
- React status-bar previews currently recreate the marks with CSS pseudo-elements, including a text `Z` for Zero.
- The product family now consists of Zero, Zero Launch, Zero Snap, Zero Awake, and Zero Paper, but there is no canonical source artwork for those five identities.

The icon system must remain legible in a macOS menu bar, adapt to light and dark system themes, remain useful on Windows, and scale to a high-resolution application icon. It must not rely on a font, emoji, third-party icon package, or runtime SVG library.

## Goals / Non-Goals

**Goals:**

- Create a recognizable family of five exact, standalone SVG masters.
- Make every status-compatible source use a 24×24 view box, transparent background, and `currentColor`.
- Preserve a crisp silhouette at 16px, 18px, 22px, and 24px.
- Derive native status images, React previews, and bundle artwork from canonical sources rather than parallel hand-drawn implementations.
- Preserve existing status-item actions and add icon identifiers only where Zero Launch and Zero Paper need first-party marks.
- Provide deterministic application icon generation and real macOS light/dark verification.

**Non-Goals:**

- Changing plugin behavior, status-item ordering, click actions, IPC payloads, or window behavior.
- Introducing a general design system, color palette, illustration set, or animation language.
- Giving third-party plugins arbitrary SVG execution in the native host.
- Replacing platform icon-generation requirements with runtime SVG rendering.
- Adding a third-party icon or rasterization dependency to the application.

## Decisions

### 1. Use one geometric grammar for the complete family

All status-compatible masters use:

- `viewBox="0 0 24 24"`
- a transparent canvas
- `fill="none"` by default
- `stroke="currentColor"`
- `stroke-width="2"`
- `stroke-linecap="round"`
- `stroke-linejoin="round"`
- primary geometry aligned to whole or half coordinates
- a practical two-unit clear space around the outer silhouette

The two-unit stroke provides enough weight after rasterizing to the native 18×18 status-item size. Rounded ends prevent jagged terminals at small sizes while the mostly orthogonal/45° geometry keeps the Modern Terminal Minimalist character.

Alternatives considered:

- A one-unit stroke looked too fragile after 18×18 rasterization.
- Filled silhouettes were stronger at very small sizes but made the coffee, frame, and terminal prompt feel like unrelated pictograms.
- Per-icon stroke widths improved individual icons but weakened family consistency.

### 2. Adopt these five canonical SVG masters

The code below is the source contract. Implementation may format whitespace but must not substitute fonts, emoji, embedded raster data, filters, or external assets.

#### Zero — terminal reset, empty state, and precise identity

The circle represents zero, continuity, and a neutral starting point. The exact 45° slash makes it identifiable as `Ø` and adds the decisive cut associated with terminal commands.

```svg
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 24 24"
     fill="none"
     stroke="currentColor"
     stroke-width="2"
     stroke-linecap="round"
     stroke-linejoin="round">
  <circle cx="12" cy="12" r="7.5"/>
  <path d="M5.5 18.5 18.5 5.5"/>
</svg>
```

#### Zero Launch — direct command and immediate execution

The `>` chevron is a terminal prompt and a forward impulse; the `_` is both a cursor and a landing line. Together they communicate search, switch, and launch without a literal rocket.

```svg
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 24 24"
     fill="none"
     stroke="currentColor"
     stroke-width="2"
     stroke-linecap="round"
     stroke-linejoin="round">
  <path d="m5 6 6 6-6 6"/>
  <path d="M13.5 18H19"/>
</svg>
```

#### Zero Snap — a capture region before content exists

Four independent corners imply a selectable screenshot region. The open center stays legible over either system theme and avoids resembling a camera application.

```svg
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 24 24"
     fill="none"
     stroke="currentColor"
     stroke-width="2"
     stroke-linecap="round"
     stroke-linejoin="round">
  <path d="M9 5H5v4"/>
  <path d="M15 5h4v4"/>
  <path d="M19 15v4h-4"/>
  <path d="M9 19H5v-4"/>
</svg>
```

#### Zero Awake — a compact, persistent wake signal

The cup directly connects to the established caffeine metaphor. One continuous steam stroke conveys activity while keeping the menu-bar silhouette calm and sparse.

```svg
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 24 24"
     fill="none"
     stroke="currentColor"
     stroke-width="2"
     stroke-linecap="round"
     stroke-linejoin="round">
  <path d="M10 7c-2-2 2-2 0-5"/>
  <path d="M5 10h10v4a5 5 0 0 1-10 0v-4Z"/>
  <path d="M15 11h1.5a2.5 2.5 0 0 1 0 5H15"/>
  <path d="M4 21h14"/>
</svg>
```

The existing enabled/disabled status behavior uses a state derivative rather than a sixth brand mark. The enabled derivative adds a single liquid-level stroke (`M6.5 14h7`) inside the cup; the base icon remains the disabled state. This preserves layout, stroke weight, and metaphor while making the state change visible.

#### Zero Paper — a daily view into a new landscape

The outer rectangle is both a picture frame and a desktop window. The mountain and sun identify wallpaper content while remaining recognizable at status-bar size.

```svg
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 24 24"
     fill="none"
     stroke="currentColor"
     stroke-width="2"
     stroke-linecap="round"
     stroke-linejoin="round">
  <rect x="4" y="5" width="16" height="14" rx="1"/>
  <circle cx="16" cy="9" r="1.5"/>
  <path d="m5 17 5-5 3 3 2-2 4 4"/>
</svg>
```

Alternatives considered:

- A lightning bolt for Launch was faster-looking but less specific to Zero's terminal identity.
- A camera body for Snap became too detailed at 18px.
- A flame or sun for Awake was ambiguous with brightness and theme controls.
- A standalone mountain for Paper lost the wallpaper/window meaning.

### 3. Separate the scalable brand mark from the application-icon container

The canonical `zero.svg` remains background-free and theme-adaptive. The application icon places the same geometry on a restrained near-black container:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
  <rect x="32" y="32" width="448" height="448" rx="104" fill="#111318"/>
  <g fill="none"
     stroke="#FFFFFF"
     stroke-width="34"
     stroke-linecap="round"
     stroke-linejoin="round">
    <circle cx="256" cy="256" r="144"/>
    <path d="M142 370 370 142"/>
  </g>
</svg>
```

The container is intentionally flat rather than glossy. At small bundle sizes, depth effects and gradients create muddy edges; the generous inset and high contrast give the icon a premium, platform-neutral finish. The canonical foreground geometry must remain optically equivalent to the 24×24 mark.

Derived bundle files include the sizes and formats referenced by `tauri.conf.json`, plus a reviewable `app-icon.png` source at 512×512. Existing Tauri tooling should generate `.icns`, `.ico`, Windows tiles, Android, and iOS outputs; generated artifacts are committed only where the repository already tracks them.

Alternatives considered:

- Using the transparent status glyph as the application icon did not provide enough presence in a dock or launcher.
- Adding neon cyan/purple accents contradicted the agreed monochrome language.
- Maintaining a separately redrawn high-resolution `Ø` risked drift between tray and application identity.

### 4. Keep SVG as source of truth and use platform-appropriate derivatives

Proposed source layout:

```text
src/assets/icons/
  zero.svg
  zero-launch.svg
  zero-snap.svg
  zero-awake.svg
  zero-awake-active.svg
  zero-paper.svg
  zero-app-icon.svg

src-tauri/icons/tray/
  zero.png
  zero-launch.png
  zero-snap.png
  zero-awake.png
  zero-awake-active.png
  zero-paper.png
```

Tauri's native tray API consumes image pixels rather than relying on WebView SVG support. Therefore, deterministic transparent RGBA PNG derivatives are produced from the SVG masters at native review sizes. The Rust status-bar service loads those bundled bytes and continues to call `.icon_as_template(true)` for macOS template behavior.

React previews use the SVG sources through a typed icon map and CSS masks with `background-color: currentColor`. This allows external SVG files to inherit the surrounding preview color without duplicating geometry as CSS borders and pseudo-elements.

`StatusBarIconId` gains additive `launch` and `paper` values and maps existing `screenshot`, `caffeine-empty`, and `caffeine-full` values to the Zero Snap and Zero Awake sources. Existing identifiers remain readable so plugin manifests and persisted records do not require migration.

Alternatives considered:

- Rendering SVG at runtime inside Rust would add parsing/rasterization complexity and a new dependency.
- Keeping the current pixel canvas would make the published SVGs documentation-only and allow native artwork to drift.
- Importing raw SVG as React components would require a Vite transform dependency; CSS masks work with the current toolchain.

### 5. Validate source structure, raster output, and real system behavior separately

Automated checks validate:

- each canonical status SVG parses as XML;
- the view box is exactly `0 0 24 24`;
- drawing color is expressed only through `currentColor`;
- status sources contain no opaque background, font, text, filter, external URL, or embedded raster payload;
- the expected icon IDs resolve to canonical assets;
- Rust tray PNG derivatives are RGBA with transparency;
- frontend previews map to the same semantic source names;
- application bundle icon files remain valid for Tauri compilation.

Visual review renders a contact sheet at 16px, 18px, 22px, 24px, 128px, and 512px on both light and dark backgrounds. Native smoke verification must then confirm the primary and plugin items in the real macOS menu bar, including automatic template recoloring and the Zero Awake state transition.

Compilation or browser-only preview does not count as macOS Template Image verification.

## Risks / Trade-offs

- [Risk] Fine geometry closes or blurs at 16–18px → Mitigation: keep two-unit strokes, round joins, sparse interior detail, and inspect raster contact sheets at 1× and 2×.
- [Risk] SVG and committed PNG derivatives drift → Mitigation: document one generation command, add structural checks, and review generated diffs together with source changes.
- [Risk] `currentColor` does not propagate through an `<img>` URL → Mitigation: render React previews through CSS masks or inline markup from trusted bundled sources.
- [Risk] macOS treats a tray image as a colored bitmap instead of a template → Mitigation: use monochrome alpha artwork, keep transparent pixels, call `.icon_as_template(true)`, and verify in both appearances.
- [Risk] The application icon appears overly sparse on Windows or Linux → Mitigation: use the high-contrast dark container for bundle icons while keeping status artwork transparent.
- [Risk] Added `StatusBarIconId` values affect manifest parsing → Mitigation: make the enum addition backward-compatible and retain all current serialized names.

## Migration Plan

1. Add the reviewed SVG masters and icon-system documentation.
2. Add deterministic native tray derivatives and replace the procedural `IconCanvas` artwork.
3. Extend typed icon mappings for Zero Launch and Zero Paper while preserving current serialized identifiers.
4. Replace CSS pseudo-element previews with the canonical SVG mask mapping.
5. Generate the 512×512 application source and existing Tauri bundle outputs.
6. Run structural, frontend, Rust, bundle, and contact-sheet checks.
7. Smoke-test real macOS menu-bar recoloring and Awake state changes before removing obsolete pixel/CSS implementations.

Rollback keeps the prior bundle assets and `IconCanvas` implementation available in version control. If native raster output is not stable, the SVG masters and React previews can land independently while the native swap is deferred.

## Open Questions

- Whether Zero Launch and Zero Paper should become native status items by default remains owned by the existing status-bar feature; this change only makes their first-party icon IDs available.
- A future contributor-icon policy may allow third-party SVGs, but v1 continues to use the generic trusted `extension` mark.
