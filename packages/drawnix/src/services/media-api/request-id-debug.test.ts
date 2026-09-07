import { describe, expect, it, vi } from 'vitest';
import {
  emitImageRequestIdDebugLog,
  getLatestImageRequestId,
  setLatestImageRequestId,
} from './request-id-debug';
import { extractRequestId } from './image-api';

describe('request-id-debug', () => {
  it('stores and exposes latest image request id', () => {
    setLatestImageRequestId('req-test-1');
    expect(getLatestImageRequestId()).toBe('req-test-1');
  });

  it('logs and stores request id when emitted', () => {
    const infoSpy = vi.spyOn(console, 'info').mockImplementation(() => {});

    emitImageRequestIdDebugLog('req-test-2', {
      model: 'gpt-image-1',
      source: 'unit-test',
    });

    expect(getLatestImageRequestId()).toBe('req-test-2');
    expect(infoSpy).toHaveBeenCalledWith('[X-Request-Id] req-test-2');

    infoSpy.mockRestore();
  });

  it('extracts uuid request id from noisy console text', () => {
    expect(
      extractRequestId(
        "[X-Request-Id][image] {requestId: 'd4ee33d0-2104-4175-89b0-ece755265a8e', model: null}"
      )
    ).toBe('d4ee33d0-2104-4175-89b0-ece755265a8e');
  });
});
