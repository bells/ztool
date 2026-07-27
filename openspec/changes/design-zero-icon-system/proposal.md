## Why

Zero has adopted a cohesive product family name, but its current application, status-bar, and plugin glyphs are a mix of generated pixels, CSS shapes, text, and legacy bundle assets. A single Modern Terminal Minimalist icon system is needed now so the Zero brand remains recognizable from a 24×24 menu-bar glyph through a 512×512 application icon.

## What Changes

- Establish a monochrome geometric icon language for Zero based on clean strokes, restrained detail, consistent optical weight, and transparent backgrounds.
- Design five standalone, dependency-free SVG masters using `viewBox="0 0 24 24"` and `currentColor`:
  - Zero: a circular `Ø` mark with a 45° slash.
  - Zero Launch: a terminal prompt combining `>` and `_`.
  - Zero Snap: four aligned viewfinder corners defining a capture region.
  - Zero Awake: a minimal coffee cup and single steam stroke, with a compatible active-state treatment for the existing status item.
  - Zero Paper: a rectangular frame containing a minimal mountain and sun.
- Define shared geometry, stroke, cap, join, clear-space, and small-size rules so all five icons read as one family at 16–24px.
- Use the same SVG source geometry for status-bar assets and React previews instead of maintaining separate pixel-drawn and CSS-drawn interpretations.
- Produce platform-ready application icon outputs from a high-resolution Zero master: a dark charcoal background with a white `Ø`, including the PNG and native bundle formats expected under `src-tauri/icons/`.
- Ensure tray/status-bar artwork has transparency and monochrome alpha suitable for macOS Template Image behavior; keep Tauri tray construction configured as template artwork on macOS.
- Document each icon's visual metaphor and provide the exact standalone SVG source without third-party icon or runtime dependencies.

## Capabilities

### New Capabilities

- `zero-icon-system`: Defines the Zero brand mark, four bundled-plugin glyphs, SVG source contract, platform asset derivation, native status-bar behavior, and visual/technical acceptance criteria.

### Modified Capabilities

None. The active `zero-brand-identity` and `status-bar-plugin-icons` changes establish naming and status-bar behavior; this change supplies the dedicated visual asset system they consume without changing their behavioral contracts.

## Impact

- SVG sources and documentation for the Zero mark and four bundled plugins.
- `src-tauri/icons/` application and tray asset outputs, including platform bundle formats.
- Rust status-bar icon loading/rendering in `src-tauri/src/services/status_bar.rs`, while preserving existing `StatusBarIconId` and click-action contracts.
- React status-bar and preferences previews currently rendered through `StatusBarGlyph` and CSS pseudo-elements.
- Build/review workflow for generating and visually verifying derived raster assets at 16px, 18px, 22px, 24px, and 512px.
- No new runtime dependency, network service, IPC payload, plugin API, or functional plugin behavior is introduced.
