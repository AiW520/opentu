# Change: Harden desktop media security and file pipeline

## Why
The desktop app has enough structure to continue toward delivery, but the current local-file surface and media pipeline are not ready for high-concurrency desktop use. Renderer-callable file commands are too broad, several media paths still materialize whole files as base64 or JavaScript arrays, and desktop runtime behavior differs from the web cache/task model.

## What Changes
- Add a desktop runtime boundary that constrains local file operations to user-approved paths and removes unnecessary runtime permissions.
- Replace large local media round-trips with bounded, stream-oriented or range-friendly paths.
- Make local asset ingestion validate media type before durable writes and clean up failed imports.
- Ensure AI reference-image handling can safely materialize local/virtual assets only at execution time with bounded memory.
- Decide and enforce desktop Service Worker behavior so cache/task expectations are explicit.
- Keep GPT Image response handling explicit and compatible with URL and base64 outputs.

## Impact
- Affected specs: `desktop-runtime`, `media-library`, `image-generation`
- Affected code:
  - `apps/desktop/src-tauri/src/commands/*`
  - `apps/desktop/src-tauri/capabilities/default.json`
  - `apps/desktop/src-tauri/tauri.conf.json`
  - `apps/desktop/src/main.tsx`
  - `apps/desktop/src/utils/*`
  - `packages/drawnix/src/services/asset-storage-service.ts`
  - `packages/drawnix/src/services/unified-cache-service.ts`
  - `packages/drawnix/src/components/ai-input-bar/AIInputBar.tsx`
  - `packages/drawnix/src/services/media-executor/*`
  - `packages/drawnix/src/services/model-adapters/*`
