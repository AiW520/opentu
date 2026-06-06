# 聊天气泡图片预览经验总结

更新日期：2026-05-17

## 背景

聊天抽屉里的用户图片最初沿用了粗橙色外框和局部预览逻辑，结果出现两个问题：

1. 视觉上太重，和抽屉整体的轻量信息流不一致。
2. 图片在气泡里容易被裁切，用户看不到完整内容。

这类问题不要只改 CSS。只要预览器还散落在气泡内部，后续就很容易继续出现实例重复、交互不统一、内存负担变大等问题。

## 架构变更

这次把链路收成了两层：

1. `UserMessageBubble` 只负责识别图片和上抛点击事件。
2. `ChatMessagesArea` 统一持有 `UnifiedMediaViewer`，负责真正打开预览。

这样做的好处是：

- 预览器只保留一份实例，不会每条消息都挂一个。
- 图片点击、关闭、切换索引都在消息列表层统一处理。
- 气泡组件重新回到“纯展示 + 事件上抛”的职责。

## 交互经验

- 图片展示用 `object-fit: contain`，优先保证完整可见。
- 图片预览入口用单击，比双击更贴近用户直觉。
- 图片区域用 pointer cursor，语义上直接告诉用户“这里能打开预览”。
- 图片节点禁止拖拽，避免和单击预览冲突。

## 视觉经验

- 用户消息不再使用粗橙渐变外框，改成更轻的中性色边框和阴影。
- 图片和文本的视觉层次要分开，图片负责内容本体，文本负责输入信息。
- 选择态、hover 态都要克制，不要让一个轻量聊天气泡看起来像大卡片。

## 经验规则

1. 图片预览器尽量放在列表层或页面层统一管理，不要塞进单条消息内部。
2. 图片默认以完整显示优先，裁切只应是明确的产品选择。
3. 图片交互语义要单一，单击负责打开预览，别再叠加多套入口。
4. 气泡组件只保留事件和内容解析，不承担全局预览状态。

## 验证结果

- `UserMessageBubble.test.tsx` 通过。
- `ChatMessagesArea.test.tsx` 通过。
- `tsc -p packages/drawnix/tsconfig.json --noEmit` 通过。

## 相关文件

- `packages/drawnix/src/components/chat-drawer/UserMessageBubble.tsx`
- `packages/drawnix/src/components/chat-drawer/ChatMessagesArea.tsx`
- `packages/drawnix/src/components/chat-drawer/user-message-bubble.scss`
- `packages/drawnix/src/components/chat-drawer/__tests__/UserMessageBubble.test.tsx`
- `packages/drawnix/src/components/chat-drawer/__tests__/ChatMessagesArea.test.tsx`
