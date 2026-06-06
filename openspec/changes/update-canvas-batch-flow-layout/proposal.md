# Change: 批量插入画布改为视口流式排布

## Why

当前批量插入默认按纵向堆叠，宽屏桌面下浪费横向空间，生成多张图片或音频卡片时需要大量向下滚动。

## What Changes

- 批量插入按当前画布可视宽度和缩放比例计算一行可用宽度
- 插入项先横向排布，超出一屏宽度后自动换行
- 插入完成后滚动到整批内容包围盒中心
- 服务层、MCP 工具层、自动入画布复用同一套布局计算

## Impact

- Affected specs:
  - `canvas-insertion`
- Affected code:
  - `packages/drawnix/src/utils/canvas-insertion-layout.ts`
  - `packages/drawnix/src/services/canvas-operations/canvas-insertion.ts`
  - `packages/drawnix/src/mcp/tools/canvas-insertion.ts`
