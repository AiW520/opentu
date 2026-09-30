## ADDED Requirements

### Requirement: MiniMax-H3 V2 Routing

The system SHALL use the V2 generation and query endpoints for MiniMax-H3 without exposing an API version selector, while retaining legacy V1 compatibility code.

#### Scenario: Ignore stale version preferences

- **GIVEN** a MiniMax-H3 task with no API version or a historical api_version=v1
- **WHEN** it is submitted or queried
- **THEN** generation SHALL use POST /v2/video_generation
- **AND** polling SHALL use GET /v2/query/video_generation/{task_id}
- **AND** the parameter panel SHALL not show an API version control

#### Scenario: Preserve other models

- **WHEN** a different video model is used
- **THEN** its existing request, polling and parameter behavior SHALL remain unchanged

### Requirement: Automatic Context IR Enhancement

The system SHALL provide a default-off enhancement switch and automatically generate video after successful Context IR processing without a separate enhancement dialog.

#### Scenario: Enabled enhancement

- **WHEN** a user submits MiniMax-H3 with enhancement enabled
- **THEN** the system SHALL submit POST /v2/h3_context_ir with model MiniMax-H3
- **AND** query its task through the V2 query endpoint
- **AND** use task.content.prompt for video generation exactly once
- **AND** the main input bar SHALL append an output-language instruction matching the interface language without translating the result

#### Scenario: Disabled or failed enhancement

- **WHEN** enhancement is disabled
- **THEN** the system SHALL not submit Context IR
- **WHEN** Context IR returns an error, fails, times out, or completes without a prompt
- **THEN** the system SHALL report failure and SHALL NOT submit video generation

#### Scenario: Provider group independence

- **WHEN** any configured provider group is selected
- **THEN** the system SHALL allow the request and preserve provider errors without a local official-group restriction

### Requirement: MiniMax-H3 2K Regeneration

The system SHALL support regeneration of completed 768P MiniMax-H3 tasks using the source remote task ID and original provider.

#### Scenario: Eligible source

- **GIVEN** a completed MiniMax-H3 768P task with a nonempty remote ID
- **WHEN** the user requests an upgrade
- **THEN** the request SHALL use POST /v2/video_regeneration with model, source_task_id and resolution=2K
- **AND** the task SHALL use existing asynchronous polling, result storage and failure handling
- **AND** progress and completion synchronization SHALL preserve remoteId and invocationRoute

#### Scenario: Ineligible source

- **GIVEN** a source that is already 2K, is not 768P, is incomplete, or lacks a remote ID
- **WHEN** upgrade availability is computed or the business action is invoked
- **THEN** the system SHALL reject the action with a clear reason

### Requirement: MiniMax-H3 Reference Videos

The system SHALL support MiniMax-H3 reference-to-video input without persisting inline video data in the workflow.

#### Scenario: Submit local reference videos

- **GIVEN** up to three cached local MP4 or MOV assets
- **WHEN** the user submits MiniMax-H3 generation or Context IR
- **THEN** the system SHALL read each asset from the local media cache at request time
- **AND** encode it as a video Data URL in content[].video_url with role=reference_video
- **AND** the persisted workflow SHALL retain only the lightweight virtual asset URL

#### Scenario: Submit public reference videos

- **GIVEN** a public HTTP(S) reference video URL
- **WHEN** the user submits MiniMax-H3
- **THEN** the system SHALL attempt to read MP4/MOV content and encode it as a video Data URL
- **AND** preserve the original URL if reading fails or the media type does not match

#### Scenario: Enforce inline request limits

- **WHEN** more than three reference videos are selected, a local video is not MP4/MOV, local video cache is missing, local videos exceed 47 MiB combined, or the serialized request exceeds 64 MiB
- **THEN** the system SHALL reject the request before provider traffic
- **AND** show a clear error that directs oversized local videos to a public URL

#### Scenario: Preserve unsupported-model validation

- **WHEN** video references are attached to a model without video-reference capability
- **THEN** the existing capability validation SHALL reject the submission
- **AND** supported non-H3 video models SHALL retain their existing behavior
