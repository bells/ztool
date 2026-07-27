## ADDED Requirements

### Requirement: Zero provides five canonical icon identities
The system SHALL provide standalone canonical SVG masters for Zero, Zero Launch, Zero Snap, Zero Awake, and Zero Paper, and each icon SHALL communicate its documented product or tool metaphor without text, emoji, or a font dependency.

#### Scenario: Canonical icon set is inspected
- **WHEN** a maintainer inspects the Zero icon source directory
- **THEN** it contains distinct SVG masters for the Zero `Ø`, Launch terminal prompt, Snap capture corners, Awake steaming cup, and Paper framed landscape

#### Scenario: Icon meaning is documented
- **WHEN** a contributor reviews an icon master
- **THEN** the icon-system documentation identifies its product name, intended metaphor, and small-size usage

### Requirement: Status-compatible SVGs follow one source contract
The system SHALL define every status-compatible canonical icon with `viewBox="0 0 24 24"`, a transparent canvas, `currentColor` drawing attributes, and no external runtime dependency.

#### Scenario: System theme changes
- **WHEN** a status-compatible icon is rendered with a different inherited color
- **THEN** all visible icon geometry adopts that color without requiring a different SVG source

#### Scenario: SVG source is used independently
- **WHEN** a consumer copies one canonical status SVG without project styles or scripts
- **THEN** the SVG renders its complete icon using only standard SVG geometry and `currentColor`

#### Scenario: Prohibited content is checked
- **WHEN** canonical status SVGs are validated
- **THEN** they contain no opaque background, font, text, emoji, external URL, filter, or embedded raster image

### Requirement: The icon family uses consistent visual geometry
The system SHALL render the five canonical identities with a shared two-unit optical stroke, rounded line caps and joins, restrained interior detail, and consistent clear space suitable for 16px through 24px display.

#### Scenario: Icons are compared at status size
- **WHEN** all five icons are rendered together at 16px, 18px, 22px, and 24px
- **THEN** their apparent weight, alignment, corner treatment, and surrounding clear space form a cohesive family

#### Scenario: Icon is rendered at the smallest target
- **WHEN** a canonical icon is rasterized at 16×16 pixels
- **THEN** its primary metaphor remains distinguishable without clipped outer strokes or merged interior details

### Requirement: Zero mark scales across status and application contexts
The system SHALL use optically equivalent circular `Ø` geometry with a 45° slash for both the transparent Zero status mark and the high-resolution Zero application icon.

#### Scenario: Zero appears in a status surface
- **WHEN** the Zero mark is rendered at 24×24 or smaller
- **THEN** it appears as a transparent circular ring crossed by a clear 45° slash

#### Scenario: Zero appears as an application icon
- **WHEN** the Zero application icon is rendered at 512×512
- **THEN** it presents the white `Ø` geometry on a high-contrast dark charcoal container with sufficient inset for platform masks

#### Scenario: Foreground masters are compared
- **WHEN** a reviewer overlays normalized status and application foreground geometry
- **THEN** the circle, slash angle, proportion, and optical weight remain recognizably the same Zero mark

### Requirement: Native status assets support platform theme adaptation
The system SHALL derive transparent monochrome native tray assets from the canonical SVGs and SHALL configure macOS status items as template images.

#### Scenario: macOS switches to dark appearance
- **WHEN** the system changes from light to dark appearance while Zero is running
- **THEN** every Zero template status icon automatically changes to the system-appropriate foreground without replacing the canonical source

#### Scenario: macOS switches to light appearance
- **WHEN** the system changes from dark to light appearance while Zero is running
- **THEN** every Zero template status icon remains legible through native template recoloring

#### Scenario: Native derivative is inspected
- **WHEN** a generated tray PNG is decoded
- **THEN** it has an RGBA channel, transparent background pixels, monochrome visible pixels, and dimensions appropriate for the native status item

### Requirement: Zero Awake preserves state feedback
The system SHALL provide a Zero Awake state derivative that preserves the canonical cup silhouette and indicates active keep-awake state without changing status-item position or click target.

#### Scenario: Keep-awake is inactive
- **WHEN** the backend reports Zero Awake as disabled
- **THEN** the status item renders the base steaming-cup icon

#### Scenario: Keep-awake is active
- **WHEN** the backend reports Zero Awake as enabled
- **THEN** the status item renders the active cup derivative with the additional liquid-level mark

#### Scenario: Awake state changes
- **WHEN** Zero Awake toggles between inactive and active
- **THEN** the icon changes state while retaining the same outer bounds, optical weight, position, and hit target

### Requirement: Native and React surfaces share canonical icon semantics
The system SHALL map native status assets and React status/preference previews to the same canonical semantic icon sources instead of independently redrawing first-party marks.

#### Scenario: Status preview is displayed
- **WHEN** React renders a first-party item in the status-bar preview or fallback action row
- **THEN** it resolves the same Zero family icon identity used by the native status item

#### Scenario: Existing icon identifier is loaded
- **WHEN** a plugin record uses an existing `zero`, `screenshot`, `caffeine-empty`, or `caffeine-full` icon identifier
- **THEN** it continues to resolve to the corresponding Zero, Zero Snap, or Zero Awake canonical artwork

#### Scenario: New first-party icon identifier is loaded
- **WHEN** a bundled contribution uses the additive `launch` or `paper` icon identifier
- **THEN** both Rust and TypeScript resolve it to the matching Zero Launch or Zero Paper canonical artwork

### Requirement: Application bundle outputs derive from a reviewable master
The system SHALL provide a reviewable 512×512 application icon source and generate every bundle icon referenced by Tauri configuration from that approved master.

#### Scenario: Desktop bundle is built
- **WHEN** Tauri validates or builds Zero for a supported desktop platform
- **THEN** all referenced PNG, ICNS, ICO, and platform tile assets exist in valid formats

#### Scenario: Application asset is regenerated
- **WHEN** a maintainer runs the documented icon-generation workflow from the approved master
- **THEN** the expected tracked bundle assets are reproduced without introducing a new runtime dependency

### Requirement: Icon delivery is verifiable
The system SHALL provide automated structural checks and visual review outputs for canonical and derived icon assets.

#### Scenario: SVG validation runs
- **WHEN** the icon validation check runs
- **THEN** it verifies XML parsing, the exact 24×24 view box, allowed color usage, prohibited content, and expected icon inventory

#### Scenario: Contact sheet is reviewed
- **WHEN** the icon contact sheet is generated
- **THEN** it shows the complete family on light and dark backgrounds at 16px, 18px, 22px, 24px, 128px, and 512px where applicable

#### Scenario: Native macOS verification is reported
- **WHEN** implementation is considered complete on macOS
- **THEN** the verification record distinguishes automated checks from real menu-bar light/dark and Zero Awake state smoke results
