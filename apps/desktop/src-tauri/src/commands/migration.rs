//! 旧格式媒体迁移到 Content-Addressed Storage (CAS)
//!
//! 扫描旧的 `图片/content-*`, `视频/content-*` 等目录，
//! 将文件迁移到 `media/blobs/{prefix}/{hash}.{ext}`，
//! 并在 SQLite `media_assets` 表中创建元数据记录。

use crate::cas::ContentAddressedStore;
use crate::database::{Database, MediaAssetRecord};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::State;

const LEGACY_CONTENT_PREFIX: &str = "content-";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgress {
    pub total_files: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub current_file: Option<String>,
    pub freed_bytes: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LegacyMediaFile {
    pub source_path: PathBuf,
    pub relative_path: String,
    pub file_type: String,
    pub size: u64,
}

pub struct MediaMigration<'a> {
    db: &'a Database,
    cas: ContentAddressedStore,
    cancelled: Arc<AtomicBool>,
}

impl<'a> MediaMigration<'a> {
    pub fn new(db: &'a Database) -> Self {
        let cas = ContentAddressedStore::new(db.media_root.clone());
        Self {
            db,
            cas,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// 扫描旧格式媒体文件
    pub fn scan_legacy_files(&self) -> Vec<LegacyMediaFile> {
        let mut files = Vec::new();
        let type_dirs = ["图片", "视频", "音频", "PPT", "文本", "压缩包"];

        for type_dir in &type_dirs {
            let dir = self.db.media_root.join(type_dir);
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    // 只处理旧格式 content-* 文件
                    if !name.starts_with(LEGACY_CONTENT_PREFIX) {
                        continue;
                    }

                    if let Ok(meta) = fs::metadata(&path) {
                        files.push(LegacyMediaFile {
                            source_path: path,
                            relative_path: format!("{}/{}", type_dir, name),
                            file_type: type_dir.to_string(),
                            size: meta.len(),
                        });
                    }
                }
            }
        }

        files
    }

    /// 迁移单个文件
    pub fn migrate_single(
        &self,
        legacy: &LegacyMediaFile,
    ) -> Result<Option<MediaAssetRecord>, String> {
        if self.is_cancelled() {
            return Err("Migration cancelled".to_string());
        }

        let source = &legacy.source_path;

        // 计算 content hash
        let content_hash = match self.compute_hash(source) {
            Ok(h) => h,
            Err(e) => return Err(format!("Hash failed: {}", e)),
        };

        // 确定文件扩展名
        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| {
                extension_from_file_type(&legacy.file_type)
            });

        // 检查是否已存在于 CAS
        let target_path = self.cas.content_path(&content_hash, &extension);
        let already_migrated = target_path.exists();

        if !already_migrated {
            // 确保目标目录存在
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Create dir failed: {}", e))?;
            }

            // 复制文件到 CAS 位置
            fs::copy(source, &target_path)
                .map_err(|e| format!("Copy failed: {}", e))?;
        }

        // 确定 MIME 类型
        let mime_type = mime_type_from_extension(&extension);

        // 获取图片尺寸（如果是图片）
        let (width, height) = if is_image_extension(&extension) {
            match image::open(source) {
                Ok(img) => {
                    let (w, h) = img.dimensions();
                    (Some(w as i64), Some(h as i64))
                }
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        let now = chrono::Utc::now().timestamp();

        // 构建 asset_id（旧路径作为唯一标识）
        let asset_id = format!("legacy-{}", legacy.relative_path.replace(['/', '\\'], "-"));

        let record = MediaAssetRecord {
            asset_id,
            content_hash: content_hash.clone(),
            mime_type: Some(mime_type),
            size: legacy.size as i64,
            width,
            height,
            duration: None,
            local_path: target_path.to_string_lossy().to_string(),
            thumbnail_path: None,
            last_accessed_at: now,
            ref_count: 0,
            source: Some(legacy.relative_path.clone()),
            status: if already_migrated { "migrated" } else { "active" }.to_string(),
        };

        // 写入数据库
        self.db
            .upsert_media_asset(&record)
            .map_err(|e| format!("DB write failed: {}", e))?;

        // 如果是新迁移，删除旧文件
        if !already_migrated {
            if let Err(e) = fs::remove_file(source) {
                eprintln!("[Migration] Warning: failed to remove old file {}: {}", source.display(), e);
            }
        }

        Ok(Some(record))
    }

    /// 批量迁移
    pub fn migrate_all(
        &self,
        progress_callback: impl Fn(MigrationProgress),
    ) -> MigrationProgress {
        let legacy_files = self.scan_legacy_files();
        let total = legacy_files.len();

        let mut progress = MigrationProgress {
            total_files: total,
            processed: 0,
            succeeded: 0,
            skipped: 0,
            failed: 0,
            current_file: None,
            freed_bytes: 0,
            errors: Vec::new(),
        };

        for legacy in &legacy_files {
            if self.is_cancelled() {
                break;
            }

            progress.current_file = Some(legacy.relative_path.clone());
            progress.processed += 1;
            progress_callback(progress.clone());

            match self.migrate_single(legacy) {
                Ok(Some(record)) => {
                    progress.succeeded += 1;
                    if record.status == "active" {
                        progress.freed_bytes += legacy.size;
                    } else {
                        progress.skipped += 1;
                    }
                }
                Ok(None) => {
                    progress.skipped += 1;
                }
                Err(e) => {
                    progress.failed += 1;
                    if progress.errors.len() < 10 {
                        progress.errors.push(format!(
                            "{}: {}",
                            legacy.relative_path, e
                        ));
                    }
                }
            }

            progress.current_file = None;
        }

        progress
    }

    fn compute_hash(&self, path: &Path) -> Result<String, String> {
        let file = File::open(path)
            .map_err(|e| format!("Open failed: {}", e))?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 128 * 1024];

        loop {
            let bytes_read = reader.read(&mut buffer)
                .map_err(|e| format!("Read failed: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(bytes_to_hex(&result))
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{:02x}", byte));
    }
    output
}

fn extension_from_file_type(file_type: &str) -> String {
    match file_type {
        "图片" | "IMAGE" => "png".to_string(),
        "视频" | "VIDEO" => "mp4".to_string(),
        "音频" | "AUDIO" => "mp3".to_string(),
        "PPT" => "pptx".to_string(),
        "文本" | "TEXT" => "txt".to_string(),
        "压缩包" | "ARCHIVE" => "zip".to_string(),
        _ => "bin".to_string(),
    }
}

fn mime_type_from_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "pdf" => "application/pdf",
        "doc" | "docx" => "application/msword",
        "ppt" | "pptx" => "application/vnd.ms-powerpoint",
        "xls" | "xlsx" => "application/vnd.ms-excel",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif"
    )
}

#[tauri::command]
pub fn run_media_migration(
    state: State<'_, crate::AppState>,
) -> Result<MigrationProgress, String> {
    let db = {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard.clone_db()
    };

    let migration = MediaMigration::new(&db);

    let result = migration.migrate_all(|progress| {
        eprintln!(
            "[Migration] {}/{} - {} succeeded, {} failed, {} skipped",
            progress.processed,
            progress.total_files,
            progress.succeeded,
            progress.failed,
            progress.skipped
        );
    });

    Ok(result)
}

#[tauri::command]
pub fn scan_legacy_media_files(
    state: State<'_, crate::AppState>,
) -> Result<Vec<LegacyMediaFile>, String> {
    let db = {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard.clone_db()
    };

    let migration = MediaMigration::new(&db);
    Ok(migration.scan_legacy_files())
}
