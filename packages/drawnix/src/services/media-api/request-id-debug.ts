const LAST_IMAGE_REQUEST_ID_KEY = '__OPENTU_LAST_IMAGE_REQUEST_ID__';
export const IMAGE_REQUEST_ID_EVENT = 'opentu:image-request-id';

type ImageRequestIdDebugGlobal = typeof globalThis & {
  [LAST_IMAGE_REQUEST_ID_KEY]?: string;
};

export interface ImageRequestIdDebugMeta {
  model?: string;
  endpoint?: string;
  source?: string;
}

export function setLatestImageRequestId(requestId: string): void {
  if (!requestId) {
    return;
  }
  const globalScope = globalThis as ImageRequestIdDebugGlobal;
  globalScope[LAST_IMAGE_REQUEST_ID_KEY] = requestId;

  if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
    window.dispatchEvent(
      new CustomEvent(IMAGE_REQUEST_ID_EVENT, {
        detail: { requestId },
      })
    );
  }
}

export function getLatestImageRequestId(): string {
  return (
    (globalThis as ImageRequestIdDebugGlobal)[LAST_IMAGE_REQUEST_ID_KEY] || ''
  );
}

export function emitImageRequestIdDebugLog(
  requestId: string,
  meta: ImageRequestIdDebugMeta = {}
): void {
  if (!requestId) {
    return;
  }

  setLatestImageRequestId(requestId);

  void meta;
  console.info(`[X-Request-Id] ${requestId}`);
}
