import { describe, expect, it, vi } from 'vitest';
import {
  advanceBatchInsertionFlow,
  createBatchInsertionFlowState,
  estimateCanvasTextSize,
  getBatchInsertionFlowCenter,
  getBottomMostInsertionPoint,
  getInsertionPointFromSavedSelection,
  getViewportAwareCardWidth,
  groupInsertionItems,
} from '../canvas-insertion-layout';

vi.mock('@plait/core', () => ({
  PlaitBoard: {
    getBoardContainer: (board: any) => board.container,
  },
  getRectangleByElements: (_board: any, elements: any[]) => {
    const element = elements[0];

    if (element?.throwRect) {
      throw new Error('bad element');
    }

    const rects = elements.map((item) => item.rect);
    const left = Math.min(...rects.map((rect) => rect.x));
    const top = Math.min(...rects.map((rect) => rect.y));
    const right = Math.max(...rects.map((rect) => rect.x + rect.width));
    const bottom = Math.max(...rects.map((rect) => rect.y + rect.height));

    return {
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    };
  },
}));

function createBoard(
  children: any[],
  selectedIds: string[] = [],
  container = { width: 1000, height: 700 },
  zoom = 1
) {
  return {
    appState: {
      lastSelectedElementIds: selectedIds,
    },
    children,
    container: {
      getBoundingClientRect: () => container,
    },
    viewport: {
      zoom,
    },
  } as any;
}

describe('canvas-insertion-layout', () => {
  it('returns undefined when saved selection is empty', () => {
    expect(getInsertionPointFromSavedSelection(createBoard([]))).toBeUndefined();
  });

  it('calculates left-aligned insertion point below saved selection', () => {
    const board = createBoard(
      [
        { id: 'a', rect: { x: 20, y: 30, width: 80, height: 40 } },
        { id: 'b', rect: { x: 120, y: 40, width: 30, height: 60 } },
      ],
      ['a', 'b']
    );

    expect(getInsertionPointFromSavedSelection(board)).toEqual([20, 150]);
  });

  it('calculates center-aligned insertion point below saved selection', () => {
    const board = createBoard(
      [{ id: 'a', rect: { x: 20, y: 30, width: 100, height: 40 } }],
      ['a']
    );

    expect(
      getInsertionPointFromSavedSelection(board, {
        align: 'center',
        targetWidth: 40,
      })
    ).toEqual([50, 120]);
  });

  it('falls back to bottommost element and ignores bad rectangles', () => {
    const board = createBoard([
      { id: 'bad', throwRect: true },
      { id: 'top', rect: { x: 10, y: 10, width: 30, height: 20 } },
      { id: 'bottom', rect: { x: 70, y: 100, width: 40, height: 25 } },
    ]);

    expect(getBottomMostInsertionPoint(board)).toEqual([70, 175]);
  });

  it('returns configured empty point when no bottommost element exists', () => {
    expect(
      getBottomMostInsertionPoint(createBoard([]), { emptyPoint: [100, 100] })
    ).toEqual([100, 100]);
  });

  it('uses legacy bottom fallback when existing elements have unusable rectangles', () => {
    const board = createBoard([{ id: 'bad', throwRect: true }]);

    expect(
      getBottomMostInsertionPoint(board, { emptyPoint: [100, 100] })
    ).toEqual([100, 50]);
  });

  it('estimates multiline text dimensions', () => {
    expect(estimateCanvasTextSize('abc\n1234567890')).toEqual({
      width: 80,
      height: 48,
    });
  });

  it('groups adjacent repeated group ids while preserving item order', () => {
    const items = [
      { id: 'a', groupId: 'g1' },
      { id: 'b', groupId: 'g1' },
      { id: 'c' },
      { id: 'd', groupId: 'g2' },
      { id: 'e', groupId: 'g2' },
    ];

    expect(groupInsertionItems(items)).toEqual([
      [items[0], items[1]],
      [items[2]],
      [items[3], items[4]],
    ]);
  });

  it('flows items horizontally until viewport canvas width is full', () => {
    const board = createBoard([], [], { width: 900, height: 600 }, 1);
    let state = createBatchInsertionFlowState(board, [100, 200], {
      horizontalGap: 20,
      verticalGap: 50,
    });

    const first = advanceBatchInsertionFlow(state, { width: 400, height: 300 });
    state = first.state;
    const second = advanceBatchInsertionFlow(state, { width: 400, height: 240 });
    state = second.state;
    const third = advanceBatchInsertionFlow(state, { width: 400, height: 260 });
    state = third.state;

    expect(first.point).toEqual([100, 200]);
    expect(second.point).toEqual([520, 200]);
    expect(third.point).toEqual([100, 550]);
    expect(third.wrapped).toBe(true);
  });

  it('uses zoom-adjusted viewport width for wrapping', () => {
    const board = createBoard([], [], { width: 900, height: 600 }, 2);
    let state = createBatchInsertionFlowState(board, [100, 100], {
      horizontalGap: 20,
      verticalGap: 50,
    });

    const first = advanceBatchInsertionFlow(state, { width: 400, height: 300 });
    state = first.state;
    const second = advanceBatchInsertionFlow(state, { width: 400, height: 300 });

    expect(first.point).toEqual([100, 100]);
    expect(second.point).toEqual([100, 450]);
    expect(second.wrapped).toBe(true);
  });

  it('calculates flow center from the full batch bounds', () => {
    const board = createBoard([], [], { width: 900, height: 600 }, 1);
    let state = createBatchInsertionFlowState(board, [100, 200], {
      horizontalGap: 20,
      verticalGap: 50,
    });

    state = advanceBatchInsertionFlow(state, { width: 400, height: 300 }).state;
    state = advanceBatchInsertionFlow(state, { width: 400, height: 240 }).state;
    state = advanceBatchInsertionFlow(state, { width: 400, height: 260 }).state;

    expect(getBatchInsertionFlowCenter(state)).toEqual([510, 505]);
  });

  it('uses viewport canvas width for markdown card width', () => {
    const board = createBoard([], [], { width: 900, height: 600 }, 1.5);

    expect(getViewportAwareCardWidth(board)).toBe(300);
  });
});
