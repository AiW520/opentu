# AI 图片蒙版编辑经验

更新日期：2026-05-17

## 背景

GPT Image 的局部编辑不是把蒙版笔迹合成到参考图里，而是通过 `/images/edits` 的独立 `mask` multipart 字段表达编辑区域。画布 UI 可以把蒙版显示成半透明笔迹，但提交给模型时必须按 OpenAI 语义输出：透明区域表示需要编辑，非透明区域表示保留。

这次问题的核心不是 UI 是否能看到蒙版，而是从画布选择、AI 输入框、任务创建、重新生成、同步/异步适配器到最终 FormData 的每一层都要保留 `maskImage`。

## 问题表现

- 蒙版笔迹被当作普通画笔合成进参考图，最终请求没有独立 `mask` 参数。
- AI 输入框预览能显示参考图，但看不出该图携带蒙版，容易误判是否会局部编辑。
- 重新生成只回填 prompt 和参考图，漏掉原任务的 `maskImage`。
- 异步图片链路只透传 `referenceImages`，未把 `maskImage` 带到 provider 表单。
- edit 请求与 generation 请求的默认参数不一致，`response_format` 没有统一默认到 `url`。

## 修复思路

- 把“蒙版画笔”作为 Freehand 的一种 shape，不新增独立实体；大小、形状、快捷键和画笔体验沿用现有工具。
- 选择单张普通图片时扫描与图片相交的蒙版笔迹，按图片显示矩形裁剪，并输出与原图自然尺寸一致的 mask 图片。
- `processSelectedContentForAI` 返回 `maskImage`，并确保蒙版笔迹不再进入普通 `graphicsImage` 合成。
- AI 输入框创建图片编辑任务时使用 `referenceImages: [原图]`、`generationMode: "image_edit"`、`maskImage`，预览中同时展示原图和蒙版。
- 任务预填、重新生成、storage、executor、同步图片适配器和异步图片适配器都透传 `maskImage`。
- GPT Image generation/edit 都默认写入 `response_format: "url"`；edit FormData 必须同时包含 `image[]` 与 `mask`。

## 代码落点

- `ai-mask-brush.ts`：蒙版笔迹发现、裁剪和导出。
- `selection-utils.ts`：选择内容时识别单图蒙版，避免把蒙版合成进普通图形。
- `AIInputBar.tsx` / `SelectedContentPreview.tsx`：AI 输入框提交与蒙版预览。
- `image-task-prefill.ts` / `task-utils.ts`：重新生成、任务回填和 mask 元数据保留。
- `gpt-image-adapter.ts`：GPT Image 请求体和 edit FormData 的默认 `response_format` 与 `mask` 字段。
- `async-image-api-service.ts` / `media-api` / `media-executor`：异步图片任务透传并提交 `maskImage`。
- `popup-toolbar.tsx` / `freehand-panel.tsx` / `with-hotkey.ts`：蒙版反选入口、工具栏入口和快捷键。

## 经验规则

- 蒙版是任务参数，不是参考图的一部分；任何合成预览都不能替代最终请求里的独立 `mask`。
- `maskImage` 必须和 `referenceImages` 一样进入任务协议、缓存解析、重新生成和异步 provider 表单。
- UI 层的 60% 透明度只是可视化效果；提交给 OpenAI 时要重新生成 alpha 语义正确的 PNG。
- 预览要展示“原图 + 蒙版”这对媒体，避免用户看见参考图却不知道是否带 mask。
- 本地缓存 URL、素材库 URL 等虚拟路径在提交 FormData 前必须解析成真实图片数据或可上传值。
- 新增图片参数时至少覆盖：选择解析、AI 输入框工作流、任务预填、同步 adapter、异步 adapter、重新生成。

## 验证

- `pnpm --dir packages/drawnix exec tsc --noEmit --pretty false`
- `pnpm --dir packages/drawnix exec vitest run src/services/__tests__/async-image-api-service.test.ts src/services/__tests__/default-image-adapter.test.ts src/services/__tests__/media-api-routing.test.ts src/services/__tests__/media-executor.test.ts`
- 建议补充抽查：`ai-mask-brush.test.ts`、`selection-utils.test.ts`、`image-task-prefill.test.ts`、`gpt-image-adapter.test.ts`、`workflow-converter.test.ts`

## 九、蒙版预览缓存不进入素材库

这次还补了一条更底层的经验：**预览可以缓存，不能默认入库**。蒙版画笔导出的 PNG 只是编辑过程中的中间态，不应被素材库当成用户素材展示。

### 问题表现

- 蒙版预览图会出现在素材库网格里，和真正的图片素材混在一起。
- 这些预览图并不是用户主动保存的资产，却会参与素材库排序、去重和浏览。
- 从产品语义上看，这会把“编辑辅助资源”误当成“可管理素材”。

### 根因

- `exportImageMaskFromBrushes()` 会把预览 PNG 写入统一缓存。
- `AssetContext` 加载素材时会扫统一缓存，并把可见缓存合并进素材库。
- 之前只排除了 `video-frame` 这类内部缓存，没覆盖蒙版预览这一类新中间态。

### 修复思路

- 不改蒙版生成逻辑，保留预览缓存能力。
- 在素材库侧新增内部缓存排除规则。
- 把 `ai-mask-brush`、`ai-mask-reference-resize` 归为内部缓存，不参与素材库展示。

### 架构变更

- 统一缓存继续承担“预览/中间态/可复用资源”的底层存储。
- 素材库不再把统一缓存全量当成业务素材全集，而是先做“可见性过滤”。
- 资产边界被拆成两层：
  - 用户素材：上传、AI 成果、可管理资源
  - 内部缓存：蒙版预览、参考图缩放、视频帧等编辑辅助资源

### 经验规则

- 任何编辑辅助图都应先问一句：它是“展示给用户看的结果”，还是“给系统内部流程用的中间态”。
- 如果是中间态，允许缓存，但不要直接进入素材库。
- 以后新增内部缓存 source 时，优先补到排除名单，而不是改素材库主流程。

### 关联代码

- [asset-utils.ts](</Users/ljq/code/tuziapi/tuzi-api/aitu/packages/drawnix/src/utils/asset-utils.ts>)
- [AssetContext.tsx](</Users/ljq/code/tuziapi/tuzi-api/aitu/packages/drawnix/src/contexts/AssetContext.tsx>)
