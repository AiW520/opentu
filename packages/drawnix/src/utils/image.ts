import { getSelectedElements, PlaitBoard } from '@plait/core';
import { base64ToBlob, download } from '@aitu/utils';
import { boardToImage } from './common';
import { fileOpen, isFileSystemAbortError } from '../data/filesystem';
import { IMAGE_MIME_TYPES } from '../constants';
import { insertImage } from '../data/image';
import { MessagePlugin } from './message-plugin';
import { getImageNaturalSize } from './image-natural-size';
import { assetStorageService } from '../services/asset-storage-service';
import { isTauriEnvironment } from './desktop-asset-url';

export { getImageNaturalSize } from './image-natural-size';

/**
 * 计算图片编辑后的新尺寸和位置
 * 保持原图的缩放比例，左上角位置不变
 */
export interface ImageElementInfo {
  url: string;
  width?: number;
  height?: number;
  points: [[number, number], [number, number]];
}

export interface ScaledImageResult {
  newPoints: [[number, number], [number, number]];
  scale: number;
}

export async function calculateEditedImagePoints(
  element: ImageElementInfo,
  newNaturalWidth: number,
  newNaturalHeight: number
): Promise<ScaledImageResult> {
  const [start, end] = element.points;
  const originalDisplayWidth = end[0] - start[0];
  const originalDisplayHeight = end[1] - start[1];
  
  // 获取原图的实际尺寸
  let originalNaturalWidth = element.width;
  let originalNaturalHeight = element.height;
  
  if (!originalNaturalWidth || !originalNaturalHeight) {
    const size = await getImageNaturalSize(
      element.url,
      originalDisplayWidth,
      originalDisplayHeight
    );
    originalNaturalWidth = size.width;
    originalNaturalHeight = size.height;
  }
  
  // 计算原图的缩放比例
  const scaleX = originalDisplayWidth / originalNaturalWidth;
  const scaleY = originalDisplayHeight / originalNaturalHeight;
  // 使用较小的缩放比例保持宽高比一致
  const scale = Math.min(scaleX, scaleY);
  
  // 计算新的显示尺寸
  const newDisplayWidth = newNaturalWidth * scale;
  const newDisplayHeight = newNaturalHeight * scale;
  
  return {
    newPoints: [start, [start[0] + newDisplayWidth, start[1] + newDisplayHeight]],
    scale,
  };
}

export const saveAsImage = (board: PlaitBoard, isTransparent: boolean) => {
  const selectedElements = getSelectedElements(board);
  void (async () => {
    try {
      const image = await boardToImage(board, {
        elements: selectedElements.length > 0 ? selectedElements : undefined,
        fillStyle: isTransparent ? 'transparent' : 'white',
      });

      if (image) {
        const ext = isTransparent ? 'png' : 'jpg';
        const pngImage = base64ToBlob(image);
        const imageName = `drawnix-${new Date().getTime()}.${ext}`;

        // 桌面端使用 Tauri 原生文件保存对话框
        if (isTauriEnvironment()) {
          try {
            const savePath = await (window as any).__TAURI_INTERNALS__.invoke('pick_save_location', {
              defaultName: imageName,
            });
            if (savePath) {
              const arrayBuffer = await pngImage.arrayBuffer();
              const uint8Array = new Uint8Array(arrayBuffer);
              await (window as any).__TAURI_INTERNALS__.invoke('write_file_to_path', {
                savePath: savePath,
                buffer: Array.from(uint8Array),
              });
              MessagePlugin.success('图片已保存');
            }
          } catch (tauriError) {
            console.warn('[ImageExport] Tauri save failed, falling back to browser download:', tauriError);
            download(pngImage, imageName);
          }
        } else {
          // 浏览器端使用原生下载
          download(pngImage, imageName);
        }
      }
    } catch (error) {
      console.warn('[ImageExport] Failed to export image:', error);
      MessagePlugin.error('导出图片失败，请稍后重试');
    }
  })();
};

export const addImage = async (board: PlaitBoard) => {
  try {
    // 桌面端使用 Tauri 原生文件选择器
    if (isTauriEnvironment()) {
      const pickedFiles = await assetStorageService.pickDesktopMediaFiles();
      if (pickedFiles.length === 0) {
        return;
      }
      // 逐个添加选中的文件
      for (const file of pickedFiles) {
        await assetStorageService.addDesktopLocalAssetFromPath({
          path: file.path,
          type: 'IMAGE',
          name: file.name,
          mimeType: file.mimeType,
        });
      }
      return;
    }
    // 浏览器端使用原生 fileOpen
    const imageFile = await fileOpen({
      description: 'Image',
      extensions: Object.keys(
        IMAGE_MIME_TYPES
      ) as (keyof typeof IMAGE_MIME_TYPES)[],
    });
    insertImage(board, imageFile);
  } catch (error) {
    if (isFileSystemAbortError(error)) {
      return;
    }
    MessagePlugin.error('添加图片失败');
    console.error('[addImage] Error:', error);
  }
};
