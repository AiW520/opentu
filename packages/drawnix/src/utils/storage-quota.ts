/**
 * Storage Quota Utility Functions
 * 存储配额工具函数
 */

import { ASSET_CONSTANTS } from '../constants/ASSET_CONSTANTS';
import type { StorageQuota, StorageStatus } from '../types/asset.types';
import { isTauriEnvironment } from './desktop-asset-url';

/**
 * Check Storage Quota
 * 检查存储配额
 */
export async function checkStorageQuota(): Promise<StorageQuota> {
  // 桌面端：优先使用 Rust CAS 存储统计
  if (isTauriEnvironment()) {
    try {
      const stats = await (window as any).__TAURI_INTERNALS__.invoke(
        'get_media_cache_stats',
      );
      if (stats && typeof stats.totalSizeBytes === 'number') {
        const usage = stats.totalSizeBytes;
        const quota = stats.maxCacheSizeBytes || DEFAULT_DESKTOP_CACHE_QUOTA;
        return {
          usage,
          quota,
          percentUsed: quota > 0 ? (usage / quota) * 100 : 0,
          available: quota - usage,
        };
      }
    } catch (error) {
      console.warn('[StorageQuota] Failed to get desktop cache stats:', error);
    }
  }

  if ('storage' in navigator && 'estimate' in navigator.storage) {
    const estimate = await navigator.storage.estimate();
    const usage = estimate.usage || 0;
    const quota = estimate.quota || 0;

    return {
      usage,
      quota,
      percentUsed: quota > 0 ? (usage / quota) * 100 : 0,
      available: quota - usage,
    };
  }

  // 如果浏览器不支持Storage API，返回默认值
  return {
    usage: 0,
    quota: 0,
    percentUsed: 0,
    available: 0,
  };
}

// 桌面端默认缓存配额 10GB
const DEFAULT_DESKTOP_CACHE_QUOTA = 10 * 1024 * 1024 * 1024;

/**
 * Get Storage Status
 * 获取存储状态
 */
export async function getStorageStatus(): Promise<StorageStatus> {
  const quota = await checkStorageQuota();

  return {
    quota,
    isNearLimit:
      quota.percentUsed >= ASSET_CONSTANTS.STORAGE_WARNING_THRESHOLD * 100,
    isCritical:
      quota.percentUsed >= ASSET_CONSTANTS.STORAGE_CRITICAL_THRESHOLD * 100,
  };
}

/**
 * Can Add Asset
 * 检查是否可以添加素材
 */
export async function canAddAssetBySize(blobSize: number): Promise<boolean> {
  const quota = await checkStorageQuota();

  if (quota.quota === 0) {
    // 浏览器不支持Storage API，允许添加
    return true;
  }

  // 保留5%缓冲空间
  const maxUsage = quota.quota * ASSET_CONSTANTS.STORAGE_CRITICAL_THRESHOLD;
  return quota.usage + blobSize < maxUsage;
}

/**
 * Trigger desktop cache cleanup
 * 触发桌面端缓存清理
 */
export async function triggerDesktopCacheCleanup(): Promise<{
  success: boolean;
  freedBytes?: number;
  error?: string;
}> {
  if (!isTauriEnvironment()) {
    return { success: false, error: 'Not in desktop environment' };
  }
  try {
    const result = await (window as any).__TAURI_INTERNALS__.invoke(
      'run_media_cache_cleanup',
    );
    return {
      success: true,
      freedBytes: result?.freedBytes || 0,
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Get Storage Warning Message
 * 获取存储警告消息
 */
export function getStorageWarningMessage(status: StorageStatus): string | null {
  if (status.isCritical) {
    return `存储空间已使用 ${status.quota.percentUsed.toFixed(1)}%，即将达到上限。请删除一些旧素材或下载后删除以释放空间。`;
  }

  if (status.isNearLimit) {
    return `存储空间已使用 ${status.quota.percentUsed.toFixed(1)}%，接近上限。建议清理一些旧素材。`;
  }

  return null;
}
