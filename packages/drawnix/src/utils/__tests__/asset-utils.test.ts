import { describe, expect, it } from 'vitest';
import {
  AssetCategory,
  AssetSource,
  AssetType,
  type Asset,
  type FilterState,
} from '../../types/asset.types';
import {
  filterAssets,
  isInternalLibraryExcludedCache,
  matchesAssetSearchQuery,
} from '../asset-utils';

function createAsset(overrides: Partial<Asset> = {}): Asset {
  return {
    id: 'asset-1',
    type: AssetType.IMAGE,
    source: AssetSource.AI_GENERATED,
    url: '/__aitu_cache__/image/test.png',
    name: '城市夜景标题',
    mimeType: 'image/png',
    createdAt: 1,
    prompt: '赛博朋克风格的雨夜街道',
    ...overrides,
  };
}

function createFilters(overrides: Partial<FilterState> = {}): FilterState {
  return {
    activeType: 'ALL',
    activeSource: 'ALL',
    activeCategory: 'ALL',
    searchQuery: '',
    sortBy: 'DATE_DESC',
    ...overrides,
  };
}

describe('asset-utils', () => {
  it('matches fuzzy keywords from asset title', () => {
    expect(matchesAssetSearchQuery(createAsset(), '夜景')).toBe(true);
  });

  it('matches fuzzy keywords from asset prompt', () => {
    expect(matchesAssetSearchQuery(createAsset(), '雨夜')).toBe(true);
  });

  it('matches multiple case-insensitive keywords across title and prompt', () => {
    const asset = createAsset({
      name: 'Cyber Cat',
      prompt: 'A neon street portrait',
    });

    expect(matchesAssetSearchQuery(asset, ' cyber  PORTRAIT ')).toBe(true);
  });

  it('filters assets by prompt keywords', () => {
    const matched = createAsset({ id: 'matched', prompt: '蓝色玻璃建筑' });
    const missed = createAsset({ id: 'missed', prompt: '红色木屋' });

    const result = filterAssets(
      [matched, missed],
      createFilters({ searchQuery: '玻璃' })
    );

    expect(result.assets.map((asset) => asset.id)).toEqual(['matched']);
  });

  it('matches fuzzy keywords from subject metadata', () => {
    const asset = createAsset({
      name: '普通标题',
      prompt: '',
      category: AssetCategory.CHARACTER,
      characterMeta: {
        name: 'Lily',
        prompt: 'short double ponytails, yellow bucket hat',
      },
    });

    expect(matchesAssetSearchQuery(asset, 'lily bucket')).toBe(true);
  });

  it('filters subject assets by business category', () => {
    const character = createAsset({
      id: 'character',
      category: AssetCategory.CHARACTER,
      characterMeta: { name: 'Dad', prompt: 'green polo shirt' },
    });
    const general = createAsset({
      id: 'general',
      category: AssetCategory.GENERAL,
    });
    const legacy = createAsset({
      id: 'legacy',
      category: undefined,
    });

    const result = filterAssets(
      [general, character, legacy],
      createFilters({ activeCategory: AssetCategory.CHARACTER })
    );

    expect(result.assets.map((asset) => asset.id)).toEqual(['character']);
  });

  it('excludes internal mask preview caches from media library', () => {
    expect(
      isInternalLibraryExcludedCache({
        metadata: { source: 'ai-mask-brush' },
      })
    ).toBe(true);
    expect(
      isInternalLibraryExcludedCache({
        metadata: { source: 'ai-mask-reference-resize' },
      })
    ).toBe(true);
    expect(
      isInternalLibraryExcludedCache({
        metadata: { source: 'AI_GENERATED' },
      })
    ).toBe(false);
  });
});
