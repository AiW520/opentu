import { describe, expect, it, vi } from 'vitest';

vi.mock('@plait/core', () => ({
  PlaitBoard: {
    getBoardContainer: (board: any) => board.container,
  },
  Transforms: {
    setNode: (board: any, properties: Record<string, unknown>, path: number[]) => {
      board.children[path[0]] = {
        ...board.children[path[0]],
        ...properties,
      };
    },
  },
  toHostPoint: (_board: any, x: number, y: number) => [x, y],
  toViewBoxPoint: (_board: any, point: [number, number]) => point,
}));

vi.mock('@plait/draw', () => ({
  DrawTransforms: {
    insertImage: (board: any, imageItem: any, point: [number, number]) => {
      const { width, height, url } = imageItem;
      board.children.push({
        id: `image-${board.children.length}`,
        type: 'image',
        points: [point, [point[0] + width, point[1] + height]],
        url,
      });
    },
  },
}));

vi.mock('../image', () => ({}));
vi.mock('../../utils/posthog-analytics', () => ({
  analytics: { track: vi.fn() },
}));
vi.mock('../../utils/selection-utils', () => ({
  getInsertionPointForSelectedElements: vi.fn(() => undefined),
  getInsertionPointBelowBottommostElement: vi.fn(() => [100, 100]),
  scrollToPointIfNeeded: vi.fn(),
}));
vi.mock('../../utils/canvas-insertion-layout', () => ({
  getInsertionPointFromSavedSelection: vi.fn(() => undefined),
}));

import { insertVideoFromUrl } from '../video';

function createBoard() {
  return {
    children: [],
    container: {
      clientWidth: 1200,
      clientHeight: 800,
    },
    viewport: {
      zoom: 1,
    },
  } as any;
}

describe('insertVideoFromUrl', () => {
  it('preserves video metadata after inserting through Plait image transform', async () => {
    const board = createBoard();

    await insertVideoFromUrl(
      board,
      'https://example.com/result.mp4',
      [100, 100],
      false,
      undefined,
      true
    );

    expect(board.children).toHaveLength(1);
    expect(board.children[0]).toMatchObject({
      type: 'image',
      url: 'https://example.com/result.mp4#video',
      isVideo: true,
      videoType: 'video',
      width: 400,
      height: 225,
    });
  });
});
