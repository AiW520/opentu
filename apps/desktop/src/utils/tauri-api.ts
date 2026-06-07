/**
 * Tauri 桌面端 API 封装
 * 提供与 Rust 后端通信的接口
 * 使用全局 Tauri API，无需依赖 @tauri-apps/api 包
 */

// 检查是否在 Tauri 环境中运行
export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
}

// 调用 Tauri 命令
async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const internals = (window as any).__TAURI_INTERNALS__;
  if (!internals) {
    throw new Error('Not running in Tauri environment');
  }
  return internals.invoke(command, args);
}

// ===== 媒体文件路径管理 =====

/** 获取媒体根目录路径 */
export async function getMediaRootPath(): Promise<string> {
  return invoke<string>('get_media_root_path');
}

/** 设置媒体根目录路径 */
export async function setMediaRootPath(path: string): Promise<string> {
  return invoke<string>('set_media_root_path', { path });
}

/** 重置媒体根目录为默认值 */
export async function resetMediaRootPath(): Promise<string> {
  return invoke<string>('reset_media_root_path');
}

/** 打开文件夹选择对话框 */
export async function pickMediaFolder(): Promise<string | null> {
  return invoke<string | null>('pick_media_folder');
}

// ===== 文件操作 =====

/** 保存文件到媒体目录 */
export async function saveFile(
  fileName: string,
  buffer: Uint8Array,
  fileType?: string
): Promise<string> {
  return invoke<string>('save_file', {
    fileName,
    buffer: Array.from(buffer),
    fileType: fileType || null,
  });
}

/** 获取文件路径 */
export async function getFilePath(
  fileName: string,
  fileType?: string
): Promise<string | null> {
  return invoke<string | null>('get_file_path', {
    fileName,
    fileType: fileType || null,
  });
}

/** 删除文件 */
export async function deleteFile(
  fileName: string,
  fileType?: string
): Promise<void> {
  return invoke<void>('delete_file', {
    fileName,
    fileType: fileType || null,
  });
}

/** 获取媒体目录 */
export async function getMediaDir(): Promise<string> {
  return invoke<string>('get_media_dir');
}

/** 获取缓存媒体文件数据（用于桌面应用中加载虚拟URL） */
export async function getCachedMediaFile(fileName: string, fileType?: string): Promise<Uint8Array | null> {
  try {
    const result = await invoke<string>('get_cached_media_file', {
      fileName,
      fileType: fileType || null,
    });
    // 解析 Base64 编码的结果
    const binaryString = atob(result);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  } catch {
    return null;
  }
}

// ===== 存储统计 =====

export interface StorageStats {
  dbSize: number;
  mediaSize: number;
  totalSize: number;
  dataDir: string;
  mediaRoot: string;
}

/** 获取存储统计信息 */
export async function getStorageStats(): Promise<StorageStats> {
  return invoke<StorageStats>('get_stats');
}

// ===== 设置存储 =====

/** 获取本地设置 */
export async function getLocalSetting(key: string): Promise<string | null> {
  return invoke<string | null>('get_local', { key });
}

/** 设置本地配置 */
export async function setLocalSetting(key: string, value: string): Promise<void> {
  return invoke<void>('set_local', { key, value });
}

/** 删除本地配置 */
export async function removeLocalSetting(key: string): Promise<void> {
  return invoke<void>('remove_local', { key });
}

// ===== 文件保存对话框 =====

/**
 * 显示保存文件对话框，让用户选择保存位置
 * @param defaultName 默认文件名
 * @returns 用户选择的文件路径，如果取消则返回 null
 */
export async function pickSaveLocation(defaultName: string): Promise<string | null> {
  return invoke<string | null>('pick_save_location', {
    defaultName,
  });
}

/** 保存数据到用户选择的位置
 * @param savePath 用户选择的保存路径
 * @param data 要保存的数据（Uint8Array）
 */
export async function saveToLocation(
  savePath: string,
  data: Uint8Array
): Promise<void> {
  const { writeFile } = await import('@tauri-apps/plugin-fs');
  await writeFile(savePath, data);
}