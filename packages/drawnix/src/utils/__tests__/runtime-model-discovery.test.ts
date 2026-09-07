import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('runtime-model-discovery', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.unstubAllGlobals();
  });

  it('不会把图片模型钉到音频类型列表里', async () => {
    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [
          {
            profileId: 'provider-image',
            discoveredAt: Date.now(),
            discoveredModels: [
              {
                id: 'gemini-3-pro-image-preview',
                label: 'Gemini Image',
                shortLabel: 'Gemini Image',
                shortCode: 'gmi',
                type: 'image',
                vendor: 'GEMINI',
              },
            ],
            selectedModelIds: ['gemini-3-pro-image-preview'],
          },
        ],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-image',
            name: '图片供应商',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { getPinnedSelectableModel } = await import(
      '../runtime-model-discovery'
    );

    expect(
      getPinnedSelectableModel('audio', 'gemini-3-pro-image-preview', {
        profileId: 'provider-image',
        modelId: 'gemini-3-pro-image-preview',
      })
    ).toBeNull();
  });

  it('主流最新静态模型可被初始选择器解析', async () => {
    const { getStaticModelConfig } = await import(
      '../../constants/model-config'
    );

    expect(getStaticModelConfig('gpt-5.1')?.type).toBe('text');
    expect(getStaticModelConfig('claude-sonnet-4-6')?.type).toBe('text');
    expect(getStaticModelConfig('seedream-v4')?.type).toBe('image');
    expect(getStaticModelConfig('veo3-fast-frames')?.type).toBe('video');
  });

  it('应用模型选择时会返回新增和移除增量', async () => {
    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [
          {
            profileId: 'provider-text',
            discoveredAt: Date.now(),
            discoveredModels: [
              {
                id: 'model-a',
                label: 'Model A',
                shortLabel: 'Model A',
                type: 'text',
                vendor: 'OPENAI',
              },
              {
                id: 'model-b',
                label: 'Model B',
                shortLabel: 'Model B',
                type: 'text',
                vendor: 'OPENAI',
              },
              {
                id: 'model-c',
                label: 'Model C',
                shortLabel: 'Model C',
                type: 'text',
                vendor: 'OPENAI',
              },
            ],
            selectedModelIds: ['model-a', 'model-b'],
          },
        ],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-text',
            name: '文本供应商',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    const result = runtimeModelDiscovery.applySelection('provider-text', [
      'model-b',
      'model-c',
    ]);

    expect(result.models.map((model) => model.id)).toEqual([
      'model-b',
      'model-c',
    ]);
    expect(result.addedModelIds).toEqual(['model-c']);
    expect(result.removedModelIds).toEqual(['model-a']);
  });

  it('加载旧目录时会刷新 HappyHorse 的供应商分类', async () => {
    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [
          {
            profileId: 'provider-happyhorse',
            discoveredAt: Date.now(),
            discoveredModels: [
              {
                id: 'happyhorse-1.0-t2v',
                label: 'HappyHorse 1.0 T2V',
                shortLabel: 'HappyHorse 1.0 T2V',
                type: 'video',
                vendor: 'OTHER',
                tags: ['happyhorse'],
              },
            ],
            selectedModelIds: ['happyhorse-1.0-t2v'],
          },
        ],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-happyhorse',
            name: 'HappyHorse',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );
    const state = runtimeModelDiscovery.getState('provider-happyhorse');

    expect(state.discoveredModels[0]).toMatchObject({
      id: 'happyhorse-1.0-t2v',
      type: 'video',
      vendor: 'HAPPYHORSE',
      sourceProfileId: 'provider-happyhorse',
    });
    expect(state.models[0]?.vendor).toBe('HAPPYHORSE');
  });

  it('运行时发现模型会识别 HappyHorse 供应商', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        text: async () =>
          JSON.stringify({
            data: [{ id: 'happyhorse-alpha-video', owned_by: 'happyhorse' }],
          }),
      }))
    );

    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-happyhorse',
            name: 'HappyHorse',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    const models = await runtimeModelDiscovery.discover(
      'provider-happyhorse',
      'https://api.example.com/v1',
      'test-key'
    );

    expect(models[0]).toMatchObject({
      id: 'happyhorse-alpha-video',
      type: 'video',
      vendor: 'HAPPYHORSE',
    });
  });

  it('运行时发现 Omni Flash 系列会识别为 Gemini 供应商', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        text: async () =>
          JSON.stringify({
            data: [
              {
                id: 'omni-flash',
                owned_by: 'openai',
                supported_endpoint_types: ['videos.generate'],
              },
              {
                id: 'omni-flash-components',
                owned_by: 'openai',
                supported_endpoint_types: ['videos.generate'],
              },
            ],
          }),
      }))
    );

    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-video',
            name: 'Video Provider',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    const models = await runtimeModelDiscovery.discover(
      'provider-video',
      'https://api.example.com/v1',
      'test-key'
    );

    expect(models.map((model) => model.vendor)).toEqual(['GEMINI', 'GEMINI']);
    expect(models.map((model) => model.type)).toEqual(['video', 'video']);
  });

  it('主端点浏览器 fetch 失败时会尝试 tuzi-api 候选端点获取模型', async () => {
    const fetchMock = vi.fn(async (url: string) => {
      if (url === 'https://apisz.ourzhishi.top/v1/models') {
        throw new TypeError('Failed to fetch');
      }

      if (url === 'https://api.tu-zi.com/v1/models') {
        return {
          ok: true,
          text: async () =>
            JSON.stringify({
              data: [
                {
                  id: 'gpt-image-2',
                  owned_by: 'openai',
                  category: '生图',
                },
              ],
            }),
        };
      }

      throw new Error(`Unexpected url: ${url}`);
    });

    vi.stubGlobal('fetch', fetchMock);

    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-tuzi',
            name: 'Tuzi',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    const models = await runtimeModelDiscovery.discover(
      'provider-tuzi',
      'https://apisz.ourzhishi.top/v1',
      'test-key',
      ['https://api.tu-zi.com']
    );

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      'https://apisz.ourzhishi.top/v1/models',
      expect.any(Object)
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      'https://api.tu-zi.com/v1/models',
      expect.any(Object)
    );
    expect(models[0]).toMatchObject({
      id: 'gpt-image-2',
      type: 'image',
    });
  });

  it('非 tuzi-api 主端点失败时不会把 API Key 发送到 tuzi-api 候选端点', async () => {
    const fetchMock = vi.fn(async (url: string) => {
      if (url === 'https://api.openai.com/v1/models') {
        throw new TypeError('Failed to fetch');
      }
      throw new Error(`Unexpected url: ${url}`);
    });

    vi.stubGlobal('fetch', fetchMock);

    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    await expect(
      runtimeModelDiscovery.discover(
        'provider-openai',
        'https://api.openai.com/v1',
        'openai-secret',
        ['https://api.tu-zi.com']
      )
    ).rejects.toThrow('Failed to fetch');

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock).toHaveBeenCalledWith(
      'https://api.openai.com/v1/models',
      expect.objectContaining({
        headers: { Authorization: 'Bearer openai-secret' },
      })
    );
  });

  it('不会把 OpenAI 自有 omni 模型误归类为 Gemini', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        text: async () =>
          JSON.stringify({
            data: [
              {
                id: 'omni-moderation-latest',
                owned_by: 'openai',
                supported_endpoint_types: ['moderations'],
              },
            ],
          }),
      }))
    );

    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-openai',
            name: 'OpenAI',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    const models = await runtimeModelDiscovery.discover(
      'provider-openai',
      'https://api.example.com/v1',
      'test-key'
    );

    expect(models).toHaveLength(1);
    expect(models[0]).toMatchObject({
      id: 'omni-moderation-latest',
      vendor: 'GPT',
    });
  });

  it('优先按接口 category 分类模型', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        text: async () =>
          JSON.stringify({
            data: [
              {
                id: 'gpt-4o-image-async',
                owned_by: 'openai',
                category: '生图',
                supported_endpoint_types: [
                  'OpenAI-Chat',
                  'edit',
                  'generate',
                  'openai-video',
                ],
              },
              {
                id: 'research-video-preview',
                owned_by: 'openai',
                category: '文本',
                supported_endpoint_types: ['openai-video'],
              },
            ],
          }),
      }))
    );

    vi.doMock('../settings-manager', () => ({
      LEGACY_DEFAULT_PROVIDER_PROFILE_ID: 'legacy-default',
      providerCatalogsSettings: {
        get: () => [],
        addListener: () => {},
        removeListener: () => {},
        update: async () => {},
      },
      providerProfilesSettings: {
        get: () => [
          {
            id: 'provider-openai',
            name: 'OpenAI',
            enabled: true,
          },
        ],
        addListener: () => {},
        removeListener: () => {},
      },
      invocationPresetsSettings: {
        addListener: () => {},
        removeListener: () => {},
      },
      settingsManager: {
        getSetting: () => ({}),
        addListener: () => {},
        removeListener: () => {},
      },
    }));

    const { runtimeModelDiscovery } = await import(
      '../runtime-model-discovery'
    );

    const models = await runtimeModelDiscovery.discover(
      'provider-openai',
      'https://api.example.com/v1',
      'test-key'
    );

    expect(models).toHaveLength(2);
    expect(
      models.find((model) => model.id === 'gpt-4o-image-async')
    ).toMatchObject({
      type: 'image',
      vendor: 'GPT',
    });
    expect(
      models.find((model) => model.id === 'research-video-preview')
    ).toMatchObject({
      type: 'text',
      vendor: 'GPT',
    });
  });
});
