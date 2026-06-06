# Change: update UI color system

## Why

素材库当前存在偏黑偏灰的 hover 与选中态，视觉反馈显得突兀且不自然，和品牌橙主色的产品识别不一致。

需要建立全局 UI 配色 token 规范，让素材库、AI 编程生成的界面和后续组件改造优先使用统一品牌橙 token，减少局部硬编码颜色继续扩散。

## What Changes

- 新增 `ui-color-system` 规范，定义全局 UI token 的使用边界
- 明确素材库 hover、selected、focus 等状态必须使用统一 token
- 明确 AI 编程 UI 规范优先使用 token，避免新增重黑遮罩、黑色主按钮和孤立蓝色 focus
- 新增中文文档 `docs/UI_COLOR_SYSTEM.md`，作为实现与评审时的配色约束说明

## Impact

- Affected specs: `ui-color-system`
- Affected docs:
  - `docs/UI_COLOR_SYSTEM.md`
