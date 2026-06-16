import { describe, expect, it } from 'vitest';
import {
  createVideoImageItem,
  isLikelyVideoUrl,
  isVideoLikeElement,
  markVideoUrl,
  stripVideoUrlMarker,
} from '../video-url';

describe('video-url', () => {
  it('marks video URLs without losing explicit video metadata', () => {
    expect(markVideoUrl('/__aitu_cache__/video/task-1.mp4')).toBe(
      '/__aitu_cache__/video/task-1.mp4#video'
    );
    expect(markVideoUrl('/signed/video?token=abc')).toBe(
      '/signed/video?token=abc#video'
    );
    expect(markVideoUrl('/video.mp4#chapter-1')).toBe(
      '/video.mp4#chapter-1#video'
    );
    expect(markVideoUrl('/video.mp4#merged-video-123')).toBe(
      '/video.mp4#merged-video-123'
    );

    expect(createVideoImageItem('/signed/video', 400, 225)).toEqual({
      url: '/signed/video#video',
      width: 400,
      height: 225,
      isVideo: true,
      videoType: 'video',
    });
    expect(createVideoImageItem('/signed/video', 400, 225, '/poster.png')).toEqual({
      url: '/signed/video#video',
      width: 400,
      height: 225,
      isVideo: true,
      videoType: 'video',
      poster: '/poster.png',
    });
  });

  it('strips only canvas video markers for playback', () => {
    expect(stripVideoUrlMarker('/video.mp4#video')).toBe('/video.mp4');
    expect(stripVideoUrlMarker('/video.mp4#chapter-1#video')).toBe(
      '/video.mp4#chapter-1'
    );
    expect(stripVideoUrlMarker('/video.mp4#video?token=abc')).toBe(
      '/video.mp4?token=abc'
    );
    expect(stripVideoUrlMarker('/signed/video?token=abc#video')).toBe(
      '/signed/video?token=abc'
    );
    expect(stripVideoUrlMarker('/video.mp4#merged-video-123')).toBe(
      '/video.mp4'
    );
    expect(stripVideoUrlMarker('/video.mp4#chapter-1')).toBe(
      '/video.mp4#chapter-1'
    );
  });

  it('recognizes video-like elements by metadata, marker, cache path, or extension', () => {
    expect(isVideoLikeElement({ url: '/media/raw', isVideo: true })).toBe(true);
    expect(isVideoLikeElement({ url: '/media/raw', videoType: 'video' })).toBe(
      true
    );
    expect(isVideoLikeElement({ url: '/media/raw#video' })).toBe(true);
    expect(isVideoLikeElement({ url: '/__aitu_cache__/video/task-1' })).toBe(
      true
    );
    expect(isLikelyVideoUrl('https://example.com/output.mp4?token=1')).toBe(
      true
    );
    expect(isVideoLikeElement({ url: '/media/image.png' })).toBe(false);
  });
});
