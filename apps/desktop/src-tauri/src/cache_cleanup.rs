use crate::cas::ContentAddressedStore;
use crate::database::{Database, MediaAssetRecord};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_CACHE_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const SETTING_KEY_MAX_SIZE: &str = "max_cache_size_bytes";

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_blobs_size_bytes: u64,
    pub total_thumbnails_size_bytes: u64,
    pub total_db_size_bytes: u64,
    pub asset_count: i64,
    pub max_cache_size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub removed_count: usize,
    pub freed_bytes: u64,
    pub orphan_files_removed: usize,
    pub orphan_records_removed: usize,
}

pub struct MediaCacheManager<'a> {
    db: &'a Database,
    cas: ContentAddressedStore,
}

impl<'a> MediaCacheManager<'a> {
    pub fn new(db: &'a Database) -> Self {
        let cas = ContentAddressedStore::new(db.media_root.clone());
        Self { db, cas }
    }

    pub fn get_max_cache_size(&self) -> u64 {
        match self.db.get_cache_setting(SETTING_KEY_MAX_SIZE) {
            Some(value) => value.parse::<u64>().unwrap_or(DEFAULT_MAX_CACHE_SIZE_BYTES),
            None => DEFAULT_MAX_CACHE_SIZE_BYTES,
        }
    }

    pub fn set_max_cache_size(
        &self,
        size_bytes: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.db
            .set_cache_setting(SETTING_KEY_MAX_SIZE, &size_bytes.to_string())?;
        Ok(())
    }

    pub fn get_stats(&self) -> CacheStats {
        let blobs_size = self.cas.total_blob_size();
        let thumbs_size = total_thumbs_size(&self.db.media_root);
        let db_size = self.db.get_total_media_db_size() as u64;
        let count = self.db.count_media_assets(None);

        CacheStats {
            total_blobs_size_bytes: blobs_size,
            total_thumbnails_size_bytes: thumbs_size,
            total_db_size_bytes: db_size,
            asset_count: count,
            max_cache_size_bytes: self.get_max_cache_size(),
        }
    }

    pub fn run_startup_check(
        &self,
    ) -> Result<CleanupResult, Box<dyn std::error::Error + Send + Sync>> {
        let orphan_files = self.find_orphan_files();
        let orphan_records = self.find_orphan_records();

        let mut files_removed = 0usize;
        let mut bytes_freed: u64 = 0;

        for path in &orphan_files {
            if let Ok(meta) = fs::metadata(path) {
                bytes_freed += meta.len();
            }
            let _ = fs::remove_file(path);
            files_removed += 1;
        }

        for (asset_id, _path) in &orphan_records {
            let _ = self.db.delete_media_asset(asset_id);
        }

        Ok(CleanupResult {
            removed_count: 0,
            freed_bytes: bytes_freed,
            orphan_files_removed: files_removed,
            orphan_records_removed: orphan_records.len(),
        })
    }

    fn find_orphan_files(&self) -> Vec<PathBuf> {
        let mut orphans = Vec::new();
        let blobs_root = self.db.media_root.join("blobs");
        if !blobs_root.exists() {
            return orphans;
        }

        if let Ok(entries) = fs::read_dir(&blobs_root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            let path = sub_entry.path();
                            if !path.is_file() {
                                continue;
                            }
                            let hash = extract_hash_from_path(&path);
                            let exists = self
                                .db
                                .conn
                                .query_row(
                                    "SELECT 1 FROM media_assets WHERE content_hash = ?1 LIMIT 1",
                                    rusqlite::params![hash],
                                    |_row| Ok(()),
                                )
                                .is_ok();
                            if !exists {
                                orphans.push(path);
                            }
                        }
                    }
                }
            }
        }
        orphans
    }

    fn find_orphan_records(&self) -> Vec<(String, String)> {
        let mut orphans = Vec::new();
        let assets = self.db.get_media_assets_paginated(0, 10000, None);
        for asset in assets {
            let path = PathBuf::from(&asset.local_path);
            if !path.exists() {
                orphans.push((asset.asset_id, asset.local_path));
            }
        }
        orphans
    }

    pub fn cleanup_if_needed(
        &self,
    ) -> Result<CleanupResult, Box<dyn std::error::Error + Send + Sync>> {
        let stats = self.get_stats();
        let current_size = stats.total_blobs_size_bytes + stats.total_thumbnails_size_bytes;
        let max_size = stats.max_cache_size_bytes;

        if current_size <= max_size {
            return Ok(CleanupResult {
                removed_count: 0,
                freed_bytes: 0,
                orphan_files_removed: 0,
                orphan_records_removed: 0,
            });
        }

        let target_bytes = current_size.saturating_sub(max_size);
        self.cleanup_by_lru(target_bytes)
    }

    pub fn cleanup_by_lru(
        &self,
        target_bytes_to_free: u64,
    ) -> Result<CleanupResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut removed: usize = 0;
        let mut freed: u64 = 0;

        let mut offset: usize = 0;
        const BATCH_SIZE: usize = 100;

        loop {
            if freed >= target_bytes_to_free {
                break;
            }

            let candidates = self
                .db
                .conn
                .prepare(
                    "SELECT asset_id, content_hash, local_path, size FROM media_assets WHERE ref_count <= 0 AND status != 'protected' ORDER BY last_accessed_at ASC LIMIT ?1 OFFSET ?2",
                )
                .and_then(|mut stmt| {
                    let rows = stmt.query_map(
                        rusqlite::params![BATCH_SIZE as i64, offset as i64],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, i64>(3)?,
                            ))
                        },
                    )?;
                    Ok::<Vec<_>, rusqlite::Error>(rows.filter_map(|r| r.ok()).collect())
                })
                .unwrap_or_default();

            if candidates.is_empty() {
                break;
            }

            for (asset_id, content_hash, local_path, size) in candidates {
                if freed >= target_bytes_to_free {
                    break;
                }
                if let Ok(meta) = fs::metadata(&local_path) {
                    freed += meta.len();
                } else {
                    freed += size.max(0) as u64;
                }

                let _ = self.cas.remove_content(&content_hash);
                let _ = self.db.delete_media_asset(&asset_id);
                removed += 1;
            }

            if candidates.len() < BATCH_SIZE {
                break;
            }
            offset += BATCH_SIZE;
        }

        Ok(CleanupResult {
            removed_count: removed,
            freed_bytes: freed,
            orphan_files_removed: 0,
            orphan_records_removed: 0,
        })
    }

    pub fn cas(&self) -> &ContentAddressedStore {
        &self.cas
    }

    pub fn db(&self) -> &Database {
        self.db
    }
}

fn extract_hash_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.split('.').next().unwrap_or("").to_string())
        .unwrap_or_default()
}

fn total_thumbs_size(root: &Path) -> u64 {
    let thumbs = root.join("thumbs");
    let mut total: u64 = 0;
    if !thumbs.exists() {
        return 0;
    }
    if let Ok(entries) = fs::read_dir(&thumbs) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                    for sub_entry in sub_entries.flatten() {
                        if let Ok(meta) = sub_entry.metadata() {
                            if meta.is_file() {
                                total += meta.len();
                            }
                        }
                    }
                }
            }
        }
    }
    total
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()).as_deref(),
        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp") | Some("webp") | Some("tiff") | Some("tif")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_works() {
        assert!(format_size(0).ends_with("B"));
        assert!(format_size(2048).contains("KB"));
        assert!(format_size(5 * 1024 * 1024).contains("MB"));
        assert!(format_size(10 * 1024 * 1024 * 1024).contains("GB"));
    }

    #[test]
    fn is_image_recognizes_extensions() {
        assert!(is_image_file(&PathBuf::from("test.png")));
        assert!(is_image_file(&PathBuf::from("test.jpg")));
        assert!(is_image_file(&PathBuf::from("TEST.JPG")));
        assert!(!is_image_file(&PathBuf::from("test.mp4")));
        assert!(!is_image_file(&PathBuf::from("test")));
    }
}
