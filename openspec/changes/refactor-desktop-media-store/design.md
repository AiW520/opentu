## Context

桌面端媒体存储需要重构以解决性能、稳定性和可维护性问题。当前架构混用了多种存储方案（Cache API、IndexedDB、文件系统、base64），导致：
- 大文件进 JS 内存造成卡顿
- 相同内容重复存储
- 无清理策略导致存储无限增长
- 难以实现缩略图缓存

参考微信等成熟桌面客户端的架构：数据库管元数据/索引，文件系统管真实内容，用 hash 做内容寻址。

## Goals / Non-Goals

### Goals
- 内容寻址存储：相同内容只存一份
- 流式处理：大文件不进 JS 内存
- 缩略图系统：异步生成，降低内存和加载时间
- LRU 清理策略：可配置容量限制
- 跨平台兼容：Windows/macOS/Linux 统一架构

### Non-Goals
- 不改动 Web 端架构（保持 Cache API + IndexedDB）
- 不改动现有 `workspaces`、`tasks`、`settings` 表
- 不支持端侧 AI 模型文件存储（单一大文件场景不同）

## Decisions

### Decision 1: Content-Addressed Storage (CAS)

**选择**: 按 SHA256 hash 前2字符分目录存储
**路径格式**: `media/blobs/{hash[0..1]}/{hash}.{ext}`

**理由**:
- 相同内容自动去重
- 避免单目录文件过多
- hash 可验证完整性
- 微信等客户端采用类似方案

**替代方案考虑**:
- 平面存储 `media/{hash}.{ext}`: 单目录文件过多，文件系统性能下降
- 按时间分目录: 无法去重，清理困难

### Decision 2: 缩略图格式

**选择**: WebP 格式，200px (small) / 800px (large)
**路径格式**: `media/thumbs/{hash[0..1]}/{hash}.{size}.webp`

**理由**:
- WebP 比 JPEG/PNG 体积小 25-35%
- 固定尺寸便于前端布局
- 异步生成不阻塞主流程

**替代方案考虑**:
- PNG: 体积大，不支持无损压缩率调整
- JPEG: 不支持透明度
- GIF: 动画场景不需要

### Decision 3: SQLite 元数据结构

**选择**: 新增 `media_assets` 表，与现有 `assets` 表分离

**理由**:
- 保持 `assets` 表稳定（可能被外部引用）
- `media_assets` 可独立演进
- `content_hash` 做去重索引
- `ref_count` 支持引用计数清理保护

**Schema**:
```sql
CREATE TABLE media_assets (
    asset_id TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    mime_type TEXT,
    size INTEGER NOT NULL DEFAULT 0,
    width INTEGER,
    height INTEGER,
    duration REAL,
    local_path TEXT NOT NULL,
    thumbnail_path TEXT,
    last_accessed_at INTEGER NOT NULL,
    ref_count INTEGER NOT NULL DEFAULT 0,
    source TEXT,
    status TEXT NOT NULL DEFAULT 'active'
);
CREATE INDEX idx_media_content_hash ON media_assets(content_hash);
CREATE INDEX idx_media_last_accessed ON media_assets(last_accessed_at);
CREATE INDEX idx_media_status ON media_assets(status);
```

### Decision 4: 清理策略

**选择**: LRU + ref_count 保护 + 手动/自动混合模式

**规则**:
1. `ref_count > 0` 的素材永不清理（已引用/已收藏/已插入画布）
2. 优先清理 `status = 'temp'` 的临时文件
3. 优先清理旧缩略图（可按需重新生成）
4. 按 `last_accessed_at` 升序清理，直到 `current_size <= target_size`

**容量默认值**: 10GB，用户可配置

### Decision 5: Rust 缩略图生成

**选择**: 使用 `image` crate 流式处理
**依赖**: `image = "0.25"` (bundled)

**理由**:
- 已有 `rusqlite` 和 `sha2` 依赖
- `image` crate 成熟稳定，支持 WebP
- 流式处理避免大图进内存

**备选**: 调用系统工具（`ffmpeg`, `convert`）: 引入外部依赖，跨平台复杂

### Decision 6: 协议 URL 格式

**选择**: 保持现有 `opentu-asset://` 协议，扩展支持缩略图

**URL 格式**:
- 原图: `opentu-asset://localhost/{encoded_path}`
- 小缩略图: `opentu-asset://localhost/thumb/small/{content_hash}`
- 大缩略图: `opentu-asset://localhost/thumb/large/{content_hash}`

**理由**:
- 现有协议已支持 Windows/macOS/Linux
- 扩展缩略图只需解析路径前缀

## Risks / Trade-offs

### Risk 1: 数据迁移复杂

**风险**: 从旧格式迁移可能丢失数据或阻塞启动

**缓解**:
- 迁移为异步操作，不阻塞启动
- 迁移前备份旧文件
- 迁移失败记录日志，不中断应用
- 新旧双轨：同时支持读写

### Risk 2: 缩略图生成性能

**风险**: 首次导入大量图片时缩略图生成可能卡顿

**缓解**:
- 使用 `request_idle_callback` 空闲时生成
- 只对超过 4MB 的图片生成缩略图
- 缩略图失败不影响原图使用

### Risk 3: 前端改造面大

**风险**: `unifiedCacheService` 深度耦合 Web 架构

**缓解**:
- 桌面端条件分支，Web 端保持不变
- Rust API 提供完整功能，前端逐步迁移
- 保留降级路径（Cache API fallback）

## Migration Plan

### Phase 1: 基础设施（不破坏现有功能）
1. 扩展数据库 schema（添加新表、新字段）
2. 实现 content-addressed 存储函数
3. 实现缩略图生成
4. 实现清理策略
5. **不修改任何现有函数调用**

### Phase 2: 新存储路径
1. 修改 `import_local_asset` 使用新存储
2. 修改 `download_url_to_media_file` 使用新存储
3. 添加 Rust API 命令
4. 前端条件分支调用新 API

### Phase 3: 迁移与清理
1. 实现启动时迁移检测
2. 实现批量迁移函数
3. 实现一致性检查
4. 添加容量设置 UI

### Rollback

- 如果 Phase 2/3 发现问题，可回滚到 Phase 1
- 旧文件不被删除，只是不再使用
- 通过 Feature Flag 控制启用/禁用

## Open Questions

1. **清理触发的时机**: 仅启动时检查，还是也定时检查？建议仅启动时。
2. **缩略图尺寸**: 200px/800px 是否满足所有场景？建议先实现，后续按需调整。
3. **是否支持视频缩略图**: 视频也需要缩略图用于素材库展示，建议后续实现。
4. **缓存设置 UI**: 是否需要单独的设置页面？建议先在现有设置中加一行。