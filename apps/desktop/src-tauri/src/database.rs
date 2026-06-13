use rusqlite::{Connection, Result as SqliteResult};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Database {
    pub conn: Connection,
    pub data_dir: PathBuf,
    pub media_root: PathBuf,
}

impl Database {
    pub fn new(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("opentu.db");
        let conn = Connection::open(&db_path)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let mut db = Database {
            conn,
            data_dir: data_dir.clone(),
            media_root: data_dir.join("media"),
        };
        db.init_schema()?;

        let custom_root: Option<String> = db
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'media_root_path'",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        if let Some(custom_path) = custom_root {
            let custom = PathBuf::from(&custom_path);
            if custom.exists() || fs::create_dir_all(&custom).is_ok() {
                db.media_root = custom;
            }
        }

        db.ensure_media_dirs()?;
        fs::create_dir_all(data_dir.join("backups"))?;
        fs::create_dir_all(data_dir.join("exports"))?;
        fs::create_dir_all(data_dir.join("logs"))?;

        Ok(db)
    }

    pub fn ensure_media_dirs(&self) -> Result<(), Box<dyn std::error::Error>> {
        for subdir in ["图片", "视频", "音频", "PPT", "文本", "压缩包"] {
            fs::create_dir_all(self.media_root.join(subdir))?;
        }
        fs::create_dir_all(self.media_root.join("blobs"))?;
        fs::create_dir_all(self.media_root.join("thumbs"))?;
        Ok(())
    }

    pub fn set_media_root(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if path.as_os_str().is_empty() {
            return Err("媒体目录路径不能为空".into());
        }
        if path.is_file() {
            return Err("媒体目录不能指向文件".into());
        }

        fs::create_dir_all(&path)?;
        let path = path.canonicalize()?;
        self.media_root = path.clone();

        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES ('media_root_path', ?1, strftime('%s','now'))",
            rusqlite::params![path.to_string_lossy().to_string()],
        )?;

        self.ensure_media_dirs()?;
        Ok(())
    }

    pub fn reset_media_root(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let default_root = self.data_dir.join("media");
        self.set_media_root(default_root)
    }

    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE TABLE IF NOT EXISTS workspaces (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                data       TEXT NOT NULL,
                thumbnail  TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS assets (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                type       TEXT NOT NULL,
                mime_type  TEXT,
                local_path TEXT NOT NULL,
                file_size  INTEGER NOT NULL DEFAULT 0,
                width      INTEGER,
                height     INTEGER,
                duration   REAL,
                metadata   TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id           TEXT PRIMARY KEY,
                type         TEXT NOT NULL,
                status       TEXT NOT NULL DEFAULT 'pending',
                params       TEXT,
                result       TEXT,
                error        TEXT,
                created_at   INTEGER NOT NULL,
                completed_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS knowledge_notes (
                id         TEXT PRIMARY KEY,
                title      TEXT NOT NULL,
                content    TEXT NOT NULL,
                tags       TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chat_history (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at      INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS media_assets (
                asset_id        TEXT PRIMARY KEY,
                content_hash    TEXT NOT NULL,
                mime_type       TEXT,
                size            INTEGER NOT NULL DEFAULT 0,
                width           INTEGER,
                height          INTEGER,
                duration        REAL,
                local_path      TEXT NOT NULL,
                thumbnail_path  TEXT,
                last_accessed_at INTEGER NOT NULL,
                ref_count       INTEGER NOT NULL DEFAULT 0,
                source          TEXT,
                status          TEXT NOT NULL DEFAULT 'active'
            );

            CREATE TABLE IF NOT EXISTS cache_settings (
                key       TEXT PRIMARY KEY,
                value     TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE INDEX IF NOT EXISTS idx_assets_type    ON assets(type);
            CREATE INDEX IF NOT EXISTS idx_assets_created ON assets(created_at);
            CREATE INDEX IF NOT EXISTS idx_tasks_status   ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_notes_updated  ON knowledge_notes(updated_at);
            CREATE INDEX IF NOT EXISTS idx_media_content_hash ON media_assets(content_hash);
            CREATE INDEX IF NOT EXISTS idx_media_last_accessed ON media_assets(last_accessed_at);
            CREATE INDEX IF NOT EXISTS idx_media_status        ON media_assets(status);
            ",
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MediaAssetRecord {
    pub asset_id: String,
    pub content_hash: String,
    pub mime_type: Option<String>,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration: Option<f64>,
    pub local_path: String,
    pub thumbnail_path: Option<String>,
    pub last_accessed_at: i64,
    pub ref_count: i32,
    pub source: Option<String>,
    pub status: String,
}

impl Database {
    pub fn upsert_media_asset(
        &self,
        asset: &MediaAssetRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT OR REPLACE INTO media_assets (asset_id, content_hash, mime_type, size, width, height, duration, local_path, thumbnail_path, last_accessed_at, ref_count, source, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                asset.asset_id,
                asset.content_hash,
                asset.mime_type,
                asset.size,
                asset.width,
                asset.height,
                asset.duration,
                asset.local_path,
                asset.thumbnail_path,
                asset.last_accessed_at,
                asset.ref_count,
                asset.source,
                asset.status,
            ],
        )?;
        Ok(())
    }

    pub fn get_media_asset(&self, asset_id: &str) -> Option<MediaAssetRecord> {
        self.conn.query_row(
            "SELECT asset_id, content_hash, mime_type, size, width, height, duration, local_path, thumbnail_path, last_accessed_at, ref_count, source, status
             FROM media_assets WHERE asset_id = ?1",
            rusqlite::params![asset_id],
            |row| {
                Ok(MediaAssetRecord {
                    asset_id: row.get(0)?,
                    content_hash: row.get(1)?,
                    mime_type: row.get(2)?,
                    size: row.get(3)?,
                    width: row.get(4)?,
                    height: row.get(5)?,
                    duration: row.get(6)?,
                    local_path: row.get(7)?,
                    thumbnail_path: row.get(8)?,
                    last_accessed_at: row.get(9)?,
                    ref_count: row.get(10)?,
                    source: row.get(11)?,
                    status: row.get(12)?,
                })
            },
        )
        .ok()
    }

    pub fn get_media_assets_paginated(
        &self,
        offset: usize,
        limit: usize,
        status_filter: Option<&str>,
    ) -> Vec<MediaAssetRecord> {
        let mut query = String::from(
            "SELECT asset_id, content_hash, mime_type, size, width, height, duration, local_path, thumbnail_path, last_accessed_at, ref_count, source, status FROM media_assets",
        );
        if status_filter.is_some() {
            query.push_str(" WHERE status = ?1");
        }
        query.push_str(" ORDER BY last_accessed_at DESC LIMIT ? OFFSET ?");

        let mut stmt = match self.conn.prepare(&query) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let result: Result<Vec<MediaAssetRecord>, _> = match status_filter {
            Some(status) => stmt
                .query_map(rusqlite::params![status, limit as i64, offset as i64], |row| {
                    Ok(MediaAssetRecord {
                        asset_id: row.get(0)?,
                        content_hash: row.get(1)?,
                        mime_type: row.get(2)?,
                        size: row.get(3)?,
                        width: row.get(4)?,
                        height: row.get(5)?,
                        duration: row.get(6)?,
                        local_path: row.get(7)?,
                        thumbnail_path: row.get(8)?,
                        last_accessed_at: row.get(9)?,
                        ref_count: row.get(10)?,
                        source: row.get(11)?,
                        status: row.get(12)?,
                    })
                })
                .map(|iter| iter.filter_map(|r| r.ok()).collect()),
            None => stmt
                .query_map(rusqlite::params![limit as i64, offset as i64], |row| {
                    Ok(MediaAssetRecord {
                        asset_id: row.get(0)?,
                        content_hash: row.get(1)?,
                        mime_type: row.get(2)?,
                        size: row.get(3)?,
                        width: row.get(4)?,
                        height: row.get(5)?,
                        duration: row.get(6)?,
                        local_path: row.get(7)?,
                        thumbnail_path: row.get(8)?,
                        last_accessed_at: row.get(9)?,
                        ref_count: row.get(10)?,
                        source: row.get(11)?,
                        status: row.get(12)?,
                    })
                })
                .map(|iter| iter.filter_map(|r| r.ok()).collect()),
        };

        result.unwrap_or_default()
    }

    pub fn delete_media_asset(&self, asset_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "DELETE FROM media_assets WHERE asset_id = ?1",
            rusqlite::params![asset_id],
        )?;
        Ok(())
    }

    pub fn touch_media_asset(
        &self,
        asset_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "UPDATE media_assets SET last_accessed_at = ?1 WHERE asset_id = ?2",
            rusqlite::params![now, asset_id],
        )?;
        Ok(())
    }

    pub fn update_ref_count(
        &self,
        asset_id: &str,
        delta: i32,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        self.conn.execute(
            "UPDATE media_assets SET ref_count = ref_count + ?1 WHERE asset_id = ?2",
            rusqlite::params![delta, asset_id],
        )?;
        let new_count: i32 = self.conn.query_row(
            "SELECT ref_count FROM media_assets WHERE asset_id = ?1",
            rusqlite::params![asset_id],
            |row| row.get(0),
        )?;
        Ok(new_count)
    }

    pub fn get_cache_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM cache_settings WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn set_cache_setting(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO cache_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, value, now],
        )?;
        Ok(())
    }

    pub fn count_media_assets(
        &self,
        status_filter: Option<&str>,
    ) -> i64 {
        if let Some(status) = status_filter {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM media_assets WHERE status = ?1",
                    rusqlite::params![status],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
        } else {
            self.conn
                .query_row(
                    "SELECT COUNT(*) FROM media_assets",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
        }
    }

    pub fn get_total_media_db_size(&self) -> i64 {
        self.conn
            .query_row("SELECT COALESCE(SUM(size), 0) FROM media_assets", [], |row| row.get(0))
            .unwrap_or(0)
    }
}
