import { unifiedCacheService } from '@aitu/drawnix';
import { isVirtualMediaUrl } from '@aitu/drawnix';

/**
 * 虚拟 URL 拦截器
 * 在桌面应用中，由于没有 Service Worker，需要手动拦截虚拟 URL 请求
 * 并从 Cache Storage 中读取数据
 */

let isInitialized = false;

export function initializeVirtualUrlInterceptor(): void {
  if (isInitialized) return;
  isInitialized = true;

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

    // 从统一缓存服务获取 blob
    const blob = await unifiedCacheService.getCachedBlob(src);
    if (!blob) {
      console.warn('[VirtualUrlInterceptor] Blob not found in cache:', src);
      return;
    }

    // 创建 blob URL
    const blobUrl = URL.createObjectURL(blob);
    img.src = blobUrl;

    // 监听图片加载完成后释放 blob URL
    img.onload = () => {
      // 延迟释放，确保图片已渲染
      setTimeout(() => {
        try {
          URL.revokeObjectURL(blobUrl);
        } catch {
          // 忽略释放错误
        }
      }, 5000);
    };
  } catch (error) {
    console.error('[VirtualUrlInterceptor] Failed to load virtual image:', error);
  }
}
