import { afterEach, describe, expect, it, vi } from 'vitest';

const originalTauriInternals = (window as Window & {
  __TAURI_INTERNALS__?: unknown;
}).__TAURI_INTERNALS__;
const originalPlatformDescriptor = Object.getOwnPropertyDescriptor(
  Navigator.prototype,
  'platform'
);
const originalUserAgentDescriptor = Object.getOwnPropertyDescriptor(
  Navigator.prototype,
  'userAgent'
);

function mockPlatform(platform: string, userAgent = platform) {
  Object.defineProperty(navigator, 'platform', {
    value: platform,
    configurable: true,
  });
  Object.defineProperty(navigator, 'userAgent', {
    value: userAgent,
    configurable: true,
  });
}

describe('desktop-asset-url', () => {
  afterEach(() => {
    vi.resetModules();
    delete (navigator as Navigator & { platform?: string }).platform;
    delete (navigator as Navigator & { userAgent?: string }).userAgent;
    if (originalPlatformDescriptor) {
      Object.defineProperty(
        Navigator.prototype,
        'platform',
        originalPlatformDescriptor
      );
    }
    if (originalUserAgentDescriptor) {
      Object.defineProperty(
        Navigator.prototype,
        'userAgent',
        originalUserAgentDescriptor
      );
    }
    if (originalTauriInternals === undefined) {
      delete (window as Window & { __TAURI_INTERNALS__?: unknown })
        .__TAURI_INTERNALS__;
    } else {
      (window as Window & { __TAURI_INTERNALS__?: unknown })
        .__TAURI_INTERNALS__ = originalTauriInternals;
    }
  });

  it('uses Tauri Windows HTTP custom protocol form on Windows', async () => {
    (window as Window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__ = {};
    mockPlatform('Win32', 'Mozilla/5.0 Windows');
    const { convertLocalFilePathToAssetUrl } = await import(
      '../desktop-asset-url'
    );

    expect(convertLocalFilePathToAssetUrl('C:\\media\\demo.png')).toBe(
      'http://opentu-asset.localhost/C%3A%5Cmedia%5Cdemo.png'
    );
  });

  it('uses custom scheme on macOS and Linux until a loopback server exists', async () => {
    (window as Window & { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__ = {};
    const { convertLocalFilePathToAssetUrl } = await import(
      '../desktop-asset-url'
    );

    mockPlatform('MacIntel', 'Mozilla/5.0 Macintosh');
    expect(convertLocalFilePathToAssetUrl('/Users/demo/image.png')).toBe(
      'opentu-asset://localhost/%2FUsers%2Fdemo%2Fimage.png'
    );

    mockPlatform('Linux x86_64', 'Mozilla/5.0 X11; Linux x86_64');
    expect(convertLocalFilePathToAssetUrl('/home/demo/image.png')).toBe(
      'opentu-asset://localhost/%2Fhome%2Fdemo%2Fimage.png'
    );
  });
});
