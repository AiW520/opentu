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
const processedImages = new WeakSet<HTMLImageElement>();

export function initializeVirtualUrlInterceptor(): void {
  if (isInitialized) return;
  isInitialized = true;

  console.log('[VirtualUrlInterceptor] Initializing for desktop environment...');

  // 立即处理所有已存在的图片（不等待 DOMReady）
  document.querySelectorAll('img').forEach((img) => {
    if (isVirtualMediaUrl(img.src) && !processedImages.has(img)) {
      processedImages.add(img);
      handleVirtualImageUrl(img).catch(console.error);
    }
  });

  // 监听 DOM 变化，处理新增的图片
  const mutationObserver = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === 'childList') {
        for (const node of mutation.addedNodes) {
          if (node instanceof HTMLImageElement) {
            handleImageElement(node);
          } else if (node instanceof HTMLElement) {
            const images = node.querySelectorAll('img');
            images.forEach(handleImageElement);
          }
        }
      } else if (mutation.type === 'attributes' && mutation.target instanceof HTMLImageElement) {
        if (mutation.attributeName === 'src') {
          handleImageElement(mutation.target);
        }
      }
    }
  });

  // 使用 document 而不是 body，确保能捕获所有变化
  mutationObserver.observe(document, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['src'],
  });

  // 定期检查新增的图片（兜底方案）
  setInterval(() => {
    document.querySelectorAll('img').forEach(handleImageElement);
  }, 500);

  console.log('[VirtualUrlInterceptor] Initialization complete');
}

function handleImageElement(img: HTMLImageElement): void {
  if (processedImages.has(img)) return;
  
  const src = img.src;
  if (src && isVirtualMediaUrl(src)) {
    processedImages.add(img);
    handleVirtualImageUrl(img).catch(console.error);
  }
}

async function handleVirtualImageUrl(img: HTMLImageElement): Promise<void> {
  try {
    const src = img.src;
    if (!src || !isVirtualMediaUrl(src)) return;

    console.log('[VirtualUrlInterceptor] Handling virtual image URL:', src);

    // 标记图片正在处理
    img.dataset.virtualProcessing = 'true';

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
      img.alt = `加载失败: ${src}`;
      return;
    }

    // 创建 blob URL
    const blobUrl = URL.createObjectURL(blob);
    
    // 保存原始 src，以便如果失败可以回退
    const originalSrc = img.src;
    
    img.src = blobUrl;

    console.log('[VirtualUrlInterceptor] Successfully loaded virtual image:', src);

    // 监听图片加载完成后释放 blob URL
    img.onload = () => {
      console.log('[VirtualUrlInterceptor] Image loaded successfully:', src);
      setTimeout(() => {
        try {
          URL.revokeObjectURL(blobUrl);
        } catch {
          // 忽略释放错误
        }
      }, 10000); // 延迟释放，确保图片已渲染
    };

    img.onerror = () => {
      console.error('[VirtualUrlInterceptor] Failed to load blob URL:', blobUrl);
      try {
        URL.revokeObjectURL(blobUrl);
      } catch {
        // 忽略
      }
      // 回退到原图
      img.src = originalSrc;
    };
  } catch (error) {
    console.error('[VirtualUrlInterceptor] Failed to load virtual image:', error);
  } finally {
    img.dataset.virtualProcessing = 'false';
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

    console.log('[VirtualUrlInterceptor] Trying to read from file system:', fileName);

    // 调用 Tauri 命令获取缓存文件
    const base64Data = await (window as any).__TAURI_INTERNALS__.invoke('get_cached_media_file', {
      fileName,
    });

    if (!base64Data) {
      console.warn('[VirtualUrlInterceptor] No base64 data returned for:', fileName);
      return null;
    }

    console.log('[VirtualUrlInterceptor] Got base64 data, decoding...');

    // 解码 base64 数据
    const byteString = atob(base64Data);
    const mimeType = getMimeTypeFromFileName(fileName);
    const ab = new ArrayBuffer(byteString.length);
    const ia = new Uint8Array(ab);
    for (let i = 0; i < byteString.length; i++) {
      ia[i] = byteString.charCodeAt(i);
    }

    const blob = new Blob([ab], { type: mimeType });
    console.log('[VirtualUrlInterceptor] Created blob from file system:', {
      size: blob.size,
      type: blob.type,
    });

    return blob;
  } catch (error) {
    console.error('[VirtualUrlInterceptor] Failed to get blob from Tauri:', error);
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
