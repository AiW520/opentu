import { isTauriEnvironment } from './tauri-env';

type TauriOpener = {
  openUrl?: (url: string) => Promise<void>;
};

type TauriInternals = {
  invoke?: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
};

const ALLOWED_EXTERNAL_PROTOCOLS = new Set(['http:', 'https:', 'mailto:', 'tel:']);

export const OPENTU_GITHUB_URL = 'https://github.com/AiW520/opentu';

function normalizeExternalUrl(url: string): string | null {
  try {
    const parsedUrl = new URL(url);
    return ALLOWED_EXTERNAL_PROTOCOLS.has(parsedUrl.protocol)
      ? parsedUrl.toString()
      : null;
  } catch {
    return null;
  }
}

function openWithBrowser(url: string): void {
  const openedWindow = window.open(url, '_blank', 'noopener,noreferrer');
  if (openedWindow) {
    openedWindow.opener = null;
  }
}

export async function openExternalUrl(url: string): Promise<void> {
  if (typeof window === 'undefined') {
    return;
  }

  const normalizedUrl = normalizeExternalUrl(url);
  if (!normalizedUrl) {
    return;
  }

  if (isTauriEnvironment()) {
    const opened = await openWithTauri(normalizedUrl);
    if (opened) {
      return;
    }
  }

  openWithBrowser(normalizedUrl);
}

async function openWithTauri(url: string): Promise<boolean> {
  const tauriWindow = window as typeof window & {
    __TAURI__?: { opener?: TauriOpener };
    __TAURI_INTERNALS__?: TauriInternals;
  };
  const opener = tauriWindow.__TAURI__?.opener;

  try {
    if (opener?.openUrl) {
      await opener.openUrl(url);
      return true;
    }

    if (tauriWindow.__TAURI_INTERNALS__?.invoke) {
      await tauriWindow.__TAURI_INTERNALS__.invoke('plugin:opener|open_url', {
        url,
      });
      return true;
    }
  } catch (error) {
    console.warn('[external-url] Tauri opener failed, falling back to browser open', error);
  }

  return false;
}
