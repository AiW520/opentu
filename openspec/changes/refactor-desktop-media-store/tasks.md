## 1. Rust 后端实现

### 1.1 数据库扩展
- [ ] 1.1.1 在 `database.rs` 中新增 `media_assets` 表
  - 字段: `asset_id` (TEXT PRIMARY KEY), `content_hash` (TEXT), `mime_type` (TEXT), `size` (INTEGER), `width` (INTEGER), `height` (INTEGER), `duration` (REAL), `local_path` (TEXT), `thumbnail_path` (TEXT), `last_accessed_at` (INTEGER), `ref_count` (INTEGER), `source` (TEXT), `status` (TEXT)
  - 索引: `idx_content_hash`, `idx_last_accessed`, `idx_status`
- [ ] 1.1.2 扩展 `assets` 表，添加 `content_hash`, `thumbnail_path`, `ref_count` 字段
- [ ] 1.1.3 创建数据库迁移函数 `migrate_media_assets_schema()`

### 1.2 内容寻址存储
- [ ] 1.2.1 实现 `get_content_addressed_path(hash: &str, ext: &str) -> PathBuf`
  - 按 hash 前2字符分目录: `media/blobs/ab/abcdef...ext`
- [ ] 1.2.2 实现 `compute_content_hash(reader: impl Read) -> String`
  - 使用 SHA256，流式计算
- [ ] 1.2.3 实现 `store_content_addressed_file(source: &Path, content_hash: &str) -> Result<PathBuf>`
  - 如果目标已存在则跳过复制
  - 否则流式复制到目标路径
- [ ] 1.2.4 实现 `get_content_addressed_file(content_hash: &str) -> Option<PathBuf>`

### 1.3 缩略图生成
- [ ] 1.3.1 添加 `image` crate 依赖到 `Cargo.toml`
- [ ] 1.3.2 实现 `generate_thumbnail(source: &Path, content_hash: &str, size: ThumbnailSize) -> Result<PathBuf>`
  - 支持 `small` (200px) 和 `large` (800px)
  - 输出 WebP 格式
  - 路径: `media/thumbs/ab/abcdef.small.webp`
- [ ] 1.3.3 实现 `get_thumbnail_path(content_hash: &str, size: ThumbnailSize) -> Option<PathBuf>`
- [ ] 1.3.4 实现 `delete_thumbnails(content_hash: &str) -> Result<()>`

### 1.4 媒体导入重构
- [ ] 1.4.1 重构 `import_local_asset()` 函数
  - 使用 content-addressed 存储
  - 流式 hash 计算
  - 生成缩略图
  - 更新 SQLite 元数据
- [ ] 1.4.2 重构 `download_url_to_media_file()` 函数
  - 边下载边 hash（不解压到内存）
  - 流式写入临时文件，完成后 rename
  - 内容去重检查
- [ ] 1.4.3 实现 `import_media_from_blob(hash: &str, blob: Vec<u8>, mime_type: &str) -> Result<AssetImportResult>`

### 1.5 缓存清理策略
- [ ] 1.5.1 在 `database.rs` 中添加 `cache_settings` 表
  - 字段: `key` (TEXT PRIMARY KEY), `value` (TEXT), `updated_at` (INTEGER)
- [ ] 1.5.2 实现 `get_max_cache_size() -> u64` (默认 10GB)
- [ ] 1.5.3 实现 `set_max_cache_size(size: u64) -> Result<()>`
- [ ] 1.5.4 实现 `calculate_cache_size() -> u64`
- [ ] 1.5.5 实现 `select_lru_candidates(limit: usize) -> Vec<MediaAsset>`
  - 查询条件: `ref_count = 0 AND status != 'protected' ORDER BY last_accessed_at`
- [ ] 1.5.6 实现 `cleanup_cache(target_size: u64) -> Result<CleanupResult>`
  - 计算需要释放的空间
  - 选择 LRU 候选
  - 删除文件和缩略图
  - 更新数据库
- [ ] 1.5.7 实现 `run_startup_cache_check() -> Result<()>`
  - 检测孤儿文件（文件存在但数据库无记录）
  - 检测孤儿记录（数据库有记录但文件不存在）
  - 可选：自动清理孤儿

### 1.6 协议层增强
- [ ] 1.6.1 扩展 `handle_opentu_asset_protocol()` 支持缩略图 URL
  - `opentu-asset://localhost/thumb/small/{content_hash}`
  - `opentu-asset://localhost/thumb/large/{content_hash}`
- [ ] 1.6.2 添加 `get_asset_url(asset_id: &str) -> Result<String>` 命令
- [ ] 1.6.3 添加 `get_asset_thumbnail_url(asset_id: &str, size: &str) -> Result<String>` 命令
- [ ] 1.6.4 添加 `update_asset_ref_count(asset_id: &str, delta: i32) -> Result<i32>` 命令
- [ ] 1.6.5 添加 `get_cache_stats() -> Result<CacheStats>` 命令
- [ ] 1.6.6 添加 `run_cache_cleanup() -> Result<CleanupResult>` 命令

### 1.7 数据迁移
- [ ] 1.7.1 实现 `detect_legacy_media_assets() -> Vec<LegacyAsset>`
- [ ] 1.7.2 实现 `migrate_single_asset(asset: LegacyAsset) -> Result<AssetImportResult>`
- [ ] 1.7.3 实现 `migrate_all_legacy_assets() -> MigrationResult`
  - 批量迁移
  - 失败记录日志，不阻塞
- [ ] 1.7.4 在 `Database::new()` 中调用迁移检测

## 2. 前端适配

### 2.1 缓存服务重构
- [ ] 2.1.1 修改 `unifiedCacheService` 桌面端重定向逻辑
  - 优先调用 Rust Tauri 命令
  - 降级到原有 Cache API（仅当 Rust 命令失败时）
- [ ] 2.1.2 实现 `saveToTauriContentAddressed()` 替代 `saveToTauriFileSystem()`
  - 使用 content hash 路径
  - 调用 `import_media_from_blob` 命令
- [ ] 2.1.3 实现 `getTauriAssetUrl(assetId)` 调用 Rust 命令

### 2.2 素材库集成
- [ ] 2.2.1 修改 `useMediaCache` 使用新 Rust API 获取缩略图 URL
- [ ] 2.2.2 修改素材库列表分页使用 Rust 端元数据查询

### 2.3 一致性检查
- [ ] 2.3.1 在应用启动时调用 `run_startup_cache_check()`
- [ ] 2.3.2 展示迁移进度对话框（如有旧数据）

## 3. 测试与验证

### 3.1 单元测试
- [ ] 3.1.1 测试 content hash 计算正确性
- [ ] 3.1.2 测试内容去重（相同文件不重复存储）
- [ ] 3.1.3 测试 LRU 清理逻辑
- [ ] 3.1.4 测试缩略图生成质量

### 3.2 集成测试
- [ ] 3.2.1 测试导入流程（本地文件、URL、剪贴板）
- [ ] 3.2.2 测试缩略图加载性能
- [ ] 3.2.3 测试容量达到上限时的清理行为
- [ ] 3.2.4 测试数据迁移完整性

### 3.3 跨平台测试
- [ ] 3.3.1 Windows: 测试中文路径、空格路径
- [ ] 3.3.2 macOS: 测试 sandbox 路径限制
- [ ] 3.3.3 Linux: 测试 XDG 路径规范

### 3.4 兼容性测试
- [ ] 3.4.1 测试旧版存储格式仍可读取
- [ ] 3.4.2 测试 Web 版不受影响