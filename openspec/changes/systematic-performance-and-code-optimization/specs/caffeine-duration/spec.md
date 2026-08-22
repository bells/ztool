## ADDED Requirements

### Requirement: Caffeine presentation timing follows surface visibility
The frontend SHALL run elapsed/remaining-time presentation updates only while an Awake surface is visible and caffeine mode is enabled, SHALL stop recurring presentation updates while inactive or hidden, and SHALL refresh the backend-owned snapshot before resuming after reveal.

#### Scenario: Inactive Awake panel is visible
- **WHEN** the Awake panel is visible while caffeine mode is inactive
- **THEN** the frontend displays the inactive controls without running a recurring elapsed-time interval

#### Scenario: Active Awake panel becomes hidden
- **WHEN** caffeine mode is active and the host hides the surface containing its panel
- **THEN** the frontend stops the one-second presentation interval without disabling or extending the native keep-awake session

#### Scenario: Active Awake panel becomes visible again
- **WHEN** a hidden Awake surface is shown while a session may still be active
- **THEN** the frontend reads a fresh Caffeine snapshot and resumes visible time feedback only if the authoritative state remains enabled

### Requirement: Hidden frontend timing does not own caffeine expiry
Finite-session expiry SHALL remain backend-owned and SHALL disable the native keep-awake behavior at the backend-calculated deadline even when every Awake UI surface is hidden or unloaded.

#### Scenario: Finite session expires while all windows are hidden
- **WHEN** the backend-calculated expiry is reached while no Awake panel is visible
- **THEN** the backend disables native keep-awake and the next visible snapshot reports the session inactive without relying on a frontend timer firing
