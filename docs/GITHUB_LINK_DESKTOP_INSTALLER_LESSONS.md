# GitHub 链接与桌面安装包经验总结

更新日期：2026-06-15

## 背景

这轮问题集中在两个链路：

- 应用内 GitHub 入口仍指向旧仓库，用户点击后无法到达当前项目。
- 桌面端需要构建本地安装包进行真实安装测试，但 Tauri 打包前置资源和生产构建内存预算不满足。

这类问题很容易被误判成“最新版没有下载成功”。实际排查时要同时检查：

- 前端可点击入口
- 静态官网和 README 链接
- 桌面端外链打开方式
- Tauri 图标资源格式
- 安装包构建命令的 Node 内存预算

## 现象

- 工具栏 GitHub 按钮仍打开 `ljquan/aitu`。
- README、官网静态页、版本页、用户手册里仍残留旧仓库地址。
- Tauri dev 首次编译失败，提示 `icons/32x32.png is not RGBA`。
- `pnpm nx run desktop:build` 在 Vite production bundle 阶段触发 Node heap OOM。
- Tauri 自带的美化 DMG 脚本在最后一步失败，但 `.app` 已经构建成功。

## 根因

### 1. 外链入口没有统一封装

桌面端继续使用 `window.open` 打开外部 URL。Web 环境一般可用，但在 Tauri 桌面端更稳的方式是走 `tauri-plugin-opener`。

如果每个入口都直接写 `window.open`：

- 仓库迁移时容易漏改。
- 桌面端权限和系统浏览器行为不可控。
- Gist、GitHub、发布页等外链行为不一致。

### 2. 仓库迁移后静态链接没有同步清理

项目已迁移到 `AiW520/opentu`，但 README、官网静态 HTML、用户手册和版本页仍有旧地址。
这会导致用户从不同入口进入不同仓库，看起来像“GitHub 无法链接过去”。

### 3. Tauri 图标资源必须满足严格格式

`src-tauri/icons/32x32.png` 不是只看文件名。Tauri build 会校验实际图像：

- 尺寸必须是 32x32。
- PNG 必须带 alpha，也就是 RGBA。

这轮文件实际是 1022x1022 RGB，因此 dev/build 都会在 `generate_context!()` 阶段失败。

### 4. 桌面生产构建需要更高 Node 堆内存

当前 renderer bundle 模块很多，默认 Node heap 约 2GB 不够。
直接跑 `pnpm nx run desktop:build` 会在 Vite 打包阶段 OOM。

更稳的构建命令是：

```bash
NODE_OPTIONS='--max-old-space-size=8192' pnpm nx run desktop:build
```

## 最终方案

### 1. 新增统一外链工具

新增 `openExternalUrl`：

- 相对 URL 继续用 `window.open`，避免把站内 `./versions.html` 交给系统浏览器。
- `http`、`https`、`mailto`、`tel` 外链在 Tauri 环境优先调用 `plugin:opener|open_url`。
- Tauri opener 失败时回退 `window.open`。
- 拦截非白名单协议，避免不安全 URL 被直接打开。

### 2. GitHub / Gist / 发布页入口统一走工具函数

改造位置包括：

- 顶部菜单 GitHub 入口
- 嵌入态工具栏 GitHub 图标
- 同步设置中的 Gist 链接
- 回收站中的 Gist 文件链接
- 用户手册、版本页、发布页入口

### 3. 静态链接统一迁移

同步替换：

- `README.md`
- `README_en.md`
- `package.json`
- `apps/web/public/home.html`
- `apps/web/public/en/home.html`
- `apps/web/public/versions.html`
- `apps/web/public/user-manual/index.html`
- `apps/desktop/src-tauri/tauri.conf.json`

### 4. 修正桌面图标

将 `apps/desktop/src-tauri/icons/32x32.png` 修正为真实 `32x32 RGBA` PNG。
后续替换图标时不能只覆盖文件名，要重新检查实际尺寸和 alpha。

## 构建步骤经验

### 本地运行

```bash
pnpm nx run desktop:dev
```

若遇到 `32x32.png is not RGBA`，先检查图标：

```bash
sips -g pixelWidth -g pixelHeight -g hasAlpha apps/desktop/src-tauri/icons/32x32.png
```

### 构建安装包

```bash
NODE_OPTIONS='--max-old-space-size=8192' pnpm nx run desktop:build
```

如果 Tauri 美化 DMG 脚本失败，但 `.app` 已生成，可以先产出测试包：

```bash
ditto -c -k --keepParent \
  apps/desktop/src-tauri/target/release/bundle/macos/Opentu.app \
  "$HOME/Downloads/Opentu_0.1.0_aarch64.app.zip"

hdiutil create \
  -volname "Opentu" \
  -srcfolder apps/desktop/src-tauri/target/release/bundle/macos/Opentu.app \
  -ov \
  -format UDZO \
  "$HOME/Downloads/Opentu_0.1.0_aarch64.dmg"
```

## 固化规则

- 外链入口不要散落 `window.open`，优先走统一工具。
- 仓库迁移时至少扫三类地址：明文 URL、URL 编码 URL、更新器 endpoint。
- 桌面端外链要优先使用 Tauri opener，Web 再走浏览器打开。
- 相对站内链接不能直接交给系统浏览器。
- Tauri 图标资源要检查实际尺寸和 alpha，而不是只看文件名。
- 桌面生产构建默认带 8GB Node heap，避免 Vite 打包阶段 OOM。
- 美化 DMG 失败不等于 `.app` 不可用，先交付 `.app.zip` 或简版 DMG 做真实测试。

## 后续检查清单

- `rg "ljquan|%2Fljquan%2Faitu"` 是否还有旧仓库残留？
- GitHub 菜单入口是否打开 `https://github.com/AiW520/opentu`？
- 同步设置里的 Gist 链接在桌面端是否能打开系统浏览器？
- `32x32.png` 是否为 `32x32` 且 `hasAlpha: yes`？
- build 是否使用 `NODE_OPTIONS='--max-old-space-size=8192'`？
- 安装包产物是否能从 `~/Downloads` 打开测试？

## 一句话总结

GitHub 链接问题不要只改一个按钮；要把“仓库地址、桌面外链打开方式、更新器 endpoint、安装包构建前置资源”当成一条完整链路一起校验。
