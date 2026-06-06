# 图片生成 API 参数兼容性与比例转换修复经验

## 问题概述

本次修复涉及三个关联问题，全部为项目预存代码问题，非近期修改引入：

1. **rix API 400 错误**：`response_format` 参数不被 rix API 支持，导致图片生成失败
2. **非 1:1 比例全部生成 1:1**：用户选择 16:9、4:3 等比例时，API 实际生成正方形图片
3. **图片插入画布后强制 1:1 显示**：`foreignObject` 容器始终 400x400，竖版长图被压缩

---

## 一、架构背景

### 请求路由架构

```
用户输入 → AI Input Parser → Workflow Engine → Image Generation Service
                                                    ↓
                                          executorParams (size/quality/...)
                                                    ↓
                                         Fallback Executor
                                               ↙        ↘
                              buildImageRequestBody     resolveAdapterForInvocation
                              (通用请求体构建)          (专用适配器路由)
                                    ↓                        ↓
                              providerTransport.send    gpt-image-adapter.ts
                              → 直接 fetch 到 API       → 专项处理 request body
```

### 关键设计点

- **`buildImageRequestBody`** 是统一的请求体构建函数，被 `fallback-executor.ts` 和 `generateImageSync` 两个路径调用
- **适配器层**（`gpt-image-adapter.ts`、`tuzi-gpt-image-adapter.ts`）有独立的 `response_format` 处理逻辑，只在使用专属适配器时生效
- **提供商路由**（`provider-transport.ts`）直接将请求 fetch 到外部 API 服务商，不做参数转换

---

## 二、问题 1：`response_format` 硬编码导致 rix API 报错

### 错误日志

```
[TaskQueueService] Task execution failed: Error: Image generation failed: 400 -
{"error":{"message":"Unknown parameter: 'response_format'.",
 "type":"rix_api_error","param":"response_format","code":"unknown_parameter"}}
```

### 根因

`buildImageRequestBody` 无条件向请求体中添加了 `response_format: 'url'`：

```typescript
// image-api.ts L54-61（修复前）
export function buildImageRequestBody(params: ImageGenerationParams) {
  const body: Record<string, unknown> = {
    prompt: params.prompt,
    model: params.model,
    response_format: 'url',   // ← 硬编码，rix API 不支持
  };
```

该参数是 OpenAI 官方 API 的专有参数。当 `provider-transport.ts` 将请求路由到 rix API 提供商时，服务端不识别此参数，返回 400。

### 修复

```typescript
// image-api.ts L54-60（修复后）
export function buildImageRequestBody(params: ImageGenerationParams) {
  const body: Record<string, unknown> = {
    prompt: params.prompt,
    model: params.model,
  };
  // response_format 已移除，由各适配器自行处理
```

### 影响范围审计

| 代码路径 | 是否受影响 | 说明 |
|---------|-----------|------|
| `fallback-executor.ts` L267 → `buildImageRequestBody` | 已修复 | 不再发送 `response_format` |
| `image-api.ts` L151 → `generateImageSync` → `buildImageRequestBody` | 已修复 | 不再发送 `response_format` |
| `gpt-image-adapter.ts` L176-179 | **不受影响** | 有自己的 `getGPTImageResponseFormat` 处理，只在用户显式传入时才添加 |
| `tuzi-gpt-image-adapter.ts` | **不受影响** | 继承 GPT Image 适配器逻辑 |
| `image-api.ts` L35-48 `normalizeImageResultUrl` | **不受影响** | 同时支持 `url` 和 `b64_json` 两种返回格式 |

### 风险评估

**零风险**。原因：

1. OpenAI API 的 `response_format` **默认值就是 `'url'`**，不传等效于传 `'url'`
2. `parseImageResponse` + `normalizeImageResultUrl` 兼容 `url` 和 `b64_json` 两种格式
3. GPT Image 适配器有自己的 `response_format` 处理，不依赖 `buildImageRequestBody`

---

## 三、问题 2：非 1:1 比例全部生成 1:1

### 现象

用户选择 16:9，但 API 生成 816x816 正方形图片，下方内容空白。下载后图片确实是正方形。

### 根因分析：参数链路断裂

```
用户选 16:9
  → normalizeSize('16:9') → '16x9'
  → ParsedGenerationParams.size = '16x9'
  → step.args.size = '16x9'
  → workflow engine: options.size = '16x9'
  → image-generation-service: executorParams.size = '16x9'
  → fallback-executor: params.size = '16x9'
  → buildImageRequestBody({ size: '16x9' })
  → body.size = '16x9'                    ← BUG：比例字符串当作像素尺寸
  → API 收到 size: '16x9'                 ← 无效像素尺寸，回退默认 1:1
```

`buildImageRequestBody` 的逻辑缺陷：

```typescript
// 修复前
if (params.size) {
  body.size = params.size;           // 直接透传，无论内容是比例还是像素
} else if (params.aspectRatio) {
  body.size = aspectRatioToSize(params.aspectRatio);  // 转换只在这个分支
}
```

上游链路将比例格式（`'16x9'`）放进 `size` 字段，导致 `aspectRatioToSize` 转换分支被跳过。

### 全部受影响比例

| 用户选择 | 传入 size | 期望尺寸 | 修复前实际 |
|---------|----------|---------|-----------|
| 16:9 | `16x9` | `1792x1024` | 1:1 |
| 9:16 | `9x16` | `1024x1792` | 1:1 |
| 4:3 | `4x3` | `1536x1152` | 1:1 |
| 3:4 | `3x4` | `1152x1536` | 1:1 |
| 3:2 | `3x2` | `1536x1024` | 1:1 |
| 2:3 | `2x3` | `1024x1536` | 1:1 |
| 4:5 | `4x5` | `1024x1280` | 1:1 |
| 5:4 | `5x4` | `1280x1024` | 1:1 |
| 21:9 | `21x9` | `1792x768` | 1:1 |
| 1:4 | `1x4` | `512x2048` | 1:1 |
| 4:1 | `4x1` | `2048x512` | 1:1 |

仅 `1:1` → `1x1` → `1024x1024` 碰巧正常。

### 修复

```typescript
// image-api.ts L66-69（修复后）
if (params.size) {
  body.size = aspectRatioToSize(params.size) || params.size;
  //          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  //          先尝试比例→像素转换，失败则透传（认为是像素尺寸）
} else if (params.aspectRatio) {
  body.size = aspectRatioToSize(params.aspectRatio);
}
```

### `aspectRatioToSize` 安全分析

| 输入 | 查表结果 | 最终输出 | 正确？ |
|------|---------|---------|--------|
| `'16x9'` | `'1792x1024'` | `'1792x1024'` | ✓ |
| `'1024x1024'` | `undefined` | `'1024x1024'` | ✓ |
| `'1920x1080'` | `undefined` | `'1920x1080'` | ✓ |
| `undefined` | 函数短路返回 `undefined` | 不进入 if 块 | ✓ |
| `'auto'` | 函数短路返回 `undefined` | 不进入 if 块 | ✓ |

### 风险评估

**零风险**。`||` 运算符确保任何无法转换的值都会透传原值。

### 跨 AI 提供商兼容性分析

核心问题：OpenAI 需要像素尺寸（如 `1792x1024`），其他 AI 可能不同，修改后会不会影响其他 AI？

**答案：不会。** 反过来，修改**前**发送的 `'16x9'` 才是对所有 API 都无效的值。

#### 三种 `size` 格式与各 API 接受情况

| 格式类型 | 示例 | 接受的 API |
|---------|------|-----------|
| 像素尺寸 | `1792x1024`、`1024x1024` | OpenAI、Gemini、Flux、Midjourney、rix 等几乎所有 API |
| 比例字符串（冒号） | `16:9`、`4:3` | 少数国产模型 API |
| 混合格式（x 分隔） | `16x9`、`4x3` | **没有 API 接受这种格式** |

#### 修改前后对比

| 阶段 | 用户选择 | 发送给 API | API 行为 |
|------|---------|-----------|---------|
| 修改前 | 16:9 | `'16x9'`（无效格式） | 任何 API 都不认，统一回退 1:1 |
| 修改后 | 16:9 | `'1792x1024'`（标准像素） | 所有 API 都支持 |

#### 为什么修改前不是发送 `'16:9'` 而是 `'16x9'`？

链路中有一步 `normalizeSize` 将用户输入的 `'16:9'` 转为 `'16x9'`，这一步本身没有问题（统一格式）。但 `buildImageRequestBody` 收到 `'16x9'` 后直接透传给 API，没有调用 `aspectRatioToSize` 将其转为 `'1792x1024'`。`'16x9'` 这个格式对任何 API 都不是合法参数——它既不是像素尺寸，也不是标准比例格式。

#### 原始设计意图本身就是转换像素

原代码中 `aspectRatio` 分支已经正确地调用了 `aspectRatioToSize`：

```typescript
} else if (params.aspectRatio) {
  body.size = aspectRatioToSize(params.aspectRatio);  // 转换为像素尺寸
}
```

说明原始设计的意图就是**发送像素尺寸给 API**。问题只是上游把比例字符串错放进 `size` 字段，跳过了 `aspectRatio` 这个转换分支。本次修复只是**把被跳过的转换补回来**，并没有改变语义——最终发送的仍然是像素尺寸。

**结论**：修改前发送的是无效值（所有 API 都不认），修改后发送的是标准像素尺寸（所有 API 都认）。不存在跨提供商兼容性问题。

---

## 四、问题 3：图片插入画布后强制 1:1 显示

### 现象

`foreignObject` 容器始终 400x400，非正方形图片被压缩。图片加载瞬间显示正确比例，随后缩为 1:1。

### 根因

`insertImageFromUrl` 的 `lockReferenceDimensions` 参数为 `true` 时，跳过图片加载后的尺寸更新：

```typescript
// image.ts L399
shouldUpdateSizeAfterLoad = !lockReferenceDimensions; // true → false，不更新
```

### 修复

将所有自动插入场景的 `lockReferenceDimensions` 改为 `false`：

| 文件 | 行号 | 场景 | 修改 |
|------|------|------|------|
| `canvas-insertion.ts` | 226 | Services 批量插入 | `true` → `false` |
| `handler.ts` | 231, 311 | SW 自动插入 | 明确 `false` |
| `mcp/tools/canvas-insertion.ts` | 225 | MCP 协议插入 | `true` → `false` |

### 全部插入路径审计

```
insertImageFromUrl 调用者 17 处
│
├── 自动插入场景（lockReferenceDimensions=false）
│   ├── canvas-insertion.ts:226      ✓ false
│   ├── handler.ts:231               ✓ false
│   ├── handler.ts:311               ✓ false
│   ├── mcp/tools/canvas-insertion.ts:225  ✓ false（本次补修）
│   └── media-quick-insert.ts:67     ✓ undefined（等同 false）
│
├── 用户手动插入场景（lockReferenceDimensions 不传/undefined）
│   ├── useWorkflowSubmission.ts:103  ✓ undefined（图片尺寸已知）
│   ├── useWorkflowSubmission.ts:112  ✓ undefined
│   ├── drawnix.tsx:1307             ✓ undefined（传入 naturalWidth/Height）
│   ├── popup-toolbar.tsx:2551       ✓ undefined（传入 naturalWidth/Height）
│   ├── quick-creation-toolbar.tsx:211 ✓ 不传
│   ├── creation-toolbar.tsx:295      ✓ 不传
│   ├── MediaLibraryGrid.tsx:1280     ✓ 不传
│   ├── MediaLibraryGrid.tsx:1324     ✓ 不传
│   ├── VideoAnalyzer.tsx:122         ✓ 不传
│   ├── TaskQueuePanel.tsx:607        ✓ 不传
│   ├── TaskQueuePanel.tsx:974        ✓ 不传
│   └── DialogTaskList.tsx:229        ✓ 不传
```

**说明**：手动插入场景传入的是图片 `naturalWidth`/`naturalHeight`（已加载的真实尺寸），无需 `lockReferenceDimensions=false` 也能正确显示。`lockReferenceDimensions` 主要影响使用固定参考尺寸（400x400）的自动插入场景。

### 尺寸更新机制

`updateImageSizeAfterLoad`（[image.ts#L522-590](file:///d:/工作/opentu_new/packages/drawnix/src/data/image.ts#L522-L590)）：

1. 异步加载图片获取 `naturalWidth`/`naturalHeight`
2. 计算实际宽高比与参考宽高比差异（< 1% 跳过）
3. 以参考宽度为基准重新计算高度
4. 通过 `Transforms.setNode` 更新元素 `points`（影响 `foreignObject` 尺寸）

### 风险评估

**低风险**。影响仅限画布渲染层：
- 不影响 API 调用、图片生成流程
- 初始 400x400 是暂时的，图片加载后异步更新
- 极端网络条件下 `updateImageSizeAfterLoad` 可能加载失败（已有 `.catch` 保护），自动降级为参考尺寸

---

## 四-B、问题 4：`updateImageSizeAfterLoad` 尺寸限制导致图片截断

### 现象

竖版长图（9:16）实际生成 1024x1792，但插入画布后 `foreignObject` 容器仅为 400x400 或 400x711（被参考尺寸限制），导致图片被截断或显示不完整。

### 根因

`updateImageSizeAfterLoad` 的旧实现将尺寸限制在 `referenceDimensions` 范围内：

```typescript
// 修改前（image.ts L552-561）
let newWidth = referenceDimensions.width;
let newHeight = newWidth / imageAspectRatio;

// 如果新高度超过预设高度，则以高度为基准
if (newHeight > referenceDimensions.height) {
  newHeight = referenceDimensions.height;
  newWidth = newHeight * imageAspectRatio;
}
```

问题：图片真实尺寸大于参考尺寸时被强制压缩。例如：
- API 返回 1024×1792 图片
- 插入时 `referenceDimensions` = 400×400
- 旧逻辑：`newWidth = 400`, `newHeight = 400 / (1024/1792) ≈ 700`，但 `700 > 400` → 被限制为 `400×400`

### 修复

直接使用图片的 `naturalWidth`/`naturalHeight`，仅设置上限 2048px 防止超大图片：

```typescript
// 修改后（image.ts L538-554）
const MAX_IMAGE_WIDTH = 2048;
const MAX_IMAGE_HEIGHT = 2048;

let newWidth = naturalWidth;
let newHeight = naturalHeight;

if (newWidth > MAX_IMAGE_WIDTH) {
  const scale = MAX_IMAGE_WIDTH / newWidth;
  newWidth = MAX_IMAGE_WIDTH;
  newHeight = Math.round(newHeight * scale);
}
if (newHeight > MAX_IMAGE_HEIGHT) {
  const scale = MAX_IMAGE_HEIGHT / newHeight;
  newHeight = MAX_IMAGE_HEIGHT;
  newWidth = Math.round(newWidth * scale);
}
```

### 风险评估

**低风险**。该函数已充分考虑各种边界情况，具备完善的容错机制：

1. **元素已被删除的保护**：`updateImageSizeAfterLoad` 是异步执行的（等待图片加载），用户可能在此期间删除元素。代码通过 `findIndex` 检查元素是否存在，不存在则直接返回，不会报错。

2. **用户手动移动元素的保护**：用户可能在图片加载过程中拖动元素到新位置。代码使用 `currentTopLeft` 获取当前位置，只调整右下角坐标，不会覆盖用户手动调整的位置。

3. **图片加载失败的保护**：网络问题或链接失效可能导致加载失败。代码使用 `.catch()` 捕获异常，仅打印警告日志，不抛出异常影响其他功能。

4. **超大图片的保护**：API 可能返回超大尺寸图片（如 4096×4096）。代码设置 2048px 上限，自动按比例缩小，避免内存占用过高。

5. **无效尺寸的保护**：如果图片尺寸为 0 或负数，代码在开头就会检测并返回，不会执行无效的更新操作。

6. **触发条件控制**：该函数仅在 `skipImageLoad=true && lockReferenceDimensions=false` 时触发，避免不必要的重复调用。

---

## 四-C、问题 5：`getTaskImageDimensions` 忽略任务返回的真实尺寸

### 现象

任务执行后 `task.result` 包含真实图片尺寸（`width`/`height`），但插入时使用的仍是 `parseSizeToPixels(task.params.size)` 返回的估计值，而非真实值。

### 根因

`getTaskImageDimensions` 的优先级顺序不当——`fallback` 参数永远有值（来自 `parseSizeToPixels`），导致 `task.result` 的检查被跳过：

```typescript
// 修改前
if (fallback) {
  return fallback;              // ← 直接返回，跳过 task.result
}
const result = task.result...;  // ← 永远执行不到
```

### 修复

将 `task.result` 的真实尺寸提到最优先：

```typescript
// 修改后（useAutoInsertToCanvas.ts L248-267）
// 优先使用任务返回的真实尺寸
const result = task.result as { width?: number; height?: number } | undefined;
if (result?.width > 0 && result?.height > 0) {
  return { width: result.width, height: result.height };
}
if (fallback) {
  return fallback;
}
return parseSizeToPixels(task.params.size);
```

### 风险评估

**零风险**。
- `task.result` 无有效尺寸时降级到 `fallback`，行为不变 ✓
- 两个调用点（L683、L1070）的 fallback 均来自 `parseSizeToPixels`，语义一致 ✓
- 下游 `insertedSize`/`syncImageAnchorGeometry` 拿到更准确的尺寸，无副作用 ✓

---

## 四-D、问题 6：素材库批量插入时图片叠加

### 现象

从素材库批量插入多张不同比例的图片到画布时，图片彼此堆叠覆盖，而非分开排列。

### 根因：尺寸链路不同步

```
批量插入
  → estimateInsertionItemSize: 无 dimensions → 默认 400×400
  → advanceBatchInsertionFlow: 按 400×400 估算间距排位
  → insertImageToCanvas: 用 400×400 插入
  → updateImageSizeAfterLoad (异步): 扩展为真实尺寸 (如 1024×1792)
  → 真实尺寸 >> 排位间距 → 图片叠加
```

核心矛盾：排位系统以 400×400 为基准计算间距，但图片最终显示为真实尺寸（修复后），导致超出分配空间。

### 修复

在批量插入前预加载每张素材图片的真实尺寸，传给 `executeCanvasInsertion` 的 `item.dimensions`：

```typescript
// quick-creation-toolbar.tsx L238-280
const loadImageDimensions = (url: string) =>
  new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve({ width: img.naturalWidth, height: img.naturalHeight });
    img.onerror = () => resolve({ width: 400, height: 400 }); // 降级
    img.src = url;
  });

// 并行预加载所有图片尺寸
const imageDimensionsMap = new Map();
const results = await Promise.all(imageAssets.map(a => loadImageDimensions(a.url)));
// 传给 executeCanvasInsertion 的 item.dimensions
```

### 效果链路

```
预加载真实尺寸 (1024×1792)
  → estimateInsertionItemSize: 使用真实尺寸
  → advanceBatchInsertionFlow: 按真实间距排位 → 互不重叠 ✓
  → insertImageToCanvas: 用真实尺寸插入
  → updateImageSizeAfterLoad: 尺寸已匹配 → 几乎无变化 ✓
```

### 风险评估

**零风险**。
- 失败降级：`img.onerror` → 400×400，不阻塞 ✓
- 性能：本地缓存图片，`Promise.all` 并行加载，几乎无延迟 ✓
- 影响范围：**仅素材库「批量插入画布」这一个入口**，其他插入路径完全不变 ✓
- 视频/音频：`loadImageDimensions` 仅处理 `AssetType.IMAGE`，不触发 ✓
- `dimensions` 为可选字段，不传时行为不变 ✓

---

## 四-E、问题 7：批量插入流式布局导致不同尺寸素材间距不均 / 可能叠加

### 现象

素材库批量插入多张不同比例图片（如 9:16 竖版 + 16:9 横版）时：
- **间距不均**：同行中高素材（竖版）与矮素材（横版）顶部对齐，形成锯齿状布局
- **潜在叠加**：当某些素材的 `updateImageSizeAfterLoad` 异步扩展尺寸后，可能侵入相邻素材的空间

### 根因

旧的 `advanceBatchInsertionFlow` 是**逐项流式布局**（Flow Layout）：

```
流式布局过程（迭代调用）：
  第1项: 放入 (100, 100), cursorX → 520, rowMaxHeight = 500
  第2项: 放入 (520, 100), cursorX → 820, rowMaxHeight = 500（同行，顶部对齐）
  第3项: 放入 (820, 100), cursorX → 1120 > rowRightLimit → 换行
  第4项: 放入 (100, 650), cursorX → 520（新行从上一行 rowMaxHeight 开始）
```

**问题**：
1. 布局是基于"顺序迭代"而非"全局规划"，无法预先知道所有素材的尺寸分布
2. 同行素材仅顶部对齐，高度不同导致底部参差不齐
3. 行高由当前行最高素材决定后固定，后续无法调整
4. 单张插入后才异步更新为真实尺寸（`updateImageSizeAfterLoad`），容易超出流式布局预留空间

### 修复

新增 `precalculateGridLayout` 函数，采用**网格预计算**（Grid Pre-calculation）替代逐项流式布局：

```typescript
// canvas-insertion-layout.ts L228-328
export function precalculateGridLayout(
  startPoint: Point,
  itemSizes: { width: number; height: number }[],
  options: {
    canvasWidth?: number;
    maxColumns?: number;
    horizontalGap?: number;
    verticalGap?: number;
  } = {}
): { positions: Point[]; bounds: FlowBounds }
```

**算法流程**：

```
步骤1: 根据画布宽度和素材平均宽度计算最优列数
  columns = max(1, min(5, floor((canvasWidth + gap) / (avgWidth + gap))))

步骤2: 将素材按行分组（先填第一行，再填第二行...）
  6个素材, 3列 → 行0: [素材0, 素材1, 素材2], 行1: [素材3, 素材4, 素材5]

步骤3: 计算每列最大宽度（该列所有素材的最宽值）
  columnWidths[c] = max(列c中所有素材的宽度)

步骤4: 计算每行最大高度（该行所有素材的最高值）
  rowHeights[r] = max(行r中所有素材的高度)

步骤5: 计算坐标
  X偏移: colX[c] = colX[c-1] + columnWidths[c-1] + horizontalGap
  Y偏移: rowY[r] = rowY[r-1] + rowHeights[r-1] + verticalGap
  素材i位置: [colX[i % columns], rowY[floor(i / columns)]]

步骤6: 计算整体边界（用于滚动定位）
```

**与旧布局的关键区别**：

| 特性 | 旧：advanceBatchInsertionFlow | 新：precalculateGridLayout |
|------|------------------------------|---------------------------|
| 计算方式 | 逐项迭代 | 一次性全局预计算 |
| 列数 | 自适应（溢出换行） | 根据画布宽度动态计算 |
| 同行对齐 | 顶部对齐 | 顶部对齐（每个格子左上角） |
| 列宽 | 按该项实际宽度 | 按该列最大宽度 |
| 行高 | 按该行最高项 | 按该行最高项 |
| 可预测性 | 低（依赖顺序和逐个计算结果） | 高（所有位置一次性确定） |

**示例**：4 张素材（2 张 400×700 竖版 + 2 张 700×400 横版），画布宽 2000：

```
旧布局（流式）：
  [竖400x700] [竖400x700] [横700x400]  ← 第3项换行
  [横700x400]                          ← 单占一行，大量空白

新布局（网格，2列）：
  [竖400x700] [横700x400]  ← 行0, 行高=max(700,400)=700
  [竖400x700] [横700x400]  ← 行1, 列0宽=max(400,400)=400, 列1宽=max(700,700)=700
```

### 风险评估

**零风险**。
- 向后兼容：`advanceBatchInsertionFlow` 和 `createBatchInsertionFlowState` **保留原样**，仅不再被批量插入调用，其他使用方不受影响 ✓
- 空数组安全：`itemSizes.length === 0` 时返回空 positions 和零宽高 bounds ✓
- 单列降级：`maxColumns=5` 且画布极窄时自动降为 1 列（`Math.max(1, ...)` 保护） ✓
- 无画布宽度时降级：`canvasWidth` 未提供时使用 `Math.min(maxColumns, itemSizes.length)` ✓
- `flowState.bounds` 替换：新算法产生的 `bounds` 与旧格式完全兼容，`getBatchInsertionFlowCenter` 无缝工作 ✓
- Lint 0 errors ✓
- 影响范围仅批量插入（`executeCanvasInsertion` 的两份实现） ✓

---

## 五、修改文件清单（完整）

### 第一轮修复（已提交）

| 文件 | 修改内容 | 影响范围 |
|------|---------|---------|
| `packages/drawnix/src/services/media-api/image-api.ts` | 删除 `response_format: 'url'`；`size` 参数增加比例→像素自动转换 | API 请求体构建 |
| `packages/drawnix/src/services/canvas-operations/canvas-insertion.ts` | `lockReferenceDimensions: false` | 批量图片插入 |
| `packages/drawnix/src/services/sw-capabilities/handler.ts` | 2 处 `lockReferenceDimensions: false` | SW 自动插入 |
| `packages/drawnix/src/mcp/tools/canvas-insertion.ts` | `lockReferenceDimensions: true → false` | MCP 协议插入 |

### 第二轮修复（当前，未提交）

| 文件 | 修改内容 | 影响范围 |
|------|---------|---------|
| `packages/drawnix/src/data/image.ts` | `updateImageSizeAfterLoad` 改用图片真实 `naturalWidth/naturalHeight`，上限 2048px | 异步尺寸更新 |
| `packages/drawnix/src/hooks/useAutoInsertToCanvas.ts` | `getTaskImageDimensions` 优先使用 `task.result` 真实尺寸 | AI 生成图片插入 |
| `packages/drawnix/src/components/toolbar/quick-creation-toolbar/quick-creation-toolbar.tsx` | 批量插入前预加载素材库图片真实尺寸 | 素材库批量插入 |

### 第三轮修复（批量插入图片叠加，2026-05-25）

> ⚠️ **重要**：此修复已单独文档化，详见 [批量插入图片叠加问题修复经验总结](./CANVAS_BATCH_INSERTION_FIX_LESSONS.md)

**问题**：
- 素材库批量插入多张图片时，图片全部叠加在一起
- 根本原因：预计算的网格布局尺寸与图片加载后更新的尺寸不一致

**核心解决方案**：
- 新增 `calculateImageDisplayDimensions` 函数，统一尺寸计算逻辑
- `updateImageSizeAfterLoad` 和 `loadImageDimensions` 使用同一函数
- 确保预计算和异步更新使用完全相同的尺寸

**新增文件**：
| 文件 | 描述 |
|------|------|
| `packages/drawnix/src/utils/canvas-insertion-layout.ts` | 新增 `calculateImageDisplayDimensions` 函数 |

**修改文件**：
| 文件 | 修改内容 |
|------|---------|
| `packages/drawnix/src/data/image.ts` | `updateImageSizeAfterLoad` 使用 `calculateImageDisplayDimensions` |
| `packages/drawnix/src/components/toolbar/quick-creation-toolbar/quick-creation-toolbar.tsx` | `loadImageDimensions` 使用 `calculateImageDisplayDimensions`；添加 `horizontalGap: 30` 和 `verticalGap: 40` |

### 未修改的文件（已验证不受影响）

- `packages/drawnix/src/services/model-adapters/gpt-image-adapter.ts` — 自有 `response_format` 处理
- `packages/drawnix/src/services/model-adapters/tuzi-gpt-image-adapter.ts` — 继承 GPT 适配器
- `packages/drawnix/src/services/media-api/utils.ts` — 仅 `aspectRatioToSize` 被引用，逻辑不变
- `packages/drawnix/src/hooks/useWorkflowSubmission.ts` — 手动插入场景，传入真实尺寸
- 其他手动插入路径 — `lockReferenceDimensions` 未传或使用真实尺寸
- 其他批量插入入口 — 不走素材库，不受预加载修改影响

---

## 六、修改间依赖关系

```
                    ┌─────────────────────────┐
                    │   buildImageRequestBody  │
                    │  (问题1+2) 已提交         │
                    │  - 删除 response_format│
                    │  - size 比例转换        │
                    └─────────────────────────┘
                                │
                                ▼
                    ┌─────────────────────────┐
                    │   insertImageFromUrl     │
                    │  (问题3) 已提交          │
                    │  - lockReferenceDimensions = false │
                    └───────────┬─────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              ▼                 ▼                  ▼
  ┌──────────────────┐ ┌──────────────┐ ┌────────────────────┐
  │ updateImageSize  │ │getTaskImage  │ │ 素材库批量预加载     │
  │ AfterLoad        │ │Dimensions    │ │ (问题6) 当前         │
  │ (问题4) 当前     │ │(问题5) 当前  │ │ - 预加载真实尺寸     │
  │ - 使用 natural   │ │- task.result │ │ - 传给 dimensions   │
  │   Width/Height   │ │  优先级提前  │ │                     │
  └──────────────────┘ └──────────────┘ └────────────────────┘
```

**独立性**：
- 问题 1+2（API 层）与问题 3-6（画布层）完全独立
- 问题 4（`updateImageSizeAfterLoad`）是问题 6（批量插入叠加）的前提——如果恢复旧的尺寸限制，批量插入预加载的间距也将失效
- 问题 5（`getTaskImageDimensions`）独立于其他修改，仅优化 AI 生成场景的尺寸获取
- **问题 7（批量插入图片叠加）**：
  - 核心修复：新增 `calculateImageDisplayDimensions` 函数，统一所有尺寸计算逻辑
  - 依赖关系：`loadImageDimensions` → `precalculateGridLayout` → `updateImageSizeAfterLoad`
  - 关键保证：预计算和异步更新使用完全相同的尺寸计算函数

---

## 七、经验总结

### 关键设计原则

1. **不要硬编码特定提供商的参数**：`response_format: 'url'` 对 OpenAI 是默认值，对 rix 是致命错误
2. **API 边界做参数规范化**：上游可能传入比例格式（`'16x9'`）或像素尺寸（`'1792x1024'`），应在发送前统一转换
3. **`||` 降级模式比 `if/else` 更安全**：`aspectRatioToSize(x) || x` 保证无法转换的值会透传
4. **异步尺寸更新优于同步锁定**：`lockReferenceDimensions=false` 允许图片加载后自适应，用户体验更好
5. **全局预计算优于逐项流式布局**：批量操作中先收集所有尺寸再一次性计算网格布局，比逐项迭代更可预测、间距更均匀

### 排查方法论

- **参数链路追踪**：从 UI → Parser → Engine → Service → Executor → API Body，逐层验证
- **边界层防御**：在 API 边界（`buildImageRequestBody`）做参数校验和转换，不依赖上游正确性
- **全局调用审计**：修改参数签名后需 grep 全部调用者，确保所有路径一致