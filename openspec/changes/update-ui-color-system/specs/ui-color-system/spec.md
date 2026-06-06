## ADDED Requirements

### Requirement: Global UI Colors SHALL Use Shared Tokens

The system SHALL define and use shared UI color tokens for common application surfaces, text, borders, interactive states, focus rings, and brand orange accents so product UI color choices remain consistent across modules.

#### Scenario: Component needs a common UI color

- **WHEN** a component needs colors for surface, text, border, hover, selected, active, disabled, or focus states
- **THEN** it SHALL use the shared UI color tokens
- **AND** it SHALL NOT introduce ad-hoc hard-coded colors when an existing token can express the same intent

#### Scenario: Component needs brand emphasis

- **WHEN** a component needs primary emphasis, selected emphasis, or brand-aligned focus treatment
- **THEN** it SHALL use the brand orange token family
- **AND** it SHALL keep contrast sufficient for readable text and visible controls

### Requirement: Media Library States SHALL Use UI Tokens

The media library SHALL use shared UI tokens for item hover, selected, active, focus, overlay, and action states so asset browsing feedback feels natural and aligned with the global brand color system.

#### Scenario: Media asset is hovered

- **WHEN** a user hovers a media library asset item
- **THEN** the item SHALL use a tokenized subtle hover background or border treatment
- **AND** it SHALL NOT use heavy black or gray overlays as the default hover feedback

#### Scenario: Media asset is selected

- **WHEN** a user selects a media library asset item
- **THEN** the item SHALL use tokenized selected background, border, or indicator treatment based on the brand orange token family
- **AND** selection SHALL remain visually distinct from hover without relying on near-black fills
- **AND** selection SHALL NOT cover the asset preview with full-card white or dark masks

#### Scenario: Media asset receives keyboard focus

- **WHEN** a media library asset item receives keyboard focus
- **THEN** it SHALL use the shared focus token
- **AND** it SHALL NOT introduce an isolated blue focus ring that conflicts with the global UI color system

#### Scenario: Media library shows multiple actions

- **WHEN** the media library presents actions in one toolbar or inspector area
- **THEN** it SHALL keep at most one brand-primary button in that area
- **AND** secondary actions such as download, cancel, delete, sync, or mark subject SHALL use outline, text, danger, or other lower-emphasis treatments

#### Scenario: Media library batch selection uses inspector actions

- **WHEN** a user is in batch selection mode and a right-side inspector is available
- **THEN** the inspector primary action SHALL represent the batch insert/select action
- **AND** the top toolbar SHALL NOT duplicate that same primary batch insert/select button

### Requirement: AI Coding UI Guidance SHALL Prefer Tokens

AI-assisted UI coding guidance SHALL require generated or modified UI code to prefer existing shared UI color tokens before adding new colors, especially for primary actions, hover states, selected states, focus rings, and overlays.

#### Scenario: AI changes UI styling

- **WHEN** AI-generated code changes component colors or interactive states
- **THEN** it SHALL first reuse existing UI color tokens
- **AND** it SHALL document or justify any new token need before adding a new color value

#### Scenario: AI creates a primary action

- **WHEN** AI-generated code creates or updates a primary action button
- **THEN** it SHALL use the brand primary token treatment
- **AND** it SHALL NOT create a black primary button unless an approved design exception exists

#### Scenario: AI creates an overlay or focus style

- **WHEN** AI-generated code creates overlays or focus styles
- **THEN** it SHALL use shared overlay and focus tokens
- **AND** it SHALL NOT introduce heavy black masks or isolated blue focus styles by default
