const SUPPORTED_EXTERNAL_PROTOCOLS = new Set(['http:', 'https:', 'mailto:', 'tel:']);
const URL_PROTOCOL_PATTERN = /^[a-zA-Z][a-zA-Z\d+.-]*:/;

function isRelativeUrl(url: string): boolean {
  const trimmedUrl = url.trim();
  return !trimmedUrl.startsWith('//') && !URL_PROTOCOL_PATTERN.test(trimmedUrl);
}

function getValidatedExternalUrl(url: string): string {
  const parsedUrl = new URL(url, window.location.href);
  if (!SUPPORTED_EXTERNAL_PROTOCOLS.has(parsedUrl.protocol)) {
    throw new Error(`Unsupported external URL protocol: ${parsedUrl.protocol}`);
  }
  return parsedUrl.href;
}

function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
}

async function openWithTauriOpener(url: string): Promise<void> {
  await (window as any).__TAURI_INTERNALS__.invoke('plugin:opener|open_url', { url });
}

export async function openExternalUrl(url: string): Promise<void> {
  if (isRelativeUrl(url)) {
    window.open(url, '_blank', 'noopener,noreferrer');
    return;
  }

  const externalUrl = getValidatedExternalUrl(url);

  if (isTauriEnvironment()) {
    try {
      await openWithTauriOpener(externalUrl);
      return;
    } catch (error) {
      console.warn('[openExternalUrl] Failed to open URL with Tauri opener:', error);
    }
  }

  window.open(externalUrl, '_blank', 'noopener,noreferrer');
}
