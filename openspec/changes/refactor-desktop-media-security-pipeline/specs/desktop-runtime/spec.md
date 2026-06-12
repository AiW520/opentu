## ADDED Requirements

### Requirement: Desktop File Commands Must Enforce Explicit User Intent
The desktop runtime SHALL reject renderer-requested local file operations unless the target path is under the canonical media root or covered by a short-lived user-granted dialog capability.

#### Scenario: Arbitrary write path is rejected
- **GIVEN** renderer code calls a desktop write command with a path that was not returned by the save dialog
- **AND** the path is outside the configured media root
- **WHEN** the command executes
- **THEN** the command SHALL fail before creating or overwriting the file

#### Scenario: Dialog save path is accepted
- **GIVEN** the user selected a destination through the native save dialog
- **WHEN** renderer code writes the matching payload to that granted path
- **THEN** the command SHALL complete
- **AND** the grant SHALL NOT allow unrelated paths

#### Scenario: Media-root operation is accepted
- **GIVEN** a local media operation targets a canonical path under the configured media root
- **WHEN** the command validates the path
- **THEN** the command SHALL allow the operation if the media-specific validation also passes

### Requirement: Desktop Runtime Permissions Must Be Minimal
The desktop runtime SHALL expose only permissions required by implemented desktop features.

#### Scenario: Unused powerful permission is absent
- **WHEN** the desktop capability manifest is built
- **THEN** unused process and filesystem permissions SHALL NOT be enabled
- **AND** opener permissions SHALL be scoped to the minimum required URL/path patterns

### Requirement: Desktop CSP Must Reduce Renderer Blast Radius
The desktop app SHALL use a content security policy that avoids broad script execution and unnecessary network wildcards.

#### Scenario: Production desktop CSP is inspected
- **WHEN** the production desktop configuration is loaded
- **THEN** `unsafe-eval` SHALL NOT be present unless a documented dependency requires it
- **AND** broad `http:` access SHALL be limited to documented local asset or development needs

### Requirement: Desktop Asset Protocol Must Stay Under Media Root
The desktop asset protocol SHALL only serve files whose canonical paths are under the configured media root.

#### Scenario: Encoded path escapes media root
- **GIVEN** an `opentu-asset` request encodes a file path outside the media root
- **WHEN** the protocol handler resolves the request
- **THEN** it SHALL return a forbidden response
- **AND** it SHALL NOT read the target file

#### Scenario: Large media range request succeeds
- **GIVEN** a large local media file under the media root
- **WHEN** the client requests a valid byte range through `opentu-asset`
- **THEN** the protocol handler SHALL return partial content with correct range headers
- **AND** memory usage SHALL be bounded by the requested range size

### Requirement: Desktop Service Worker Policy Must Be Explicit
The desktop runtime SHALL either bundle the required Service Worker contract or disable Service Worker registration with a documented fallback.

#### Scenario: Desktop build starts without silent missing SW
- **WHEN** the desktop renderer boots
- **THEN** it SHALL NOT repeatedly attempt to register a missing `sw.js`
- **AND** task/cache features SHALL use the configured desktop policy
