## 1. Desktop Security Boundary
- [ ] 1.1 Audit renderer-callable Tauri commands and classify them as read, write, move, delete, import, protocol, or dialog.
- [x] 1.2 Add short-lived dialog-backed path grants for file open/import and save operations.
- [x] 1.3 Reject arbitrary renderer-provided paths unless they are under the canonical media root or covered by a valid grant.
- [x] 1.4 Remove unused or overly broad desktop permissions and plugin registrations.
- [ ] 1.5 Tighten desktop CSP and document any required exceptions.

## 2. Media Library Import And Preview
- [x] 2.1 Prefer native desktop picker for desktop media-library imports.
- [x] 2.2 Validate media type and extension before copying into the durable media directory.
- [ ] 2.3 Clean up temporary files if frontend validation or metadata persistence fails.
- [x] 2.4 Make `opentu-asset` image/video/audio serving range-friendly for large files.
- [ ] 2.5 Add UI fallback messaging for local assets that cannot be previewed.

## 3. Memory-Bounded AI Reference Handling
- [x] 3.1 Stop converting AI input uploads to base64 earlier than required when a stable local reference is available.
- [x] 3.2 Extend virtual/local asset detection to include desktop asset URLs where AI execution needs conversion.
- [ ] 3.3 Convert local references to compressed provider payloads at execution time with bounded concurrency.
- [ ] 3.4 Strip base64/reference payloads from in-memory and persisted task records after submission where safe.

## 4. Desktop Runtime Parity
- [x] 4.1 Decide whether desktop bundles `sw.js` or disables SW registration.
- [x] 4.2 Implement the chosen desktop SW/cache/task policy.
- [ ] 4.3 Align desktop cache recovery and task execution paths with that policy.
- [x] 4.4 Handle updater configuration explicitly: signed updater or no updater permission.

## 5. Response Compatibility
- [ ] 5.1 Preserve current GPT Image behavior: no unsupported default `response_format`.
- [ ] 5.2 Add explicit `response_format` plumbing only where the selected request schema supports it.
- [ ] 5.3 Keep response parsing compatible with `data[].url`, `data[].b64_json`, and provider gateway variants.

## 6. Verification
- [ ] 6.1 Add Rust tests for denied arbitrary paths, media-root escape attempts, and granted import/save paths.
- [x] 6.2 Add Rust tests for large `opentu-asset` range requests and non-range behavior.
- [x] 6.3 Add Vitest tests for desktop/local asset URLs used as image references.
- [x] 6.4 Run targeted Vitest and Cargo tests.
- [ ] 6.5 Run a desktop smoke test covering import, preview, select from library, AI generation, export, and delete.
