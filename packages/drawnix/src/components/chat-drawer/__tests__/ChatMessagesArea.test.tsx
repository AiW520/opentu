// @vitest-environment jsdom
import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';

const mockUnifiedMediaViewer = vi.fn();

vi.mock('../../shared/media-preview', () => ({
  UnifiedMediaViewer: (props: Record<string, unknown>) => {
    mockUnifiedMediaViewer(props);
    return <div data-testid="media-viewer" />;
  },
}));

vi.mock('../WorkflowMessageBubble', () => ({
  WorkflowMessageBubble: () => <div data-testid="workflow-message-bubble" />,
}));

vi.mock('../../MarkdownReadonly', () => ({
  default: ({ markdown }: { markdown: string }) => (
    <div data-testid="markdown-readonly">{markdown}</div>
  ),
}));

afterEach(() => {
  cleanup();
  mockUnifiedMediaViewer.mockClear();
});

describe('ChatMessagesArea', () => {
  it('opens media preview when user image is double clicked', async () => {
    const { ChatMessagesArea } = await import('../ChatMessagesArea');

    render(
      <ChatMessagesArea
        handler={{
          messages: [
            {
              id: 'msg-1',
              role: 'user',
              parts: [
                {
                  type: 'data-file',
                  data: {
                    url: '/image-1.png',
                    filename: 'image-1.png',
                    mediaType: 'image/png',
                  },
                },
              ],
            },
          ],
          status: 'ready',
          sendMessage: vi.fn(),
          stop: vi.fn(),
          regenerate: vi.fn(),
          setMessages: vi.fn(),
        }}
        workflowMessages={new Map()}
        retryingWorkflowId={null}
        handleWorkflowRetry={vi.fn()}
      />
    );

    fireEvent.click(screen.getByAltText('image-1.png'));

    await waitFor(() => {
      expect(mockUnifiedMediaViewer).toHaveBeenCalled();
      expect(mockUnifiedMediaViewer.mock.calls.at(-1)?.[0]).toMatchObject({
        visible: true,
        initialIndex: 0,
        items: [
          {
            id: 'msg-1-image-0',
            url: '/image-1.png',
            type: 'image',
            alt: 'image-1.png',
          },
        ],
      });
    });
  });
});
