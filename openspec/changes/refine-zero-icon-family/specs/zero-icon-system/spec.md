## ADDED Requirements

### Requirement: Branded application surfaces reuse the canonical Zero mark
The system SHALL render the canonical duck-egg Zero foreground in the main-window header and About surface without a text glyph, font dependency, or independently drawn foreground geometry.

#### Scenario: Main window is displayed
- **WHEN** the Zero main window renders its product heading
- **THEN** its icon container displays the same top-narrow solid egg and negative 45-degree slash identity used by the canonical status artwork

#### Scenario: About surface is displayed
- **WHEN** the About surface renders its product mark
- **THEN** it displays the canonical Zero foreground rather than a literal `Z` character

## MODIFIED Requirements

### Requirement: Zero provides five canonical icon identities
The system SHALL provide standalone canonical SVG masters for Zero, Zero Launch, Zero Snap, Zero Awake, and Zero Paper, and each icon SHALL communicate its documented product or tool metaphor without text, emoji, or a font dependency.

#### Scenario: Canonical icon set is inspected
- **WHEN** a maintainer inspects the Zero icon source directory
- **THEN** it contains distinct SVG masters for the solid asymmetric Zero duck egg with a negative 45-degree slash, Launch rocket, Snap capture corners, Awake steam-free coffee cup and saucer, and Paper framed landscape

#### Scenario: Icon meaning is documented
- **WHEN** a contributor reviews an icon master
- **THEN** the icon-system documentation identifies its product name, intended metaphor, and small-size usage

### Requirement: The icon family uses consistent visual geometry
The system SHALL render the five canonical identities with a shared two-unit optical stroke, rounded line caps and joins, restrained interior detail, consistent clear space, and recognizable silhouettes suitable for 16px through 24px display.

#### Scenario: Icons are compared at status size
- **WHEN** all five icons are rendered together at 16px, 18px, 22px, and 24px
- **THEN** their apparent weight, alignment, corner treatment, and surrounding clear space form a cohesive family while the solid Zero egg, Launch rocket, and enlarged Awake cup remain distinct

#### Scenario: Icon is rendered at the smallest target
- **WHEN** a canonical icon is rasterized at 16×16 pixels
- **THEN** its primary metaphor remains distinguishable without clipped outer strokes, merged interior details, or reliance on color

### Requirement: Zero mark scales across status and application contexts
The system SHALL use the same solid, top-narrow and bottom-full duck-egg silhouette with a negative 45-degree slash for the transparent Zero status mark, branded React application surfaces, and the high-resolution Zero application icon.

#### Scenario: Zero appears in a status surface
- **WHEN** the Zero mark is rendered at 24×24 or smaller
- **THEN** it appears as a solid asymmetric egg cut by a clear transparent 45-degree slash

#### Scenario: Zero appears in an application surface
- **WHEN** the main-window header or About surface renders the Zero mark
- **THEN** it uses the canonical solid egg and negative-slash foreground without substituting a text character

#### Scenario: Zero appears as an application icon
- **WHEN** the Zero application icon is rendered at 512×512
- **THEN** it presents the white solid egg and transparent-slash geometry on a high-contrast dark charcoal container with sufficient inset for platform masks

#### Scenario: Foreground masters are compared
- **WHEN** a reviewer overlays normalized status, React, and application-icon foreground geometry
- **THEN** the egg asymmetry, slash angle, centering, and optical weight remain recognizably the same Zero mark

### Requirement: Zero Awake preserves state feedback
The system SHALL provide a steam-free Zero Awake base icon with an enlarged coffee cup, handle, and saucer plus an active-state derivative that preserves the canonical silhouette and indicates keep-awake state without changing status-item position or click target.

#### Scenario: Keep-awake is inactive
- **WHEN** the backend reports Zero Awake as disabled
- **THEN** the status item renders the enlarged steam-free cup and saucer icon

#### Scenario: Keep-awake is active
- **WHEN** the backend reports Zero Awake as enabled
- **THEN** the status item renders the same cup derivative with one bounded liquid-level mark

#### Scenario: Awake state changes
- **WHEN** Zero Awake toggles between inactive and active
- **THEN** the icon changes state while retaining the same outer bounds, optical weight, position, and hit target

### Requirement: Icon delivery is verifiable
The system SHALL provide automated structural checks, visual review outputs, and explicit native verification for canonical and derived icon assets.

#### Scenario: SVG validation runs
- **WHEN** the icon validation check runs
- **THEN** it verifies XML parsing, the exact 24×24 view box, allowed color usage, prohibited content, expected inventory, the solid asymmetric Zero egg and negative 45-degree slash, the absence of Awake steam, base/active equivalence, and Launch rocket cues

#### Scenario: Contact sheet is reviewed
- **WHEN** the icon contact sheet is generated
- **THEN** it shows the complete family on light and dark backgrounds at 16px, 18px, 22px, 24px, 128px, and 512px where applicable

#### Scenario: Native macOS verification is reported
- **WHEN** implementation is considered complete on macOS
- **THEN** the verification record distinguishes automated checks from real menu-bar light/dark, grouped Template Image, and Zero Awake base/active smoke results
