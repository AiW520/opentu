// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlaitBoard } from '@plait/core';
import { buildDrawnixHotkeyPlugin } from './with-hotkey';

const {
  updatePointerTypeMock,
  setCreationModeMock,
  updateAppStateMock,
  globalKeyDownMock,
  keyDownMock,
  getMovingPointInBoardMock,
  isMovingPointInBoardMock,
  hasBeenTextEditingMock,
  getSelectedElementsMock,
  getFreehandSettingsMock,
  setFreehandStrokeWidthMock,
  setEraserWidthMock,
  getPenSettingsMock,
  setPenStrokeWidthMock,
  updatePencilCursorMock,
  updateEraserCursorMock,
} = vi.hoisted(() => ({
  updatePointerTypeMock: vi.fn(),
  setCreationModeMock: vi.fn(),
  updateAppStateMock: vi.fn(),
  globalKeyDownMock: vi.fn(),
  keyDownMock: vi.fn(),
  getMovingPointInBoardMock: vi.fn(),
  isMovingPointInBoardMock: vi.fn(),
  hasBeenTextEditingMock: vi.fn(),
  getSelectedElementsMock: vi.fn(),
  getFreehandSettingsMock: vi.fn(),
  setFreehandStrokeWidthMock: vi.fn(),
  setEraserWidthMock: vi.fn(),
  getPenSettingsMock: vi.fn(),
  setPenStrokeWidthMock: vi.fn(),
  updatePencilCursorMock: vi.fn(),
  updateEraserCursorMock: vi.fn(),
}));

vi.mock('@plait/core', () => ({
  BoardTransforms: {
    updatePointerType: updatePointerTypeMock,
  },
  getSelectedElements: getSelectedElementsMock,
  PlaitBoard: {
    getMovingPointInBoard: getMovingPointInBoardMock,
    isMovingPointInBoard: isMovingPointInBoardMock,
    hasBeenTextEditing: hasBeenTextEditingMock,
    isPointer: vi.fn(),
    isInPointer: vi.fn((board: { pointer?: string }, pointers: string[]) =>
      pointers.includes(board.pointer || '')
    ),
  },
  PlaitPointerType: {
    hand: 'hand',
    selection: 'selection',
  },
}));

vi.mock('@plait/common', () => ({
  BoardCreationMode: {
    drawing: 'drawing',
    dnd: 'dnd',
  },
  setCreationMode: setCreationModeMock,
}));

vi.mock('@plait/mind', () => ({
  MindPointerType: {
    mind: 'mind',
  },
}));

vi.mock('./freehand/type', () => ({
  FreehandShape: {
    feltTipPen: 'felt-tip-pen',
    mask: 'mask',
    eraser: 'eraser',
    laserPointer: 'laser-pointer',
  },
}));

vi.mock('./pen/type', () => ({
  PenShape: {
    pen: 'pen',
  },
}));

vi.mock('./freehand/freehand-settings', () => ({
  getFreehandSettings: getFreehandSettingsMock,
  setFreehandStrokeWidth: setFreehandStrokeWidthMock,
  setEraserWidth: setEraserWidthMock,
}));

vi.mock('./pen/pen-settings', () => ({
  getPenSettings: getPenSettingsMock,
  setPenStrokeWidth: setPenStrokeWidthMock,
}));

vi.mock('../hooks/usePencilCursor', () => ({
  updatePencilCursor: updatePencilCursorMock,
  updateEraserCursor: updateEraserCursorMock,
}));

vi.mock('@plait/draw', () => ({
  ArrowLineShape: {
    straight: 'arrow-straight',
  },
  BasicShapes: {
    rectangle: 'rectangle',
    ellipse: 'ellipse',
    text: 'text',
  },
}));

vi.mock('../utils/image', () => ({
  addImage: vi.fn(),
  saveAsImage: vi.fn(),
}));

vi.mock('../data/json', () => ({
  saveAsJSON: vi.fn(),
}));

vi.mock('../transforms/alignment', () => ({
  AlignmentTransforms: {
    alignLeft: vi.fn(),
    alignCenter: vi.fn(),
    alignRight: vi.fn(),
    alignTop: vi.fn(),
    alignMiddle: vi.fn(),
    alignBottom: vi.fn(),
  },
}));

vi.mock('../transforms/distribute', () => ({
  DistributeTransforms: {
    distributeHorizontal: vi.fn(),
    distributeVertical: vi.fn(),
    autoArrange: vi.fn(),
  },
}));

vi.mock('../transforms/boolean', () => ({
  BooleanTransforms: {
    union: vi.fn(),
    subtract: vi.fn(),
    intersect: vi.fn(),
    exclude: vi.fn(),
    flatten: vi.fn(),
  },
}));

vi.mock('./with-frame', () => ({
  FramePointerType: 'frame',
}));

vi.mock('./with-lasso-selection', () => ({
  LassoPointerType: 'lasso',
}));

type TestBoard = PlaitBoard & {
  globalKeyDown: (event: KeyboardEvent) => void;
  keyDown: (event: KeyboardEvent) => void;
  undo: () => void;
  redo: () => void;
};

function createBoard(): TestBoard {
  return {
    globalKeyDown: globalKeyDownMock,
    keyDown: keyDownMock,
    undo: vi.fn(),
    redo: vi.fn(),
  } as unknown as TestBoard;
}

describe('buildDrawnixHotkeyPlugin', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getMovingPointInBoardMock.mockReturnValue([0, 0]);
    isMovingPointInBoardMock.mockReturnValue(false);
    hasBeenTextEditingMock.mockReturnValue(false);
    getSelectedElementsMock.mockReturnValue([]);
    getFreehandSettingsMock.mockReturnValue({
      strokeWidth: 10,
      eraserWidth: 32,
    });
    getPenSettingsMock.mockReturnValue({
      strokeWidth: 4,
    });
  });

  it('switches to vector pen on Shift+P', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)(createBoard()) as TestBoard;
    const event = new KeyboardEvent('keydown', {
      key: 'P',
      shiftKey: true,
      cancelable: true,
    });

    board.globalKeyDown(event);

    expect(setCreationModeMock).toHaveBeenCalledWith(board, 'drawing');
    expect(updatePointerTypeMock).toHaveBeenCalledWith(board, 'pen');
    expect(updateAppStateMock).toHaveBeenCalledWith({ pointer: 'pen' });
    expect(event.defaultPrevented).toBe(true);
    expect(globalKeyDownMock).not.toHaveBeenCalled();
  });

  it.each(['felt-tip-pen', 'mask', 'eraser', 'laser-pointer'])(
    'switches %s back to selection on Escape',
    (pointer) => {
      const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
        ...createBoard(),
        pointer,
      } as TestBoard & { pointer: string }) as TestBoard;
      const event = new KeyboardEvent('keydown', {
        key: 'Escape',
        cancelable: true,
      });

      board.globalKeyDown(event);

      expect(updatePointerTypeMock).toHaveBeenCalledWith(board, 'selection');
      expect(updateAppStateMock).toHaveBeenCalledWith({ pointer: 'selection' });
      expect(event.defaultPrevented).toBe(true);
      expect(globalKeyDownMock).not.toHaveBeenCalled();
    }
  );

  it('does not switch drawing tool to selection while typing Escape', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'eraser',
    } as TestBoard & { pointer: string }) as TestBoard;
    const input = document.createElement('input');
    const event = new KeyboardEvent('keydown', {
      key: 'Escape',
      cancelable: true,
    });
    Object.defineProperty(event, 'target', { value: input });

    board.globalKeyDown(event);

    expect(updatePointerTypeMock).not.toHaveBeenCalled();
    expect(updateAppStateMock).not.toHaveBeenCalled();
    expect(globalKeyDownMock).toHaveBeenCalledWith(event);
  });

  it.each(['felt-tip-pen', 'mask'])(
    'undoes the last stroke with Backspace while using %s',
    (pointer) => {
      const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
        ...createBoard(),
        pointer,
      } as TestBoard & { pointer: string }) as TestBoard;
      const event = new KeyboardEvent('keydown', {
        key: 'Backspace',
        cancelable: true,
      });

      board.globalKeyDown(event);

      expect(board.undo).toHaveBeenCalledTimes(1);
      expect(event.defaultPrevented).toBe(true);
      expect(globalKeyDownMock).not.toHaveBeenCalled();
    }
  );

  it.each(['eraser', 'laser-pointer', 'selection'])(
    'does not undo with Backspace while using %s',
    (pointer) => {
      const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
        ...createBoard(),
        pointer,
      } as TestBoard & { pointer: string }) as TestBoard;
      const event = new KeyboardEvent('keydown', {
        key: 'Backspace',
        cancelable: true,
      });

      board.globalKeyDown(event);

      expect(board.undo).not.toHaveBeenCalled();
    }
  );

  it('does not undo with Backspace while typing in an input', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'felt-tip-pen',
    } as TestBoard & { pointer: string }) as TestBoard;
    const input = document.createElement('input');
    const event = new KeyboardEvent('keydown', {
      key: 'Backspace',
      cancelable: true,
    });
    Object.defineProperty(event, 'target', { value: input });

    board.globalKeyDown(event);

    expect(board.undo).not.toHaveBeenCalled();
    expect(globalKeyDownMock).toHaveBeenCalledWith(event);
  });

  it('decreases freehand brush size with -', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'felt-tip-pen',
      appState: { toolSettingsVersion: 2 },
    } as TestBoard & { pointer: string; appState: { toolSettingsVersion: number } }) as TestBoard;
    const event = new KeyboardEvent('keydown', {
      key: '-',
      cancelable: true,
    });

    board.globalKeyDown(event);

    expect(setFreehandStrokeWidthMock).toHaveBeenCalledWith(board, 9);
    expect(updatePencilCursorMock).toHaveBeenCalledWith(board, 'felt-tip-pen');
    expect(updateAppStateMock).toHaveBeenCalledWith({ toolSettingsVersion: 3 });
    expect(event.defaultPrevented).toBe(true);
    expect(globalKeyDownMock).not.toHaveBeenCalled();
  });

  it('increases mask brush size with +', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'mask',
    } as TestBoard & { pointer: string }) as TestBoard;
    const event = new KeyboardEvent('keydown', {
      key: '+',
      shiftKey: true,
      cancelable: true,
    });

    board.globalKeyDown(event);

    expect(setFreehandStrokeWidthMock).toHaveBeenCalledWith(board, 11);
    expect(event.defaultPrevented).toBe(true);
  });

  it('increases eraser size with +', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'eraser',
    } as TestBoard & { pointer: string }) as TestBoard;
    const event = new KeyboardEvent('keydown', {
      key: '+',
      shiftKey: true,
      cancelable: true,
    });

    board.globalKeyDown(event);

    expect(setEraserWidthMock).toHaveBeenCalledWith(board, 33);
    expect(updateEraserCursorMock).toHaveBeenCalledWith(board);
    expect(event.defaultPrevented).toBe(true);
  });

  it('increases vector pen size with +', () => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'pen',
    } as TestBoard & { pointer: string }) as TestBoard;
    const event = new KeyboardEvent('keydown', {
      key: '+',
      shiftKey: true,
      cancelable: true,
    });

    board.globalKeyDown(event);

    expect(setPenStrokeWidthMock).toHaveBeenCalledWith(board, 5);
    expect(event.defaultPrevented).toBe(true);
  });

  it.each([
    ['ArrowUp', 11],
    ['ArrowRight', 11],
    ['ArrowDown', 9],
    ['ArrowLeft', 9],
  ])('adjusts freehand brush size with %s', (key, expectedSize) => {
    const board = buildDrawnixHotkeyPlugin(updateAppStateMock)({
      ...createBoard(),
      pointer: 'felt-tip-pen',
    } as TestBoard & { pointer: string }) as TestBoard;
    const event = new KeyboardEvent('keydown', {
      key,
      cancelable: true,
    });

    board.globalKeyDown(event);

    expect(setFreehandStrokeWidthMock).toHaveBeenCalledWith(
      board,
      expectedSize
    );
    expect(event.defaultPrevented).toBe(true);
  });
});
