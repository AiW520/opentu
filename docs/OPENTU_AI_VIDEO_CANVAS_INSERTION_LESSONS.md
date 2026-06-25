# AI 视频生成与白板插入修复经验

更新日期：2026-06-16

## 背景

AI 视频生成完成后，用户在任务面板点击插入白板，可能遇到两类问题：

- 视频接口请求被拼成相对地址，例如 `POST /v1/videos`，导致生成失败。
- 视频插入白板后显示为空白，或在弹窗内点击插入时没有进入当前选中的 Frame。

本次修复覆盖功能代码链路：视频接口路由、视频元素识别、视频首帧显示、Frame 插入目标保持，以及桌面运行时 Service Worker 注册兜底。

## 修复思路

### 1. Provider baseUrl 必须归一为绝对地址

历史设置中可能存在 `/v1` 这类相对 `baseUrl`。当视频生成走 `/v1/videos` 时，相对地址会被浏览器解释为当前站点路径，最终触发 `Invalid URL` 或错误请求。

修复原则：

1. 设置管理层读取路由时先校验 `baseUrl` 是否是 `http/https` 绝对地址。
2. 非法或相对地址回退到默认 Tuzi provider 地址。
3. 执行器最终发请求前再次兜底，避免遗漏入口把相对 URL 传入底层请求。

### 2. 视频 URL 标识要集中处理

旧逻辑直接用 `#video` 判断视频，容易破坏带 hash、签名参数或合并视频标识的 URL。修复后把视频 URL 能力集中到 `utils/video-url.ts`：

- `markVideoUrl` 负责安全添加视频标识。
- `stripVideoUrlMarker` 负责播放前移除画布内部标识。
- `isVideoLikeElement` 统一判断视频元素，支持 `isVideo`、`videoType`、虚拟路径和常见视频扩展名。
- `createVideoImageItem` 创建带视频元数据的 image 元素，并保留可选 poster。

入口层不要继续手写 `url.includes('#video')` 或拼接 `${url}#video`。

### 3. 白板视频不能依赖浏览器默认首帧

视频资源实际已经加载成功时，暂停在 `0s` 仍可能显示黑屏或空白。修复后：

- 视频组件加载元数据后，如果没有 poster，自动 seek 到 `0.1s` 作为暂停预览帧。
- 任务结果插入白板时传递 `thumbnailUrl`、`thumbnailUrls` 或 `previewImageUrl` 作为 poster。
- 素材库插入视频时传递素材 `thumbnail`。
- 旧 video 元素迁移为 image+video 元数据时不再删除 poster。

这样既支持有封面的视频，也支持没有封面但可 seek 的远程视频。

### 4. 弹窗点击会清空画布选中态，需要保留最近非空选中

AI 视频弹窗打开后，点击弹窗内“插入”会让画布当前 selection 变空。旧逻辑只读当前选中或 `lastSelectedElementIds`，因此插入无法命中原先选中的 Frame，只能落到 Frame 下方。

修复后新增 `lastNonEmptySelectedElementIds`：

- 画布选中非空时更新。
- 选中被弹窗交互清空时不覆盖。
- Frame 插入逻辑优先使用当前选中，其次使用最近非空选中，最后兼容旧的 `lastSelectedElementIds`。

### 5. 桌面运行时跳过 Service Worker

桌面运行时加载本地构建产物时，旧 Service Worker 可能请求 `sw.js` 得到 HTML，从而出现 unsupported MIME type 或 404 风险。修复后 Tauri 环境：

- 不注册 Service Worker。
- 若已有注册，启动时注销。

这属于桌面运行时启动逻辑，用于避免本地运行时被旧 Service Worker 干扰。

## 实现要点

- `settings-manager.ts`：新增 provider baseUrl 归一化，避免相对地址进入调用路由。
- `fallback-executor.ts`：执行器层增加绝对 URL 兜底，视频接口统一 trim `/v1` 后拼接。
- `video-url.ts`：集中视频标识、识别、播放 URL 清洗和视频 image 元素创建。
- `with-video.ts`：复用统一视频识别逻辑，迁移旧 video 元素时保留 poster。
- `video.ts`：插入视频时写入 `isVideo`、`videoType`、尺寸和 poster 元数据。
- `VideoPosterPreview.tsx` / `plugins/components/video.tsx`：播放前剥离内部视频标识，支持首帧兜底。
- `TaskQueuePanel.tsx` / `DialogTaskList.tsx` / 工具栏素材插入：传递任务或素材封面给白板视频。
- `drawnix.tsx` / `use-drawnix.tsx` / `frame-insertion-utils.ts` / `canvas-insertion-layout.ts`：保留最近非空选中 Frame，保证弹窗内插入回到原目标。
- `bootstrap.tsx`：Tauri 运行时跳过并清理 Service Worker。
- `virtual-url-interceptor.ts`：虚拟媒体 URL 同时支持 `src` 和 `poster`，并按属性释放 Object URL。

## 注意事项

- 不要在调用点直接拼 `#video`，统一用 `createVideoImageItem` 或 `markVideoUrl`。
- 不要在播放组件中直接使用带内部标识的 URL，统一用 `stripVideoUrlMarker`。
- 弹窗、任务面板、素材库等会抢焦点的入口，插入位置应依赖最近非空选中态，而不是只读当前 selection。
- 远程签名视频可能拒绝 canvas 截帧或跨域读取，首帧兜底采用 video seek，不强行把视频读入内存。
- poster Object URL 要按 `src` / `poster` 分别管理，避免释放 src 时误释放 poster，或反过来造成泄漏。

## 验证建议

1. 精确单测：

```bash
pnpm exec vitest run \
  packages/drawnix/src/utils/__tests__/settings-manager.test.ts \
  packages/drawnix/src/services/__tests__/media-api-routing.test.ts \
  packages/drawnix/src/services/__tests__/media-executor.test.ts \
  packages/drawnix/src/utils/__tests__/video-url.test.ts \
  packages/drawnix/src/data/__tests__/video.test.ts \
  packages/drawnix/src/utils/__tests__/frame-insertion-utils.test.ts \
  packages/drawnix/src/utils/__tests__/canvas-insertion-layout.test.ts
```

2. 手动回归：

- 生成 AI 视频，确认请求不再出现 `POST /v1/videos` 相对路径。
- 点击任务面板插入视频，白板中应显示封面或首帧。
- 选中 Frame 后打开 AI 视频弹窗，再点击任务插入，视频应进入选中的 Frame。
- 插入素材库视频，若素材有 thumbnail，应作为白板视频 poster。
- 在桌面运行时启动应用，不应重复注册 Service Worker。

## 提交备注模板

```text
问题描述:
- AI 视频生成可能因相对 baseUrl 拼出 /v1/videos 导致请求失败。
- 视频插入白板后缺少 poster 或首帧，暂停状态看起来为空白。
- 弹窗点击插入会清空画布选中态，导致视频无法插入原选中 Frame。

修复思路:
- 在设置路由和执行器两层兜底 provider baseUrl，保证请求使用绝对地址。
- 收敛视频 URL 标识和清洗逻辑，插入时保留视频元数据与 poster。
- 视频组件在无 poster 时自动 seek 到 0.1s 作为预览帧。
- 保存最近一次非空选中元素，弹窗插入时仍能定位目标 Frame。

更新代码架构:
- 新增 video-url 工具统一处理视频 URL 标识、识别和元素创建。
- Frame 插入与普通视频插入复用统一视频 image 元数据。
- 桌面运行时启动逻辑跳过并清理 Service Worker，避免本地运行 SW 干扰。
```
