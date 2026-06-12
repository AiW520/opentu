import { beforeEach, describe, expect, it, vi } from 'vitest';

const cacheMediaFromBlob = vi.fn();
const cachedUrls = new Set<string>();
const isCached = vi.fn(async (url: string) => cachedUrls.has(url));
const calculateBlobChecksum = vi.fn(async () => 'a'.repeat(64));

vi.mock('@aitu/utils', async () => {
  const actual = await vi.importActual<typeof import('@aitu/utils')>(
    '@aitu/utils'
  );
  return {
    ...actual,
    calculateBlobChecksum,
  };
});

vi.mock('../unified-cache-service', () => ({
  unifiedCacheService: {
    cacheMediaFromBlob,
    isCached,
  },
}));

describe('cacheRemoteUrl', () => {
  beforeEach(() => {
    cacheMediaFromBlob.mockReset();
    isCached.mockClear();
    calculateBlobChecksum.mockClear();
    cachedUrls.clear();
    cacheMediaFromBlob.mockImplementation(async (url: string) => {
      cachedUrls.add(url);
      return url;
    });
  });

  it('caches raw base64 image payloads as content-addressed local URLs', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(new Blob(['png-binary'], { type: 'image/png' }), {
        status: 200,
      })
    );

    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');

    const result = await cacheRemoteUrl(
      'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
      'task-raw-b64',
      'image',
      'png'
    );

    expect(result).toMatch(
      /^\/__aitu_cache__\/image\/content-[0-9a-f]{64}\.png$/
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringMatching(/^data:image\/png;base64,/)
    );
    expect(cacheMediaFromBlob).toHaveBeenCalledWith(
      result,
      expect.any(Blob),
      'image',
      { taskId: 'task-raw-b64' }
    );

    vi.unstubAllGlobals();
  });

  it('reuses the same cached file for identical base64 payloads across tasks', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockImplementation(
      async () =>
        new Response(new Blob(['same-binary'], { type: 'image/png' }), {
          status: 200,
        })
    );

    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const base64 =
      'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==';

    const first = await cacheRemoteUrl(base64, 'task-a', 'image', 'png');
    const second = await cacheRemoteUrl(base64, 'task-b', 'image', 'png');

    expect(first).toBe(second);
    expect(cacheMediaFromBlob).toHaveBeenCalledTimes(1);

    vi.unstubAllGlobals();
  });

  it('keeps remote https image urls unchanged without rewriting them to local cache paths', async () => {
    const fetchMock = vi.fn<typeof fetch>();
    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const remoteUrl = 'https://cdn.example.com/generated/task-123.png?sig=abc';

    const result = await cacheRemoteUrl(remoteUrl, 'task-http', 'image', 'png');

    expect(result).toBe(remoteUrl);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(cacheMediaFromBlob).not.toHaveBeenCalled();

    vi.unstubAllGlobals();
  });

  it('caches remote https audio urls while keeping original URLs', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(new Blob(['audio-binary'], { type: 'audio/mpeg' }), {
        status: 200,
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const remoteUrl = 'https://cdn.example.com/audio/task-123.mp3';

    const result = await cacheRemoteUrl(
      remoteUrl,
      'task-audio',
      'audio',
      'mp3'
    );

    expect(result).toBe(remoteUrl);
    expect(fetchMock).toHaveBeenCalledWith(remoteUrl, {
      credentials: 'omit',
      cache: 'no-store',
      referrerPolicy: 'no-referrer',
    });
    expect(cacheMediaFromBlob).toHaveBeenCalledWith(
      remoteUrl,
      expect.any(Blob),
      'audio',
      {
        taskId: 'task-audio',
        source: 'AI_GENERATED',
      }
    );

    vi.unstubAllGlobals();
  });

  it('caches playback-only remote audio urls while keeping original URLs', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(new Blob(['audio-binary'], { type: 'audio/mpeg' }), {
        status: 200,
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const remoteUrl = 'https://cdn.example.com/audio/task-456.mp3';

    const result = await cacheRemoteUrl(
      remoteUrl,
      'asset:d88312b4-5b86-4f11-b9a6-c4162ba07486',
      'audio',
      'mp3',
      undefined,
      { source: 'PLAYBACK_CACHE' }
    );

    expect(result).toBe(remoteUrl);
    expect(cacheMediaFromBlob).toHaveBeenCalledWith(
      remoteUrl,
      expect.any(Blob),
      'audio',
      {
        taskId: 'asset:d88312b4-5b86-4f11-b9a6-c4162ba07486',
        source: 'PLAYBACK_CACHE',
      }
    );

    vi.unstubAllGlobals();
  });

  it('caches force-remote cover images while keeping original URLs', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(new Blob(['cover-binary'], { type: 'image/jpeg' }), {
        status: 200,
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const remoteUrl = 'https://cdn.example.com/audio/cover.jpg';

    const result = await cacheRemoteUrl(
      remoteUrl,
      'task-audio-cover',
      'image',
      'jpg',
      1,
      { forceRemoteCache: true }
    );

    expect(result).toBe(remoteUrl);
    expect(cacheMediaFromBlob).toHaveBeenCalledWith(
      remoteUrl,
      expect.any(Blob),
      'image',
      {
        taskId: 'task-audio-cover',
        source: 'AI_GENERATED',
      }
    );

    vi.unstubAllGlobals();
  });

  it('keeps the original remote URL when cache write cannot be verified', async () => {
    cacheMediaFromBlob.mockResolvedValueOnce(
      'https://cdn.example.com/audio/cover.jpg'
    );

    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(new Blob(['cover-binary'], { type: 'image/jpeg' }), {
        status: 200,
      })
    );
    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const remoteUrl = 'https://cdn.example.com/audio/cover.jpg';

    const result = await cacheRemoteUrl(
      remoteUrl,
      'task-cover',
      'image',
      'jpg',
      undefined,
      { forceRemoteCache: true }
    );

    expect(result).toBe(remoteUrl);
    expect(cacheMediaFromBlob).toHaveBeenCalledWith(
      remoteUrl,
      expect.any(Blob),
      'image',
      {
        taskId: 'task-cover',
        source: 'AI_GENERATED',
      }
    );

    vi.unstubAllGlobals();
  });

  it('keeps remote http urls unchanged as well', async () => {
    const fetchMock = vi.fn<typeof fetch>();
    vi.stubGlobal('fetch', fetchMock);

    const { cacheRemoteUrl } = await import('./fallback-utils');
    const remoteUrl = 'http://cdn.example.com/video/task-123.mp4';

    const result = await cacheRemoteUrl(
      remoteUrl,
      'task-video',
      'video',
      'mp4'
    );

    expect(result).toBe(remoteUrl);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(cacheMediaFromBlob).not.toHaveBeenCalled();

    vi.unstubAllGlobals();
  });
});

describe('ensureBase64ForAI', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('loads desktop asset urls before sending image references to providers', async () => {
    class TestFileReader {
      result: string | ArrayBuffer | null = null;
      onload: (() => void) | null = null;
      onerror: ((error: unknown) => void) | null = null;

      readAsDataURL(blob: Blob) {
        blob
          .arrayBuffer()
          .then((buffer) => {
            const bytes = new Uint8Array(buffer);
            let binary = '';
            for (const byte of bytes) {
              binary += String.fromCharCode(byte);
            }
            this.result = `data:${blob.type};base64,${btoa(binary)}`;
            this.onload?.();
          })
          .catch((error) => this.onerror?.(error));
      }
    }

    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(new Blob(['desktop-image'], { type: 'image/png' }), {
        status: 200,
      })
    );
    vi.stubGlobal('window', {
      location: { origin: 'http://localhost' },
    });
    vi.stubGlobal('fetch', fetchMock);
    vi.stubGlobal('FileReader', TestFileReader);

    const { ensureBase64ForAI } = await import('./fallback-utils');
    const url = 'opentu-asset://localhost/C%3A%5Cimages%5Cref.png';

    const result = await ensureBase64ForAI({ type: 'url', value: url });

    expect(fetchMock).toHaveBeenCalledWith(url, {
      signal: undefined,
      referrerPolicy: 'no-referrer',
    });
    expect(result).toBe('data:image/png;base64,ZGVza3RvcC1pbWFnZQ==');
  });
});
