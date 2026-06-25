const VIDEO_URL_MARKER = '#video';
const MERGED_VIDEO_MARKER_REGEXP = /#merged-video-[^?#&]*/i;
const VIDEO_URL_MARKER_REGEXP = /#video(?=$|[?#&])/i;
const VIDEO_EXTENSION_REGEXP =
  /\.(mp4|webm|mov|mkv|avi|m4v|flv|wmv)(?:[?#]|$)/i;

export function markVideoUrl(url: string): string {
  if (!url || hasVideoUrlMarker(url)) {
    return url;
  }
  return `${url}${VIDEO_URL_MARKER}`;
}

export function stripVideoUrlMarker(url: string): string {
  if (!url) {
    return '';
  }
  return url
    .replace(VIDEO_URL_MARKER_REGEXP, '')
    .replace(MERGED_VIDEO_MARKER_REGEXP, '');
}

export function hasVideoUrlMarker(url: string): boolean {
  return VIDEO_URL_MARKER_REGEXP.test(url) || MERGED_VIDEO_MARKER_REGEXP.test(url);
}

export function isLikelyVideoUrl(url: string): boolean {
  if (!url) {
    return false;
  }

  const normalizedUrl = url.toLowerCase();
  return (
    hasVideoUrlMarker(normalizedUrl) ||
    normalizedUrl.includes('/__aitu_cache__/video/') ||
    normalizedUrl.includes('/asset-library/video/') ||
    VIDEO_EXTENSION_REGEXP.test(normalizedUrl)
  );
}

export function isVideoLikeElement(element: any): boolean {
  if (!element || typeof element.url !== 'string') {
    return false;
  }

  if (element.isVideo === true || element.videoType) {
    return true;
  }

  return isLikelyVideoUrl(element.url);
}

export function createVideoImageItem(
  videoUrl: string,
  width: number,
  height: number,
  poster?: string
) {
  return {
    url: markVideoUrl(videoUrl),
    width,
    height,
    isVideo: true,
    videoType: 'video',
    ...(poster ? { poster } : {}),
  };
}
