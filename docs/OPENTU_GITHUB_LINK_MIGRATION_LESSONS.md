# GitHub 链接迁移经验总结

更新日期：2026-06-15

## 背景

这轮问题的核心不是“最新版没有拉取到”，而是仓库迁移后仍有入口指向旧地址。用户从 README、官网静态页、工具栏或桌面端入口点击 GitHub 时，可能进入旧仓库，表现为“GitHub 无法链接过去”。

本次处理 GitHub 仓库地址与外链打开链路。

## 现象

- 工具栏 GitHub 按钮仍打开旧仓库。
- README、官网静态页、版本页、用户手册里残留旧仓库地址。
- 桌面端外链入口分散使用 `window.open`，不同入口在 Tauri 环境下行为不一致。
- Gist、GitHub、模型文档等外链没有统一协议校验和回退逻辑。

## 根因

### 1. 外链入口没有统一封装

多个组件直接调用 `window.open`。Web 环境一般可用，但桌面端更适合优先使用 Tauri opener 打开系统浏览器。

入口分散会带来三个问题：

- 仓库迁移时容易漏改。
- 桌面端和 Web 端打开行为不一致。
- 非预期协议缺少统一拦截。

### 2. 仓库迁移后静态链接没有同步清理

项目已迁移到 `AiW520/opentu`，但静态页面、README、用户手册、版本页和 package 元数据中仍存在历史链接。

这类链接不会被 TypeScript 编译或单元测试覆盖，必须通过全文扫描和人工点链路兜底。

### 3. 相对站内链接和外部链接混在同一处理路径

用户手册、版本页等相对路径仍应在当前页面打开；GitHub、Gist、发布页和模型文档才应交给系统浏览器。

如果不区分相对 URL 和外部 URL，桌面端可能把站内页面交给系统浏览器，导致路径解析异常。

## 最终方案

### 1. 新增统一外链工具

新增 `openExternalUrl`：

- 相对 URL 继续使用浏览器默认打开方式。
- `http`、`https`、`mailto`、`tel` 外链在 Tauri 环境优先调用 opener。
- Tauri opener 失败时回退 `window.open`。
- 拦截非白名单协议，避免不安全 URL 被直接打开。

### 2. 统一 GitHub / Gist / 文档入口

改造以下入口统一走工具函数：

- 顶部菜单 GitHub 入口
- 嵌入态工具栏 GitHub 图标
- 同步设置中的 Gist 链接
- 回收站中的 Gist 文件链接
- 模型文档链接

### 3. 静态链接统一迁移

同步替换：

- `README.md`
- `README_en.md`
- `package.json`
- `apps/web/public/home.html`
- `apps/web/public/en/home.html`
- `apps/web/public/versions.html`
- `apps/web/public/user-manual/index.html`

## 固化规则

- 外链入口不要散落 `window.open`，优先走统一工具。
- 仓库迁移时至少扫描明文 URL、URL 编码 URL 和 package 元数据。
- 桌面端外链优先使用 Tauri opener，失败后再回退 Web 打开。
- 相对站内链接不能交给系统浏览器。
- 新增 GitHub、Gist、文档、发布页等入口时，先判断是否可以复用 `openExternalUrl`。

## 后续检查清单

- `rg "ljquan|%2Fljquan%2Faitu"` 是否还有旧仓库残留？
- GitHub 菜单入口是否打开 `https://github.com/AiW520/opentu`？
- 同步设置里的 Gist 链接在桌面端是否能打开系统浏览器？
- 模型文档链接是否仍能在 Web 和桌面端打开？
- 用户手册、版本页等相对站内链接是否仍在当前应用内可访问？

## 一句话总结

GitHub 链接迁移不能只改一个按钮；要把“代码入口、静态页面、文档链接、桌面外链打开方式”作为一条链路一起校验。
