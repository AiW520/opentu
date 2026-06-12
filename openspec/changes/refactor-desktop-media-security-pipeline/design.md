## Context
The desktop app reuses the web renderer while adding Tauri commands for local storage, downloads, media import, and a custom `opentu-asset` protocol. This makes the product direction viable, but local filesystem access becomes a trust boundary. The implementation must assume renderer code can be compromised by XSS, injected dependencies, or unsafe remote content.

The media pipeline also serves large images, video, audio, ZIP imports, and generated assets. Whole-file base64 conversion and JavaScript array copies are acceptable for small web uploads, but they are not acceptable as a default desktop path.

## Goals
- Constrain desktop file operations to explicit user intent.
- Keep local media operations memory-bounded under large files and batches.
- Preserve current browser behavior where possible.
- Make local asset URLs usable in UI and AI generation without exposing arbitrary filesystem reads.
- Keep GPT Image response behavior compatible with both `url` and `b64_json`.

## Non-Goals
- Rewriting all storage to a new database.
- Adding a new desktop-only media library UI.
- Changing provider profile compatibility semantics beyond safe response/request handling.
- Shipping auto-update signing keys in this change.

## Decisions

### User-approved local file capability
Tauri commands that read, write, move, import, or copy local files must either:
- operate under the configured media root, or
- operate on a path returned by a recent native dialog flow, or
- operate on a save path returned by the save dialog.

Plain renderer-provided arbitrary paths must not be enough for destructive or exfiltrating operations. The implementation can use short-lived in-memory grants keyed by path plus operation type.

### Keep custom protocol constrained to media root
`opentu-asset` remains the runtime display protocol, but it must only serve canonical files under the configured media root. Large file responses must be range-friendly. Full responses above the bounded threshold should either stream safely or return a clear failure that UI can avoid.

### Prefer native desktop import for desktop uploads
Desktop media-library upload should prefer the native picker so file paths are available and large files do not enter the renderer as full `File` blobs. Browser file input remains the fallback for web and unsupported desktop paths.

### Bounded AI materialization
The UI may carry lightweight references such as virtual URLs or desktop asset URLs. Execution code is responsible for converting references into provider-compatible payloads. Conversion must:
- detect local virtual and desktop asset URLs,
- load through approved cache/protocol helpers,
- compress oversized images before base64 conversion,
- avoid storing full base64 strings in durable task metadata after submission.

### Desktop Service Worker policy
The desktop build must choose one explicit path:
- include the same `sw.js` contract as web if desktop task/cache features depend on it, or
- disable SW registration and use desktop-safe fallback services consistently.

Silent missing `sw.js` registration is not acceptable.

### Response format compatibility
Official GPT Image requests should not add unsupported default `response_format` fields. Explicit `response_format` values may be passed when supported, and response parsing must accept both URL and base64 image results.

## Risks
- Short-lived path grants must not persist across app restarts.
- Some existing download/export flows may rely on direct renderer-provided paths and need migration to dialog-backed calls.
- Reducing CSP may require removing inline/eval assumptions from the desktop shell.

## Validation
- Unit tests for path grant validation and denied arbitrary paths.
- Rust tests for protocol range/full response behavior and media-root escape attempts.
- Vitest coverage for local asset reference conversion into AI requests.
- Manual desktop smoke test for upload, preview, select-as-reference, generate, export, and delete.
