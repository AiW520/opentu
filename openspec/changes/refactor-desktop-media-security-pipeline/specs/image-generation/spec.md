## ADDED Requirements

### Requirement: Image References Must Materialize Safely For Providers
Image generation SHALL convert local, virtual, and desktop asset references into provider-compatible payloads only at the execution boundary.

#### Scenario: Asset-library reference is submitted
- **GIVEN** an image generation request includes a reference image from the asset library
- **WHEN** the task executes
- **THEN** the system SHALL resolve the reference through the unified cache or desktop media helpers
- **AND** the provider request SHALL receive a valid URL, multipart file, or base64 payload according to the selected request schema

#### Scenario: Desktop asset reference is submitted
- **GIVEN** an image generation request includes a desktop `opentu-asset` reference
- **WHEN** the task executes
- **THEN** the system SHALL read the local media through the approved desktop asset path
- **AND** it SHALL NOT expose arbitrary filesystem reads to the renderer

#### Scenario: Multiple large references are submitted
- **GIVEN** an image generation request includes multiple local reference images
- **WHEN** the task materializes those references
- **THEN** conversions SHALL run with bounded concurrency
- **AND** oversized images SHALL be compressed before base64 payload creation when the provider accepts compressed image input

#### Scenario: Task record is persisted after local reference submission
- **GIVEN** an image generation request used local or desktop asset references
- **WHEN** the task is persisted after submission
- **THEN** the durable task record SHALL keep lightweight references and route metadata
- **AND** it SHALL NOT retain full base64 payloads, arbitrary filesystem paths, or full Blob contents when those are no longer required for retry

### Requirement: Image Response Parsing Must Preserve URL And Base64 Compatibility
Image generation SHALL accept provider responses that return either URL or base64 image outputs.

#### Scenario: Provider returns URL output
- **GIVEN** an image provider response contains `data[].url` or an equivalent URL field
- **WHEN** the response is parsed
- **THEN** the generated asset result SHALL preserve the URL

#### Scenario: Provider returns base64 output
- **GIVEN** an image provider response contains `data[].b64_json`
- **WHEN** the response is parsed
- **THEN** the generated asset result SHALL normalize it to a data URL or durable cached media URL

### Requirement: GPT Image Response Format Must Stay Schema-Aware
Official GPT Image request serialization SHALL NOT add unsupported default `response_format` fields.

#### Scenario: Official GPT Image generation without explicit response format
- **GIVEN** the selected request schema is official GPT Image generation
- **AND** the user did not explicitly set `response_format`
- **WHEN** the request is serialized
- **THEN** `response_format` SHALL be omitted

#### Scenario: Explicit supported response format
- **GIVEN** the selected request schema supports `response_format`
- **AND** the user explicitly set a supported response format
- **WHEN** the request is serialized
- **THEN** the supported response format SHALL be included
