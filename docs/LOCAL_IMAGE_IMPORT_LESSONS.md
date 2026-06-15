# 本地图片导入修复经验

更新日期：2026-06-15

## 背景

桌面端用户通过工具栏或快捷键添加本地图片时，部分图片无法进入画布。问题集中在两类场景：

- Tauri 桌面环境仍走浏览器文件读取链路，无法稳定复用桌面端本地素材路径能力。
- Web 上传、拖拽、粘贴和素材库导入依赖 `File.type`，当系统返回空 MIME 时，图片会被误判为不支持的文件。

## 修复思路

本次修复把图片类型识别收敛到公共能力，并让桌面端入口优先走 Tauri 原生本地素材链路。

### 1. 桌面端优先使用本地路径导入

`addImage` 在 Tauri 环境中调用桌面文件选择能力，拿到本地文件路径后通过 `assetStorageService.addDesktopLocalAssetFromPath` 入库，再用运行时 URL 插入画布。

这样可以避免把用户本地图片先读成大 Blob 再传给前端，减少内存峰值，也和桌面端素材库已有的本地文件访问模型保持一致。

### 2. MIME 识别不能只信 `File.type`

浏览器和系统文件选择器可能返回空 MIME。图片判断新增两层逻辑：

1. 优先接受标准 `image/*` MIME。
2. MIME 为空或缺失时，根据扩展名推断受支持图片类型。

当前支持：`png`、`jpg`、`jpeg`、`webp`、`gif`、`svg`、`bmp`、`ico`、`avif`、`jfif`。

### 3. 上传入口统一复用判断函数

拖拽、粘贴、素材库上传、画布插入、素材入库都复用 `getSupportedImageFileMimeType`。不要在入口层继续散落 `file.type.startsWith('image/')` 判断，否则后续新增格式或修复空 MIME 时容易漏改。

### 4. 入库前补齐真实 MIME

当图片文件类型可通过扩展名推断，但原始 `File.type` 为空时，插入画布前会用推断出的 MIME 包装 Blob。这样素材库校验、缓存服务和后续渲染拿到的是一致的图片类型。

## 实现要点

- `data/blob.ts` 新增图片扩展名到 MIME 的映射，以及 `getImageMimeTypeFromFileName`、`getSupportedImageFileMimeType`。
- `utils/image.ts` 在 Tauri 环境走桌面本地素材导入，Web 环境继续走文件选择并补齐 MIME。
- `data/image.ts` 在插入图片时根据推断结果修正 Blob MIME，保证入库数据一致。
- `plugins/with-image.tsx` 让拖拽和粘贴图片支持空 MIME 文件。
- `MediaLibraryModal.tsx` 和 `AssetContext.tsx` 复用公共 MIME 判断，避免素材库上传误拒。
- `ASSET_CONSTANTS.ts` 和 `asset-utils.ts` 扩展图片白名单与错误提示。
- `asset-utils.test.ts` 覆盖新增图片 MIME 的校验和类型识别。

## 注意事项

- 不要新增独立的图片格式白名单；公共来源应是 `IMAGE_MIME_TYPES` 和 `getSupportedImageFileMimeType`。
- 桌面端本地图片导入应保留路径型素材能力，避免高并发场景下把大文件集中读入 JS 内存。
- Web 端仍需要保留 Blob 插入路径，保证浏览器环境、拖拽、粘贴和上传不依赖 Tauri API。
- 素材库校验和画布插入要同时更新，否则会出现“能选文件但不能入库”或“能入库但不能插入”的断层。

## 验证建议

1. 类型检查：

```bash
pnpm exec tsc --noEmit --pretty false -p packages/drawnix/tsconfig.json
```

2. 精确单测：

```bash
pnpm vitest run packages/drawnix/src/utils/__tests__/asset-utils.test.ts
```

3. 手动回归：

- 桌面端点击工具栏图片按钮，选择本地 PNG/JPG/WebP 后应插入画布。
- 拖拽 MIME 为空但扩展名有效的图片到画布，应能插入。
- 素材库上传 AVIF/BMP/JFIF 图片，应能识别为图片素材。
- 选择非图片文件时，不应被图片入口误插入。

## 提交备注模板

```text
问题描述:
- 桌面端本地图片添加依赖浏览器文件链路，部分本地图片无法插入画布。
- 多个上传入口只检查 File.type，空 MIME 图片会被误判为不支持。

修复思路:
- Tauri 环境优先使用桌面本地素材路径导入，减少前端大 Blob 读取。
- 收敛图片 MIME 推断逻辑，按扩展名兜底识别常见图片格式。
- 画布、拖拽、粘贴、素材库上传和入库校验统一复用公共判断。

更新代码架构:
- data/blob.ts 承担图片文件类型归一化能力。
- 各入口只消费 getSupportedImageFileMimeType，避免重复判断。
```
