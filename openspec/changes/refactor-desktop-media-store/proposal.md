# Change: 重构桌面端媒体存储架构

## Why

当前桌面端媒体存储存在以下问题：

1. **存储分散**: 混用了 Cache API、IndexedDB、文件系统、base64 回读、JS Blob，职责不清
2. **性能问题**: 大文件（如视频、高分辨率图片）通过 base64 传入 JS 内存，导致卡顿
3. **缺乏内容去重**: 相同内容的图片可能被重复存储
4. **缺少清理策略**: 无容量限制、无 LRU 清理机制，存储会无限增长
5. **缩略图缺失**: 每次都加载原图，内存和加载时间浪费

参考成熟桌面客户端（微信等）的架构：数据库管元数据/索引，文件系统管真实媒体内容，用 hash 做内容寻址。

## What Changes

### 核心架构变更

1. **SQLite 只存元数据**
   - 新增 `media_assets` 表，字段：`asset_id`, `content_hash`, `mime_type`, `size`, `width`, `height`, `duration`, `local_path`, `thumbnail_path`, `last_accessed_at`, `ref_count`, `source`, `status`
   - 保留 `assets` 表用于现有功能
   - 通过 `content_hash` 做内容去重

2. **文件系统存储真实媒体**
   - 目录结构: `media/blobs/ab/abcdef...png`（按 hash 前2字符分目录）
   - 缩略图: `media/thumbs/ab/abcdef.small.webp`, `media/thumbs/ab/abcdef.large.webp`
   - 同内容只存一份（内容寻址）

3. **全部读写走 Rust 流式处理**
   - 导入时边读边 hash、边复制
   - 视频/大图不进 JS 内存
   - 展示用 `opentu-asset` 协议 Range/stream
   - 前端只拿 URL，不拿 base64

4. **缓存清理策略**
   - 配置最大缓存容量（默认 10GB，可用户自定义）
   - LRU 清理：优先删未引用、旧缩略图、临时文件
   - 保留"已收藏/已插入画布/项目引用"的素材
   - 启动时做轻量一致性检查

5. **跨平台兼容**
   - Windows: `%APPDATA%/Opentu/media/`
   - macOS: `~/Library/Application Support/Opentu/media/`
   - Linux: `~/.local/share/opentu/media/`
   - 协议 URL: `getAssetRuntimeUrl(asset)`

### 涉及文件变更

**Rust 后端 (`apps/desktop/src-tauri/`)**
- `database.rs`: 新增 `media_assets` 表，扩展 `assets` 表
- `commands/media.rs`: 重构 `import_local_asset`, `download_url_to_media_file` 等函数
- 新增 `commands/media_store.rs`: 内容寻址存储、hash 计算、清理策略
- 新增 `commands/thumbnail.rs`: 缩略图生成

**前端 (`packages/drawnix/`)**
- `services/unified-cache-service.ts`: 桌面端重定向到 Rust API
- `hooks/useMediaCache.ts`: 适配新存储 API
- 移除 `saveToTauriFileSystem` 的临时实现

### 数据迁移

- 启动时检测旧格式媒体
- 逐步迁移到新 content-addressed 存储
- 迁移失败不阻塞启动

## Impact

- **受影响的能力**: 媒体缓存、素材库、画布图片展示、导入/导出
- **受影响的代码**: Rust 后端媒体处理、前端缓存服务
- **兼容性**: Web 版保持原有 Cache API + IndexedDB 架构不变
- **Breaking Changes**: 
  - 旧版 SQLite `assets` 表结构有变更
  - 媒体文件路径格式变化

### 风险缓解

- 渐进式迁移：先写新文件，兼容读取旧文件
- 新旧双轨：同时维护新旧存储路径
- 回滚方案：保留旧文件直到确认新架构稳定

### 非目标

- 不改动 Web 端的 Service Worker / Cache API / IndexedDB 架构
- 不改动现有的 `workspaces` / `tasks` / `settings` 表结构