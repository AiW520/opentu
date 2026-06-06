## 1. Specification

- [x] 1.1 新增 `ui-color-system` OpenSpec 变更说明
- [x] 1.2 定义全局 UI token、素材库状态 token、AI 编程 token 优先级要求
- [x] 1.3 为每个 requirement 补充至少一个 Scenario

## 2. Documentation

- [x] 2.1 新增 `docs/UI_COLOR_SYSTEM.md`
- [x] 2.2 说明 token 使用边界
- [x] 2.3 说明 hover、selected、focus 默认规范
- [x] 2.4 明确禁止新增重黑遮罩、黑色主按钮、孤立蓝色 focus

## 3. Implementation

- [x] 3.1 在全局样式与 Drawnix/TDesign 主题中新增 `--aitu-ui-*` token
- [x] 3.2 将素材库卡片 hover、selected、focus、overlay、badge、checkbox 状态切换为 token
- [x] 3.3 将素材库工具栏、筛选器、详情栏、窗口、虚拟框选、存储条和视图切换切换为 token
- [x] 3.4 移除素材库组件内局部 `$brand-orange` 与黑色主按钮/重黑遮罩用法
- [x] 3.5 收敛素材库多选操作区，避免重复品牌主按钮与选中整图白色蒙层

## 4. Validation

- [x] 4.1 运行 `openspec validate update-ui-color-system --strict`
- [x] 4.2 运行素材库样式静态扫描，确认无局部 `$brand-orange`、黑色主按钮、重黑遮罩
- [x] 4.3 运行最小类型/构建检查
