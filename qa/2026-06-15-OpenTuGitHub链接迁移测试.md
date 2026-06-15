# GitHub 链接迁移测试文档

> 生成时间：2026-06-15
> 测试范围：GitHub 外链迁移、Tauri 桌面端外链打开、静态页面旧仓库地址清理
> 目标平台：Web、macOS 桌面端

---

## 背景与范围

本轮测试覆盖以下改动：

- 应用内 GitHub 入口迁移到 `AiW520/opentu`。
- 桌面端外链统一优先走 Tauri opener。
- README、官网静态页、用户手册、版本页和 package 元数据清理旧 GitHub 地址。
- Gist、GitHub、模型文档等外链入口统一通过 `openExternalUrl` 打开。

不覆盖：

- GitHub Token 权限有效性。
- Gist 服务端数据一致性。
- 自动更新发布流程。

---

## 测试环境

| 项目 | 值 |
|------|----|
| 日期 | 2026-06-15 |
| Web 环境 | 浏览器 |
| 桌面环境 | macOS Tauri |
| 目标仓库 | `https://github.com/AiW520/opentu` |
| 核心工具 | `openExternalUrl` |

---

## 覆盖矩阵

| 模块 | 测试点 | 状态 |
|------|--------|------|
| 工具栏 GitHub | 点击后打开当前仓库 | 待人工复验 |
| 嵌入态工具栏 GitHub | 点击后打开当前仓库 | 待人工复验 |
| 同步设置 Gist 链接 | 桌面端使用系统浏览器打开 | 待人工复验 |
| 回收站 Gist 文件链接 | 桌面端使用系统浏览器打开 | 待人工复验 |
| 模型文档链接 | Web 和桌面端均可打开外部文档 | 待人工复验 |
| README / 官网静态页 | 旧仓库地址清理 | 已静态检查 |
| 用户手册 / 版本页 | 相对站内链接仍可访问 | 待人工复验 |

---

## 手动测试用例

### TC-01：工具栏 GitHub 入口

**前置条件**

- 启动 Web 或桌面端。
- 系统默认浏览器可用。

**步骤**

1. 打开应用主界面。
2. 点击工具栏或菜单中的 GitHub 入口。
3. 观察打开地址。

**预期结果**

- 打开 `https://github.com/AiW520/opentu`。
- 桌面端没有弹出空白 WebView。
- 控制台没有外链协议错误。

### TC-02：同步设置中的 Gist 外链

**前置条件**

- 已配置 GitHub Gist 同步，或页面中存在可展示的 Gist 链接。

**步骤**

1. 打开同步设置。
2. 点击“在 GitHub 查看”。
3. 观察系统浏览器打开地址。

**预期结果**

- Gist 链接在系统浏览器打开。
- 应用窗口不被错误导航到 GitHub。

### TC-03：回收站中的 Gist 文件链接

**前置条件**

- 回收站中存在来自 Gist 的文件记录。

**步骤**

1. 打开回收站。
2. 点击 Gist 文件链接。
3. 观察打开地址。

**预期结果**

- 链接通过系统浏览器打开。
- 链接仍指向对应 Gist 文件。

### TC-04：模型文档链接

**步骤**

1. 打开模型选择或模型配置入口。
2. 点击模型文档链接。
3. 分别在 Web 和桌面端观察打开方式。

**预期结果**

- 外部文档链接可以正常打开。
- 桌面端优先打开系统浏览器。

### TC-05：站内相对链接

**步骤**

1. 打开应用菜单。
2. 点击用户手册。
3. 点击版本记录。

**预期结果**

- `./user-manual/index.html` 和 `./versions.html` 仍按站内页面打开。
- 不会把相对路径交给系统浏览器导致文件找不到。

---

## 自动化与命令检查

### 旧仓库地址扫描

```bash
rg -n "ljquan|%2Fljquan%2Faitu" \
  README.md README_en.md package.json apps/web/public packages/drawnix/src \
  --glob '!**/node_modules/**' \
  --glob '!**/sw-debug/**'
```

**预期结果**

- 不应出现旧 GitHub 仓库链接。
- `package.json` 的 author 字段如仍为历史作者名，不作为链接问题处理。

### TypeScript 调用点检查

```bash
rg -n "openExternalUrl|window.open" packages/drawnix/src/components packages/drawnix/src/utils
```

**预期结果**

- GitHub、Gist、模型文档等外链入口优先调用 `openExternalUrl`。
- 相对站内链接仍可保留现有浏览器打开方式。

---

## 已执行记录

| 命令 / 动作 | 结果 |
|------------|------|
| `rg -n "ljquan\|%2Fljquan%2Faitu" ...` | 旧仓库链接已清理 |
| `git diff --check` | 待执行 |
| GitHub 入口人工点击 | 待人工复验 |
| Gist 外链人工点击 | 待人工复验 |
| 相对站内链接人工点击 | 待人工复验 |

---

## 风险与缺口

- 同步设置中的 Gist 链接需要真实配置后人工复验。
- 桌面端外链依赖 Tauri opener 权限，后续调整 capabilities 时要保留 opener 权限。
- 静态 HTML 链接不受类型系统约束，后续仓库迁移仍需全文扫描。

---

## 回归清单

- GitHub 菜单入口打开当前仓库。
- README 和官网按钮不再指向旧仓库。
- 用户手册和版本页仍能正常打开。
- Gist 链接在桌面端打开系统浏览器。
- 模型文档链接在 Web 和桌面端均可打开。
