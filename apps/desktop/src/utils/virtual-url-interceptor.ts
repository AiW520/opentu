import { unifiedCacheService } from '@drawnix/drawnix';

// 内联 isVirtualMediaUrl 函数，避免复杂的跨包导入问题
const ASSET_LIBRARY_URL_PREFIX = '/asset-library/';
const CACHE_URL_PREFIX = '/__aitu_cache__/';
const AI_GENERATED_URL_PREFIX = '/__aitu_generated__/';
const AI_GENERATED_AUDIO_URL_PREFIX = `${AI_GENERATED_URL_PREFIX}audio/`;

function normalizeVirtualMediaUrl(url: string): string {
  if (!url) return url;
  try {
    const parsed = new URL(url, 'http://aitu.local');
    return parsed.pathname;
  } catch {
    return url;
  }
}

function isAssetLibraryUrl(url: string): boolean {
  return normalizeVirtualMediaUrl(url).startsWith(ASSET_LIBRARY_URL_PREFIX);
}

function isLegacyCacheUrl(url: string): boolean {
  return normalizeVirtualMediaUrl(url).startsWith(CACHE_URL_PREFIX);
}

function isAIGeneratedAudioUrl(url: string): boolean {
  return normalizeVirtualMediaUrl(url).startsWith(AI_GENERATED_AUDIO_URL_PREFIX);
}

function isAIGeneratedVirtualUrl(url: string): boolean {
  return isAIGeneratedAudioUrl(url);
}

function isVirtualMediaUrl(url: string): boolean {
  return (
    isAssetLibraryUrl(url) ||
    isLegacyCacheUrl(url) ||
    isAIGeneratedVirtualUrl(url)
  );
}

/**
 * 虚拟 URL 拦截器
 * 在桌面应用中，由于没有 Service Worker，需要手动拦截虚拟 URL 请求
 * 并从 Cache Storage 或文件系统中读取数据
 */

let isInitialized = false;

export function initializeVirtualUrlInterceptor(): void {
  if (isInitialized) return;
  isInitialized = true;

  console.log('[VirtualUrlInterceptor] Initializing for desktop environment...');

  // 创建一个 IntersectionObserver 来处理图片加载
  const observer = new IntersectionObserver(
    async (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          const img = entry.target as HTMLImageElement;
          observer.unobserve(img);
          await handleVirtualImageUrl(img);
        }
      }
    },
    { rootMargin: '50px', threshold: 0.1 }
  );

  // 监听 DOM 变化
  const mutationObserver = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        for (const node of mutation.addedNodes) {
          if (node instanceof HTMLImageElement) {
            handleImageElement(node, observer);
          } else if (node instanceof HTMLElement) {
            const images = node.querySelectorAll('img');
            images.forEach((img) => handleImageElement(img, observer));
          }
        }
      } else if (mutation.type === 'attributes' && mutation.target instanceof HTMLImageElement) {
        if (mutation.attributeName === 'src') {
          handleImageElement(mutation.target, observer);
        }
      }
    }
  });

  mutationObserver.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['src'],
  });

  // 处理已存在的图片
  document.querySelectorAll('img').forEach((img) => {
    handleImageElement(img, observer);
  });

  // 立即处理所有虚拟 URL 图片
  setTimeout(() => {
    document.querySelectorAll('img').forEach((img) => {
      if (isVirtualMediaUrl(img.src)) {
        handleVirtualImageUrl(img).catch(console.error);
      }
    });
  }, 100);
}

function handleImageElement(img: HTMLImageElement, observer: IntersectionObserver): void {
  const src = img.src;
  if (src && isVirtualMediaUrl(src)) {
    observer.observe(img);
  }
}

async function handleVirtualImageUrl(img: HTMLImageElement): Promise<void> {
  try {
    const src = img.src;
    if (!src || !isVirtualMediaUrl(src)) return;

    console.log('[VirtualUrlInterceptor] Handling virtual image URL:', src);

    // 尝试从统一缓存服务获取 blob（优先使用 Cache API）
    let blob = await unifiedCacheService.getCachedBlob(src);

    if (!blob) {
      console.warn('[VirtualUrlInterceptor] Blob not found in Cache API, trying fallback...');
      
      // 尝试从 Tauri 文件系统读取
      blob = await getBlobFromTauriFileSystem(src);
    }

    if (!blob) {
      console.warn('[VirtualUrlInterceptor] Blob not found in any source:', src);
      
      // 尝试显示占位图或错误提示
      img.style.background = '#f5f5f5';
      return;
    }

    // 创建 blob URL
    const blobUrl = URL.createObjectURL(blob);
    img.src = blobUrl;

    console.log('[VirtualUrlInterceptor] Successfully loaded virtual image:', src);

    // 监听图片加载完成后释放 blob URL
    img.onload = () => {
      setTimeout(() => {
        try {
          URL.revokeObjectURL(blobUrl);
        } catch {
          // 忽略释放错误
        }
      }, 5000);
    };

    img.onerror = () => {
      console.error('[VirtualUrlInterceptor] Failed to load blob URL:', blobUrl);
      URL.revokeObjectURL(blobUrl);
    };
  } catch (error) {
    console.error('[VirtualUrlInterceptor] Failed to load virtual image:', error);
  }
}

/**
 * 从 Tauri 文件系统读取图片数据
 */
async function getBlobFromTauriFileSystem(url: string): Promise<Blob | null> {
  try {
    // 解析虚拟 URL 获取文件名
    const normalizedUrl = normalizeVirtualMediaUrl(url);
    const fileName = normalizedUrl.split('/').pop();
    
    if (!fileName) {
      return null;
    }

    // 调用 Tauri 命令获取缓存文件
    const base64Data = await (window as any).__TAURI_INTERNALS__.invoke('get_cached_media_file', {
      fileName,
    });

    if (!base64Data) {
      return null;
    }

    // 解码 base64 数据
    const byteString = atob(base64Data);
    const mimeType = getMimeTypeFromFileName(fileName);
    const ab = new ArrayBuffer(byteString.length);
    const ia = new Uint8Array(ab);
    for (let i = 0; i < byteString.length; i++) {
      ia[i] = byteString.charCodeAt(i);
    }

    return new Blob([ab], { type: mimeType });
  } catch (error) {
    console.warn('[VirtualUrlInterceptor] Failed to get blob from Tauri:', error);
    return null;
  }
}

/**
 * 从文件名推断 MIME 类型
 */
function getMimeTypeFromFileName(fileName: string): string {
  const extension = fileName.split('.').pop()?.toLowerCase();
  const mimeTypes: Record<string, string> = {
    'png': 'image/png',
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'webp': 'image/webp',
    'gif': 'image/gif',
    'svg': 'image/svg+xml',
    'mp4': 'video/mp4',
    'webm': 'video/webm',
    'mp3': 'audio/mpeg',
    'wav': 'audio/wav',
  };
  return mimeTypes[extension || ''] || 'application/octet-stream';
}
