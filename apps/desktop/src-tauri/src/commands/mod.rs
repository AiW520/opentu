pub mod export;
pub mod file_manager;
pub mod media;
pub mod migration;
pub mod storage;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cas::ContentAddressedStore;
use crate::database::MediaAssetRecord;
use crate::path_grants::canonical_existing_file;
use crate::thumbnail::{read_image_info, ThumbnailGenerator, ThumbnailSize};
use crate::AppState;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAssetResponse {
    pub asset_id: String,
    pub content_hash: String,
    pub mime_type: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub local_path: String,
    pub thumbnail_small_path: Option<String>,
    pub thumbnail_large_path: Option<String>,
    pub last_accessed_at: i64,
    pub ref_count: i32,
    pub source: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCacheStatsResponse {
    pub total_size_bytes: u64,
    pub total_size_human: String,
    pub blob_size_bytes: u64,
    pub thumbnail_size_bytes: u64,
    pub asset_count: u64,
    pub max_cache_size_bytes: u64,
    pub max_cache_size_human: String,
    pub percent_used: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCleanupResponse {
    pub removed_count: u64,
    pub freed_bytes: u64,
    pub orphan_files_removed: u64,
    pub orphan_records_removed: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMediaParams {
    pub source_path: String,
    pub file_type: Option<String>,
    pub original_name: Option<String>,
    pub mime_type: Option<String>,
    pub source: Option<String>,
    pub generate_thumbnails: Option<bool>,
}

#[tauri::command]
pub fn import_local_media_asset(
    state: State<'_, AppState>,
    params: ImportMediaParams,
) -> Result<MediaAssetResponse, String> {

    let source_path = PathBuf::from(&params.source_path);
    if !source_path.exists() || !source_path.is_file() {
        return Err("源文件不存在或不是文件".to_string());
    }

    let source_path = canonical_existing_file(&source_path).map_err(|e| e.to_string())?;

    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };

    let metadata = fs::metadata(&source_path).map_err(|e| e.to_string())?;
    let file_size = metadata.len();

    let content_hash = {
        let mut file = File::open(&source_path).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 128 * 1024];
        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in &result[..] {
            use std::fmt::Write;
            let _ = write!(hex, "{:02x}", byte);
        }
        hex
    };

    let detected_mime = params
        .mime_type
        .clone()
        .unwrap_or_else(|| detect_mime_type(&source_path));

    let extension = source_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| extension_from_mime(&detected_mime));

    let cas = ContentAddressedStore::new(media_root.clone());
    let target_path = cas.content_path(&content_hash, &extension);

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let reused = target_path.exists();
    if !reused {
        let tmp_path = target_path.with_extension(format!(
            "{}.tmp",
            extension
        ));
        let mut source = File::open(&source_path).map_err(|e| e.to_string())?;
        let mut tmp = File::create(&tmp_path).map_err(|e| e.to_string())?;
        let mut buffer = [0u8; 128 * 1024];
        loop {
            let bytes_read = source.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }
            tmp.write_all(&buffer[..bytes_read]).map_err(|e| e.to_string())?;
        }
        tmp.flush().map_err(|e| e.to_string())?;
        drop(tmp);
        drop(source);

        fs::rename(&tmp_path, &target_path).map_err(|e| e.to_string())?;
    }

    let (width, height) = if is_image_file(&source_path) {
        match read_image_info(&target_path) {
            Ok(info) => (Some(info.width), Some(info.height)),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let (thumb_small, thumb_large) = if params
        .generate_thumbnails
        .unwrap_or(true)
        && is_image_file(&target_path)
    {
        let gen = ThumbnailGenerator::new(&cas);
        let mut small_path = None;
        let mut large_path = None;

        if let Ok(small) = gen.generate(&target_path, &content_hash, ThumbnailSize::Small)
        {
            small_path = Some(small.local_path.to_string_lossy().to_string());
        }
        if let Ok(large) = gen.generate(&target_path, &content_hash, ThumbnailSize::Large)
        {
            large_path = Some(large.local_path.to_string_lossy().to_string());
        }
        (small_path, large_path)
    } else {
        (None, None)
    };

    let now = chrono::Utc::now().timestamp();
    let asset_id = format!("asset-{}", content_hash.clone());

    let record = MediaAssetRecord {
        asset_id: asset_id.clone(),
        content_hash: content_hash.clone(),
        mime_type: Some(detected_mime.clone()),
        size: file_size as i64,
        width: width.map(|w| w as i64),
        height: height.map(|h| h as i64),
        duration: None,
        local_path: target_path.to_string_lossy().to_string(),
        thumbnail_path: thumb_small.clone(),
        last_accessed_at: now,
        ref_count: 1,
        source: params.source.clone(),
        status: "active".to_string(),
    };

    {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard
            .upsert_media_asset(&record)
            .map_err(|e| e.to_string())?;
    }

    Ok(MediaAssetResponse {
        asset_id,
        content_hash,
        mime_type: detected_mime,
        size: file_size,
        width,
        height,
        local_path: target_path.to_string_lossy().to_string(),
        thumbnail_small_path: thumb_small,
        thumbnail_large_path: thumb_large,
        last_accessed_at: now,
        ref_count: 1,
        source: params.source,
        status: "active".to_string(),
    })
}

#[tauri::command]
pub fn download_url_to_media_asset(
    state: State<'_, AppState>,
    url: String,
    _file_type: Option<String>,
    source: Option<String>,
    generate_thumbnails: Option<bool>,
) -> Result<MediaAssetResponse, String> {

    let parsed = reqwest::Url::parse(&url).map_err(|e| format!("无效 URL: {}", e))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("仅支持 HTTP/HTTPS 下载".to_string());
    }

    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    let (bytes, content_type, content_size) = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("Opentu/1.0")
            .build()
            .map_err(|e| e.to_string())?;

        let response = client
            .get(parsed.clone())
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("下载失败: HTTP {}", response.status()));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let content_size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let body = response.bytes().await.map_err(|e| e.to_string())?;
        Ok::<(Vec<u8>, String, Option<u64>), String>((body.to_vec(), content_type, content_size))
    })?;

    let file_size = content_size.unwrap_or(bytes.len() as u64);

    let content_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        use std::fmt::Write;
        for byte in &result[..] {
            let _ = write!(hex, "{:02x}", byte);
        }
        hex
    };

    let extension = extension_from_mime(&content_type);

    let cas = ContentAddressedStore::new(media_root.clone());
    let target_path = cas.content_path(&content_hash, &extension);

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let reused = target_path.exists();
    if !reused {
        let tmp_path = target_path.with_extension(format!("{}.tmp", extension));
        let mut tmp = File::create(&tmp_path).map_err(|e| e.to_string())?;
        tmp.write_all(&bytes).map_err(|e| e.to_string())?;
        tmp.flush().map_err(|e| e.to_string())?;
        drop(tmp);
        fs::rename(&tmp_path, &target_path).map_err(|e| e.to_string())?;
    }

    let (width, height) = if is_image_file(&target_path) {
        match read_image_info(&target_path) {
            Ok(info) => (Some(info.width), Some(info.height)),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let (thumb_small, thumb_large) = if generate_thumbnails.unwrap_or(true)
        && is_image_file(&target_path)
    {
        let gen = ThumbnailGenerator::new(&cas);
        let mut small_path = None;
        let mut large_path = None;
        if let Ok(small) = gen.generate(&target_path, &content_hash, ThumbnailSize::Small)
        {
            small_path = Some(small.local_path.to_string_lossy().to_string());
        }
        if let Ok(large) = gen.generate(&target_path, &content_hash, ThumbnailSize::Large)
        {
            large_path = Some(large.local_path.to_string_lossy().to_string());
        }
        (small_path, large_path)
    } else {
        (None, None)
    };

    let now = chrono::Utc::now().timestamp();
    let asset_id = format!("asset-{}", content_hash.clone());

    let record = MediaAssetRecord {
        asset_id: asset_id.clone(),
        content_hash: content_hash.clone(),
        mime_type: Some(content_type.clone()),
        size: file_size as i64,
        width: width.map(|w| w as i64),
        height: height.map(|h| h as i64),
        duration: None,
        local_path: target_path.to_string_lossy().to_string(),
        thumbnail_path: thumb_small.clone(),
        last_accessed_at: now,
        ref_count: 1,
        source: source.clone(),
        status: "active".to_string(),
    };

    {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard
            .upsert_media_asset(&record)
            .map_err(|e| e.to_string())?;
    }

    Ok(MediaAssetResponse {
        asset_id,
        content_hash,
        mime_type: content_type,
        size: file_size,
        width,
        height,
        local_path: target_path.to_string_lossy().to_string(),
        thumbnail_small_path: thumb_small,
        thumbnail_large_path: thumb_large,
        last_accessed_at: now,
        ref_count: 1,
        source,
        status: "active".to_string(),
    })
}

#[tauri::command]
pub fn import_blob_to_media_asset(
    state: State<'_, AppState>,
    blob: Vec<u8>,
    _file_type: Option<String>,
    original_name: Option<String>,
    mime_type: Option<String>,
    source: Option<String>,
    generate_thumbnails: Option<bool>,
) -> Result<MediaAssetResponse, String> {

    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };

    let content_hash = {
        let mut hasher = Sha256::new();
        hasher.update(&blob);
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        use std::fmt::Write;
        for byte in &result[..] {
            let _ = write!(hex, "{:02x}", byte);
        }
        hex
    };

    let detected_mime = mime_type.clone().unwrap_or_else(|| {
        original_name
            .as_ref()
            .map(|name| {
                let path = PathBuf::from(name);
                detect_mime_type(&path)
            })
            .unwrap_or_else(|| "application/octet-stream".to_string())
    });

    let extension = extension_from_mime(&detected_mime);

    let cas = ContentAddressedStore::new(media_root.clone());
    let target_path = cas.content_path(&content_hash, &extension);

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let reused = target_path.exists();
    if !reused {
        let tmp_path = target_path.with_extension(format!("{}.tmp", extension));
        let mut tmp = File::create(&tmp_path).map_err(|e| e.to_string())?;
        tmp.write_all(&blob).map_err(|e| e.to_string())?;
        tmp.flush().map_err(|e| e.to_string())?;
        drop(tmp);
        fs::rename(&tmp_path, &target_path).map_err(|e| e.to_string())?;
    }

    let file_size = blob.len() as u64;

    let (width, height) = if is_image_file(&target_path) {
        match read_image_info(&target_path) {
            Ok(info) => (Some(info.width), Some(info.height)),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let (thumb_small, thumb_large) = if generate_thumbnails.unwrap_or(true)
        && is_image_file(&target_path)
    {
        let gen = ThumbnailGenerator::new(&cas);
        let mut small_path = None;
        let mut large_path = None;
        if let Ok(small) = gen.generate(&target_path, &content_hash, ThumbnailSize::Small)
        {
            small_path = Some(small.local_path.to_string_lossy().to_string());
        }
        if let Ok(large) = gen.generate(&target_path, &content_hash, ThumbnailSize::Large)
        {
            large_path = Some(large.local_path.to_string_lossy().to_string());
        }
        (small_path, large_path)
    } else {
        (None, None)
    };

    let now = chrono::Utc::now().timestamp();
    let asset_id = format!("asset-{}", content_hash.clone());

    let record = MediaAssetRecord {
        asset_id: asset_id.clone(),
        content_hash: content_hash.clone(),
        mime_type: Some(detected_mime.clone()),
        size: file_size as i64,
        width: width.map(|w| w as i64),
        height: height.map(|h| h as i64),
        duration: None,
        local_path: target_path.to_string_lossy().to_string(),
        thumbnail_path: thumb_small.clone(),
        last_accessed_at: now,
        ref_count: 1,
        source: source.clone(),
        status: "active".to_string(),
    };

    {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard
            .upsert_media_asset(&record)
            .map_err(|e| e.to_string())?;
    }

    Ok(MediaAssetResponse {
        asset_id,
        content_hash,
        mime_type: detected_mime,
        size: file_size,
        width,
        height,
        local_path: target_path.to_string_lossy().to_string(),
        thumbnail_small_path: thumb_small,
        thumbnail_large_path: thumb_large,
        last_accessed_at: now,
        ref_count: 1,
        source,
        status: "active".to_string(),
    })
}

#[tauri::command]
pub fn get_media_cache_stats(
    state: State<'_, AppState>,
) -> Result<MediaCacheStatsResponse, String> {
    let db = {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard.clone_db()
    };

    let manager = crate::cache_cleanup::MediaCacheManager::new(&db);
    let raw = manager.get_stats();

    let total = raw.total_blobs_size_bytes + raw.total_thumbnails_size_bytes;
    let max = raw.max_cache_size_bytes;
    let percent = if max > 0 {
        (total as f64 / max as f64) * 100.0
    } else {
        0.0
    };

    Ok(MediaCacheStatsResponse {
        total_size_bytes: total,
        total_size_human: crate::cache_cleanup::format_size(total),
        blob_size_bytes: raw.total_blobs_size_bytes,
        thumbnail_size_bytes: raw.total_thumbnails_size_bytes,
        asset_count: raw.asset_count as u64,
        max_cache_size_bytes: max,
        max_cache_size_human: crate::cache_cleanup::format_size(max),
        percent_used: percent,
    })
}

#[tauri::command]
pub fn set_media_cache_max_size(
    state: State<'_, AppState>,
    max_size_bytes: u64,
) -> Result<MediaCacheStatsResponse, String> {
    {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard
            .set_cache_setting(
                "max_cache_size_bytes",
                &max_size_bytes.to_string(),
            )
            .map_err(|e| e.to_string())?;
    }
    get_media_cache_stats(state)
}

#[tauri::command]
pub fn run_media_cache_cleanup(
    state: State<'_, AppState>,
    only_orphans: Option<bool>,
) -> Result<MediaCleanupResponse, String> {
    let db = {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard.clone_db()
    };
    let manager = crate::cache_cleanup::MediaCacheManager::new(&db);

    if only_orphans.unwrap_or(false) {
        let result = manager.run_startup_check().map_err(|e| e.to_string())?;
        return Ok(MediaCleanupResponse {
            removed_count: 0,
            freed_bytes: result.freed_bytes,
            orphan_files_removed: result.orphan_files_removed as u64,
            orphan_records_removed: result.orphan_records_removed as u64,
        });
    }

    let orphan_result = manager
        .run_startup_check()
        .unwrap_or(crate::cache_cleanup::CleanupResult {
            removed_count: 0,
            freed_bytes: 0,
            orphan_files_removed: 0,
            orphan_records_removed: 0,
        });

    let cleanup_result = manager.cleanup_if_needed().map_err(|e| e.to_string())?;

    Ok(MediaCleanupResponse {
        removed_count: cleanup_result.removed_count as u64,
        freed_bytes: orphan_result.freed_bytes + cleanup_result.freed_bytes,
        orphan_files_removed: orphan_result.orphan_files_removed as u64,
        orphan_records_removed: orphan_result.orphan_records_removed as u64,
    })
}

#[tauri::command]
pub fn get_asset_runtime_url(
    state: State<'_, AppState>,
    asset_id: String,
) -> Result<Option<String>, String> {
    let db = {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard.clone_db()
    };

    if let Some(record) = db.get_media_asset(&asset_id) {
        let path = PathBuf::from(&record.local_path);
        let encoded = percent_encode_path(&path);
        return Ok(Some(format!(
            "opentu-asset://localhost/{}",
            encoded
        )));
    }

    Ok(None)
}

#[tauri::command]
pub fn get_thumbnail_runtime_url(
    state: State<'_, AppState>,
    content_hash: String,
    size: Option<String>,
) -> Result<Option<String>, String> {
    let (media_root, db_thumbnail_path) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let media_root = db.media_root.clone();
        // 查询数据库中是否有该 content_hash 的缩略图路径
        let thumbnail_path = db
            .conn
            .query_row(
                "SELECT thumbnail_path FROM media_assets WHERE content_hash = ?1 LIMIT 1",
                rusqlite::params![content_hash],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        (media_root, thumbnail_path)
    };

    let cas = ContentAddressedStore::new(media_root);
    let size_str = match size.as_deref() {
        Some("large") => "large",
        _ => "small",
    };

    // 优先检查 CAS 标准路径
    let thumb_path = cas.thumbnail_path(&content_hash, size_str);
    if thumb_path.exists() {
        let encoded = percent_encode_path(&thumb_path);
        return Ok(Some(format!("opentu-asset://localhost/{}", encoded)));
    }

    // 回退：检查 DB 中存储的 thumbnail_path
    if let Some(ref db_path) = db_thumbnail_path {
        let path = PathBuf::from(db_path);
        if path.exists() {
            let encoded = percent_encode_path(&path);
            return Ok(Some(format!("opentu-asset://localhost/{}", encoded)));
        }
    }

    Ok(None)
}

#[tauri::command]
pub fn list_media_assets_paginated(
    state: State<'_, AppState>,
    offset: usize,
    limit: usize,
    status_filter: Option<String>,
) -> Result<Vec<MediaAssetResponse>, String> {
    let db = {
        let db_guard = state.db.lock().map_err(|e| e.to_string())?;
        db_guard.clone_db()
    };

    let records = db.get_media_assets_paginated(
        offset,
        limit.max(1).min(200),
        status_filter.as_deref(),
    );

    let cas = ContentAddressedStore::new(db.media_root.clone());

    let mut responses = Vec::with_capacity(records.len());
    for record in records {
        let large_thumb_path = cas
            .thumbnail_path(&record.content_hash, "large");
        let has_large = large_thumb_path.exists();

        responses.push(MediaAssetResponse {
            asset_id: record.asset_id,
            content_hash: record.content_hash,
            mime_type: record.mime_type.unwrap_or_default(),
            size: record.size as u64,
            width: record.width.map(|w| w as u32),
            height: record.height.map(|h| h as u32),
            local_path: record.local_path,
            thumbnail_small_path: record.thumbnail_path.clone(),
            thumbnail_large_path: if has_large {
                Some(large_thumb_path.to_string_lossy().to_string())
            } else {
                record.thumbnail_path
            },
            last_accessed_at: record.last_accessed_at,
            ref_count: record.ref_count,
            source: record.source,
            status: record.status,
        });
    }

    Ok(responses)
}

fn detect_mime_type(path: &Path) -> String {
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "m4v" => "video/x-m4v",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn extension_from_mime(mime_type: &str) -> String {
    match mime_type.to_ascii_lowercase().as_str() {
        "image/jpeg" => "jpg".to_string(),
        "image/png" => "png".to_string(),
        "image/gif" => "gif".to_string(),
        "image/webp" => "webp".to_string(),
        "image/bmp" => "bmp".to_string(),
        "image/svg+xml" => "svg".to_string(),
        "video/mp4" => "mp4".to_string(),
        "video/webm" => "webm".to_string(),
        "video/quicktime" => "mov".to_string(),
        "audio/mpeg" => "mp3".to_string(),
        "audio/wav" => "wav".to_string(),
        "application/zip" => "zip".to_string(),
        _ => "bin".to_string(),
    }
}

fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()).as_deref(),
        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp") | Some("webp") | Some("tiff") | Some("tif")
    )
}

fn percent_encode_path(path: &Path) -> String {
    path.to_string_lossy()
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

impl crate::database::Database {
    pub fn clone_db(&self) -> Self {
        Self {
            conn: rusqlite::Connection::open(&self
                .get_connection_path())
                .expect("Failed to reopen database"),
            data_dir: self.data_dir.clone(),
            media_root: self.media_root.clone(),
        }
    }

    fn get_connection_path(&self) -> PathBuf {
        self.data_dir.join("opentu.db")
    }
}
