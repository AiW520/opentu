## ADDED Requirements

### Requirement: Media Library Must Store Local Assets Durably
The media library SHALL persist local uploads as metadata plus durable media content without relying on transient object URLs.

#### Scenario: Web local image upload
- **GIVEN** the user uploads a supported image in the web runtime
- **WHEN** the upload completes
- **THEN** the asset metadata SHALL be stored durably
- **AND** the binary content SHALL be retrievable for preview and AI reference use

#### Scenario: Desktop native asset import
- **GIVEN** the user selects a supported media file through the desktop native picker
- **WHEN** the import completes
- **THEN** the file SHALL be copied or materialized under the configured media root
- **AND** asset metadata SHALL include the local file path and content hash

### Requirement: Media Library Import Must Validate Before Durable Writes
The media library SHALL validate supported media type and file safety before committing imported files to durable storage.

#### Scenario: Unsupported desktop file is selected
- **GIVEN** the user selects an unsupported file through the desktop native picker
- **WHEN** the import command processes the file
- **THEN** the file SHALL NOT be committed into the media root
- **AND** no orphan asset metadata SHALL be created

#### Scenario: Import metadata persistence fails
- **GIVEN** a local media file has been copied to a temporary import path
- **WHEN** frontend metadata persistence fails
- **THEN** the temporary file SHALL be removed or marked for cleanup
- **AND** the media library SHALL not show a broken asset entry

### Requirement: Media Library Preview Must Handle Large Desktop Assets
The media library SHALL preview large desktop assets without requiring an unbounded full-file read.

#### Scenario: Large image asset preview
- **GIVEN** a desktop image asset larger than the full-response threshold
- **WHEN** the user views it in the media library
- **THEN** the preview SHALL either load through a bounded protocol path or show a clear preview-unavailable state
- **AND** it SHALL NOT repeatedly request a full response that is known to fail

### Requirement: AI Input Local Images Must Avoid Early Base64 Inflation
The AI input bar SHALL avoid converting local images to base64 before submission when a stable local or virtual reference can be retained.

#### Scenario: Desktop image selected for AI input
- **GIVEN** the user selects a local desktop image as an AI reference
- **WHEN** the image is added to the AI input bar
- **THEN** the input state SHOULD keep a lightweight local reference
- **AND** provider-specific binary or base64 payloads SHALL be created only during execution

#### Scenario: Browser image fallback
- **GIVEN** the runtime cannot provide a stable local reference
- **WHEN** the user uploads a small browser image
- **THEN** the current browser-compatible base64 fallback MAY be used within configured size limits
