# GitHub 链接与桌面安装包测试文档

> 生成时间：2026-06-15
> 测试范围：GitHub 外链迁移、Tauri 桌面端外链打开、桌面安装包构建
> 目标平台：macOS Apple Silicon

---

## 背景与范围

本轮测试覆盖以下改动：

- 应用内 GitHub 入口迁移到 `AiW520/opentu`。
- 桌面端外链统一优先走 Tauri opener。
- README、官网静态页、用户手册、版本页和更新器 endpoint 清理旧 GitHub 地址。
- Tauri `32x32.png` 图标修正为 32x32 RGBA。
- 构建桌面 `.app`、`.zip` 和简版 `.dmg` 供本机实际安装测试。

不覆盖：

- GitHub Token 权限有效性。
- Gist 服务端数据一致性。
- macOS 签名、公证和自动更新发布流程。

---

## 测试环境

| 项目 | 值 |
|------|----|
| 日期 | 2026-06-15 |
| 系统 | macOS |
| 架构 | aarch64 |
| Node | 本机 Homebrew Node 26 |
| 构建命令 | `NODE_OPTIONS='--max-old-space-size=8192' pnpm nx run desktop:build` |
| 产物 | `Opentu_0.1.0_aarch64.dmg`、`Opentu_0.1.0_aarch64.app.zip` |

---

## 覆盖矩阵

| 模块 | 测试点 | 状态 |
|------|--------|------|
| 工具栏 GitHub | 点击后打开当前仓库 | 待人工复验 |
| 嵌入态工具栏 GitHub | 点击后打开当前仓库 | 待人工复验 |
| 同步设置 Gist 链接 | 桌面端使用系统浏览器打开 | 待人工复验 |
| 回收站 Gist 文件链接 | 桌面端使用系统浏览器打开 | 待人工复验 |
| README / 官网静态页 | 旧仓库地址清理 | 已静态检查 |
| Tauri 更新器 | endpoint 指向当前仓库 | 已静态检查 |
| Tauri 图标 | `32x32.png` 满足 32x32 RGBA | 已命令检查 |
| 桌面安装包 | `.app` 构建成功 | 已通过 |
| 桌面安装包 | 简版 `.dmg` 生成成功 | 已通过 |

---

## 手动测试用例

### TC-01：工具栏 GitHub 入口

**前置条件**

- 启动桌面端或安装包。
- 系统默认浏览器可用。

**步骤**

1. 打开应用主界面。
2. 点击工具栏或菜单中的 GitHub 入口。
3. 观察系统浏览器打开地址。

**预期结果**

- 打开 `https://github.com/AiW520/opentu`。
- 桌面端没有弹出空白 WebView。
- 控制台没有外链协议错误。

### TC-02：同步设置中的 Gist 外链

**前置条件**

- 已配置 GitHub Gist 同步，或有可展示的 Gist 链接。

**步骤**

1. 打开同步设置。
2. 点击“在 GitHub 查看”。
3. 观察系统浏览器打开地址。

**预期结果**

- Gist 链接在系统浏览器打开。
- 应用窗口不被错误导航到 GitHub。

### TC-03：站内相对链接

**步骤**

1. 打开应用菜单。
2. 点击用户手册。
3. 点击版本记录。

**预期结果**

- `./user-manual/index.html` 和 `./versions.html` 仍按站内页面打开。
- 不会把相对路径交给系统浏览器导致文件找不到。

### TC-04：安装包启动

**步骤**

1. 打开 `~/Downloads/Opentu_0.1.0_aarch64.dmg`。
2. 将 `Opentu.app` 拖入 Applications，或直接从 DMG 中打开。
3. 如果 macOS 提示无法验证开发者，右键选择“打开”。
4. 进入应用后执行 TC-01。

**预期结果**

- 应用可以启动。
- GitHub 外链能打开当前仓库。

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

### 图标格式检查

```bash
sips -g pixelWidth -g pixelHeight -g hasAlpha apps/desktop/src-tauri/icons/32x32.png
```

**预期结果**

```text
pixelWidth: 32
pixelHeight: 32
hasAlpha: yes
```

### 桌面构建

```bash
NODE_OPTIONS='--max-old-space-size=8192' pnpm nx run desktop:build
```

**预期结果**

- Vite production bundle 完成。
- Rust release 编译完成。
- `target/release/bundle/macos/Opentu.app` 生成。

### 测试包生成

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

---

## 已执行记录

| 命令 / 动作 | 结果 |
|------------|------|
| `pnpm nx run desktop:dev` | 首次因图标非 RGBA 失败，修正后通过 |
| `sips -g pixelWidth -g pixelHeight -g hasAlpha .../32x32.png` | 32x32 且 hasAlpha=yes |
| `pnpm nx run desktop:build` | 默认 Node heap OOM |
| `NODE_OPTIONS='--max-old-space-size=8192' pnpm nx run desktop:build` | 前端和 Rust release 构建通过，Tauri 美化 DMG 脚本失败 |
| `ditto -c -k --keepParent ...` | 生成 app zip |
| `hdiutil create ...` | 生成简版 dmg |

---

## 风险与缺口

- 当前本地测试包未签名、未公证，macOS 首次打开可能需要右键“打开”。
- Tauri 自带美化 DMG 脚本仍需单独排查；当前简版 DMG 可满足本机安装测试。
- 同步设置中的 Gist 链接需要真实配置后人工复验。
- 桌面端外链依赖 `tauri-plugin-opener` 权限，后续调整 capabilities 时要保留 `opener:default`。

---

## 回归清单

- GitHub 菜单入口打开当前仓库。
- README 和官网按钮不再指向旧仓库。
- 更新器 endpoint 指向当前仓库 release。
- 用户手册和版本页仍能正常打开。
- 桌面端安装包能启动。
- macOS 阻止未签名应用时，右键打开可以进入应用。
