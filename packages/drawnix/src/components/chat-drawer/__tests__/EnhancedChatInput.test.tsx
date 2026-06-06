import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { EnhancedChatInput } from '../EnhancedChatInput';

vi.mock('../../../contexts/ChatDrawerContext', () => ({
  useChatDrawerControl: () => ({
    submitGenerationFromDrawer: async () => true,
  }),
}));

vi.mock('../../../hooks/usePromptHistory', () => ({
  usePromptHistory: () => ({
    addHistory: () => undefined,
  }),
}));

vi.mock('../../../i18n', () => ({
  useI18n: () => ({
    language: 'zh',
  }),
}));

vi.mock('../../../contexts/AssetContext', () => ({
  useAssets: () => ({
    addAsset: async () => undefined,
  }),
}));

vi.mock('../useChatDrawerGenerationControls', () => ({
  useChatDrawerGenerationControls: () => ({
    generationType: 'agent',
    setGenerationType: () => undefined,
    selectedModel: 'gpt-5.4',
    selectedModelRef: null,
    selectedSelectionKey: 'gpt-5.4',
    selectedParams: {},
    compatibleParams: [],
    selectedCount: 1,
    setSelectedCount: () => undefined,
    currentModels: [],
    handleModelSelect: () => undefined,
    handleModelConfigSelect: () => undefined,
    handleParamSelect: () => undefined,
  }),
}));

vi.mock('../../shared/SelectedContentPreview', () => ({
  SelectedContentPreview: () => (
    <div data-testid="selected-content-preview">selected</div>
  ),
}));

vi.mock('../../ai-input-bar/GenerationTypeDropdown', () => ({
  GenerationTypeDropdown: () => <button type="button">类型</button>,
}));

vi.mock('../../ai-input-bar/ModelDropdown', () => ({
  ModelDropdown: () => <button type="button">模型</button>,
}));

vi.mock('../../ai-input-bar/ParametersDropdown', () => ({
  ParametersDropdown: () => <button type="button">参数</button>,
}));

vi.mock('../../ai-input-bar/CountDropdown', () => ({
  CountDropdown: () => <button type="button">数量</button>,
}));

vi.mock('../../media-library/MediaLibraryModal', () => ({
  MediaLibraryModal: () => <div data-testid="media-library-modal" />,
}));

vi.mock('../../icons', () => ({
  ImageUploadIcon: () => <span aria-hidden="true">upload</span>,
  MediaLibraryIcon: () => <span aria-hidden="true">library</span>,
}));

afterEach(() => {
  cleanup();
});

describe('EnhancedChatInput placeholder', () => {
  it('renders through the shared drawer composer shell', () => {
    render(
      <EnhancedChatInput
        selectedContent={[]}
        onSend={() => undefined}
        placeholder="继续描述要修改的内容..."
      />
    );

    expect(screen.getByTestId('ai-input-composer-shell-drawer')).toBeTruthy();
    expect(screen.getByTestId('drawer-ai-send-btn')).toBeTruthy();
    expect(screen.getByText('类型')).toBeTruthy();
    expect(screen.getByText('模型')).toBeTruthy();
  });

  it('uses continuation copy when there is no attached content', () => {
    render(
      <EnhancedChatInput
        selectedContent={[]}
        onSend={() => undefined}
        placeholder="继续描述要修改的内容..."
      />
    );

    expect(screen.getByPlaceholderText('继续描述要修改的内容...')).toBeTruthy();
  });

  it('keeps effect-description copy when content is attached', () => {
    render(
      <EnhancedChatInput
        selectedContent={[
          {
            type: 'image',
            name: '参考图',
            url: 'data:image/png;base64,abc',
          },
        ]}
        onSend={() => undefined}
        placeholder="继续描述要修改的内容..."
      />
    );

    expect(screen.getByPlaceholderText('描述你想要的效果...')).toBeTruthy();
  });
});
