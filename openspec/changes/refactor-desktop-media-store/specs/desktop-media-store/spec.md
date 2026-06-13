## ADDED Requirements

### Requirement: 桌面端必须使用 Content-Addressed Storage (CAS) 存储媒体
桌面端 SHALL 将媒体文件存储在文件系统中，使用内容 SHA256 hash 作为唯一标识符，按 hash 前两个字符分目录。

#### Scenario: 导入重复图片
- **GIVEN** 用户导入一张图片 content-hash 为 `abcdef123456...`
- **WHEN** 系统计算 hash 并检查存储
- **THEN** 文件应存储为 `media/blobs/ab/abcdef123456....{ext}`
- **AND** 如果 hash 相同的文件已存在，应复用而非重新存储

### Requirement: 桌面端必须使用 SQLite 管理媒体元数据
桌面端 SHALL 在 SQLite 中使用独立的 `media_assets` 表管理媒体元数据，包含 content_hash、mime_type、size、width、height、duration、local_path、thumbnail_path、last_accessed_at、ref_count、source、status 字段。

#### Scenario: 查询素材信息
- **GIVEN** 前端需要显示素材库列表
- **WHEN** 调用 Rust 命令获取元数据
- **THEN** 应返回完整元数据包括缩略图路径和引用计数

### Requirement: 桌面端必须实现缩略图系统
桌面端 SHALL 在导入媒体时异步生成缩略图，存储为 WebP 格式，路径格式为 `media/thumbs/{hash[0..1]}/{hash}.{size}.webp`，支持 small (200px) 和 large (800px) 两种尺寸。

#### Scenario: 生成缩略图
- **GIVEN** 用户导入一张 4K 分辨率图片
- **WHEN** 图片被存储到 content-addressed 路径
- **THEN** 系统应异步生成 small (200px) 和 large (800px) 两个 WebP 缩略图
- **AND** 缩略图路径应记录到数据库

### Requirement: 桌面端必须实现 LRU 缓存清理策略
桌面端 SHALL 实现基于最近访问时间的缓存清理策略，支持配置最大缓存容量（默认 10GB），优先清理 ref_count=0 且 status!=protected 的素材。

#### Scenario: 缓存达到容量上限
- **GIVEN** 缓存大小达到配置的最大容量 (10GB)
- **WHEN** 用户导入新素材
- **THEN** 系统应按 LRU 顺序清理 ref_count=0 且 status='temp' 的素材
- **AND** 直到缓存大小降到目标阈值以下

### Requirement: 桌面端必须支持内容去重
桌面端 SHALL 在导入媒体时计算 content hash，如果相同 hash 的文件已存在则复用，不重复存储。

#### Scenario: 导入相同内容不同名称
- **GIVEN** 用户先后导入 `photo.jpg` 和 `图片.png` 两个文件
- **WHEN** 两个文件内容完全相同 (hash 相同)
- **THEN** 存储层只应保存一份文件
- **AND** 两个素材记录应指向同一 content hash

### Requirement: 桌面端媒体读写必须流式处理
桌面端 SHALL 在导入、导出媒体时使用 Rust 流式处理，大文件不进 JS 内存，通过 opentu-asset 协议的 Range 请求读取。

#### Scenario: 读取大视频文件
- **GIVEN** 用户在画布上加载一个 500MB 的视频
- **WHEN** 视频通过 opentu-asset 协议加载
- **THEN** Rust 应使用 Range 请求流式传输
- **AND** 视频数据不应通过 JS base64 传递

### Requirement: 桌面端必须实现启动时一致性检查
桌面端 SHALL 在启动时检测孤儿文件（文件存在但数据库无记录）和孤儿记录（数据库有记录但文件不存在），并可选择自动清理孤儿。

#### Scenario: 检测孤儿记录
- **GIVEN** 数据库中有一条 media_assets 记录
- **WHEN** 对应的文件在文件系统中不存在
- **THEN** 系统应记录此孤儿记录
- **AND** 可选：自动从数据库删除孤儿记录

### Requirement: 桌面端必须支持跨平台路径
桌面端 SHALL 使用 Rust PathBuf 管理所有文件路径：
- Windows: %APPDATA%/Opentu/media/
- macOS: ~/Library/Application Support/Opentu/media/
- Linux: ~/.local/share/opentu/media/

#### Scenario: Windows 中文路径
- **GIVEN** 用户目录为 `C:\用户\张三\`
- **WHEN** 应用初始化媒体存储
- **THEN** 媒体根目录应为 `C:\用户\张三\AppData\Roaming\Opentu\media\`
- **AND** 所有路径操作使用 Rust PathBuf，不让前端拼原生路径