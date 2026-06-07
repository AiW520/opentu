import { unifiedCacheService } from '@drawnix/drawnix';

const ASSET_LIBRARY_URL_PREFIX = '/asset-library/';
const CACHE_URL_PREFIX = '/__aitu_cache__/';
const AI_GENERATED_URL_PREFIX = '/__aitu_generated__/';
const AI_GENERATED_AUDIO_URL_PREFIX = `${AI_GENERATED_URL_PREFIX}audio/`;

let isInitialized = false;
let fetchPatched = false;

const objectUrls = new WeakMap<Element, string>();

function normalizeVirtualMediaUrl(url: string): string {
  if (!url) return url;
  try {
    return new URL(url, 'http://aitu.local').pathname;
  } catch {
    return url;
  }
}

function isVirtualMediaUrl(url: string): boolean {
  const normalized = normalizeVirtualMediaUrl(url);
  return (
    normalized.startsWith(ASSET_LIBRARY_URL_PREFIX) ||
    normalized.startsWith(CACHE_URL_PREFIX) ||
    normalized.startsWith(AI_GENERATED_AUDIO_URL_PREFIX)
  );
}

export function initializeVirtualUrlInterceptor(): void {
  if (isInitialized) return;
  isInitialized = true;

  patchFetchForVirtualMedia();
  scanVirtualMediaElements(document);

  const mutationObserver = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        mutation.addedNodes.forEach((node) => {
          if (node instanceof Element) {
            scanVirtualMediaElements(node);
          }
        });
        mutation.removedNodes.forEach((node) => {
          if (node instanceof Element) {
            revokeElementObjectUrls(node);
          }
        });
        continue;
      }

      if (mutation.type === 'attributes' && mutation.target instanceof Element) {
        handleMediaElement(mutation.target);
      }
    }
  });

  mutationObserver.observe(document, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['src', 'poster'],
  });

  window.setInterval(() => scanVirtualMediaElements(document), 1000);
}

function patchFetchForVirtualMedia(): void {
  if (fetchPatched || typeof window.fetch !== 'function') return;
  fetchPatched = true;

  const nativeFetch = window.fetch.bind(window);
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === 'string'
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    const method =
      init?.method || (input instanceof Request ? input.method : undefined) || 'GET';

    if (method.toUpperCase() === 'GET' && isVirtualMediaUrl(url)) {
      const blob = await unifiedCacheService.getCachedBlob(url);
      if (blob) {
        return new Response(blob, {
          status: 200,
          headers: {
            'Content-Type': blob.type || 'application/octet-stream',
            'Content-Length': String(blob.size),
          },
        });
      }
    }

    return nativeFetch(input, init);
  };
}

function scanVirtualMediaElements(root: ParentNode): void {
  if (root instanceof Element) {
    handleMediaElement(root);
  }

  root
    .querySelectorAll('img, video, audio, source')
    .forEach((element) => handleMediaElement(element));
}

function handleMediaElement(element: Element): void {
  if (
    !(
      element instanceof HTMLImageElement ||
      element instanceof HTMLVideoElement ||
      element instanceof HTMLAudioElement ||
      element instanceof HTMLSourceElement
    )
  ) {
    return;
  }

  const src = element.getAttribute('src') || '';
  const poster =
    element instanceof HTMLVideoElement ? element.getAttribute('poster') || '' : '';

  if (src && isVirtualMediaUrl(src)) {
    void replaceElementUrl(element, src, 'src');
  }

  if (poster && isVirtualMediaUrl(poster) && element instanceof HTMLVideoElement) {
    void replaceElementUrl(element, poster, 'poster');
  }
}

async function replaceElementUrl(
  element: HTMLImageElement | HTMLMediaElement | HTMLSourceElement,
  url: string,
  attribute: 'src' | 'poster'
): Promise<void> {
  const blob = await unifiedCacheService.getCachedBlob(url);
  if (!blob) return;

  revokeElementObjectUrl(element);

  const blobUrl = URL.createObjectURL(blob);
  objectUrls.set(element, blobUrl);
  element.setAttribute(attribute, blobUrl);

  if (element instanceof HTMLImageElement) {
    const revokeLater = () => {
      window.setTimeout(() => revokeElementObjectUrl(element), 10000);
    };
    element.addEventListener('load', revokeLater, { once: true });
    element.addEventListener('error', () => revokeElementObjectUrl(element), { once: true });
  } else if (element instanceof HTMLMediaElement) {
    element.load();
    element.addEventListener('emptied', () => revokeElementObjectUrl(element), {
      once: true,
    });
    element.addEventListener('error', () => revokeElementObjectUrl(element), { once: true });
  }
}

function revokeElementObjectUrl(element: Element): void {
  const objectUrl = objectUrls.get(element);
  if (objectUrl) {
    URL.revokeObjectURL(objectUrl);
    objectUrls.delete(element);
  }

  element.querySelectorAll?.('img, video, audio, source').forEach((child) => {
    const childUrl = objectUrls.get(child);
    if (childUrl) {
      URL.revokeObjectURL(childUrl);
      objectUrls.delete(child);
    }
  });
}
