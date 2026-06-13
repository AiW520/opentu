use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

pub const CAS_BLOBS_DIR: &str = "blobs";
pub const CAS_THUMBS_DIR: &str = "thumbs";

#[derive(Debug)]
pub struct ContentAddressedStore {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ContentHash(pub String);

#[derive(Debug)]
pub struct StoredContent {
    pub content_hash: String,
    pub local_path: PathBuf,
    pub size: u64,
    pub reused: bool,
}

impl ContentAddressedStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn blobs_root(&self) -> PathBuf {
        self.root.join(CAS_BLOBS_DIR)
    }

    fn thumbs_root(&self) -> PathBuf {
        self.root.join(CAS_THUMBS_DIR)
    }

    pub fn content_path(&self, content_hash: &str, extension: &str) -> PathBuf {
        let hash_prefix = if content_hash.len() >= 2 {
            &content_hash[..2]
        } else {
            content_hash
        };
        let clean_ext = extension.trim_start_matches('.');
        self.blobs_root()
            .join(hash_prefix)
            .join(format!("{content_hash}.{clean_ext}"))
    }

    pub fn thumbnail_path(&self, content_hash: &str, size: &str) -> PathBuf {
        let hash_prefix = if content_hash.len() >= 2 {
            &content_hash[..2]
        } else {
            content_hash
        };
        self.thumbs_root()
            .join(hash_prefix)
            .join(format!("{content_hash}.{size}.webp"))
    }

    fn ensure_dirs(&self, content_hash: &str) -> std::io::Result<()> {
        let hash_prefix = if content_hash.len() >= 2 {
            &content_hash[..2]
        } else {
            content_hash
        };
        fs::create_dir_all(self.blobs_root().join(hash_prefix))?;
        fs::create_dir_all(self.thumbs_root().join(hash_prefix))?;
        Ok(())
    }

    pub fn compute_hash_sync(&self, reader: &mut dyn Read) -> std::io::Result<(String, u64)> {
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 128 * 1024];
        let mut total_size: u64 = 0;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
            total_size += bytes_read as u64;
        }

        let hash_result = hasher.finalize();
        let hex_hash = bytes_to_hex(&hash_result);
        Ok((hex_hash, total_size))
    }

    pub fn store_from_file(&self, source: &Path, extension: &str) -> std::io::Result<StoredContent> {
        let mut file = File::open(source)?;
        self.store_from_reader(&mut file, extension)
    }

    pub fn store_from_reader<R: Read>(
        &self,
        reader: &mut R,
        extension: &str,
    ) -> std::io::Result<StoredContent> {
        let mut buffer = Vec::new();
        let mut hasher = Sha256::new();
        let mut reader_buf = BufReader::new(reader);
        let mut read_buffer = [0u8; 128 * 1024];
        loop {
            let bytes_read = reader_buf.read(&mut read_buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&read_buffer[..bytes_read]);
            buffer.extend_from_slice(&read_buffer[..bytes_read]);
        }
        let hash_result = hasher.finalize();
        let content_hash = bytes_to_hex(&hash_result);
        let size = buffer.len() as u64;

        let target_path = self.content_path(&content_hash, extension);
        let mut reused = false;

        if target_path.exists() {
            reused = true;
        } else {
            self.ensure_dirs(&content_hash)?;
            let tmp_path = target_path.with_extension(format!(
                "{}.tmp",
                target_path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
            ));
            let mut tmp_file = File::create(&tmp_path)?;
            tmp_file.write_all(&buffer)?;
            tmp_file.flush()?;
            fs::rename(&tmp_path, &target_path)?;
        }

        Ok(StoredContent {
            content_hash,
            local_path: target_path,
            size,
            reused,
        })
    }

    pub fn get_content_path_if_exists(&self, content_hash: &str) -> Option<PathBuf> {
        let hash_prefix = if content_hash.len() >= 2 {
            &content_hash[..2]
        } else {
            content_hash
        };
        let dir = self.blobs_root().join(hash_prefix);
        if dir.exists() && dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with(content_hash) {
                            return Some(path);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn remove_content(&self, content_hash: &str) -> std::io::Result<()> {
        if let Some(path) = self.get_content_path_if_exists(content_hash) {
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        let small_thumb = self.thumbnail_path(content_hash, "small");
        if small_thumb.exists() {
            let _ = fs::remove_file(&small_thumb);
        }
        let large_thumb = self.thumbnail_path(content_hash, "large");
        if large_thumb.exists() {
            let _ = fs::remove_file(&large_thumb);
        }
        Ok(())
    }

    pub fn total_blob_size(&self) -> u64 {
        let mut total: u64 = 0;
        if let Ok(entries) = fs::read_dir(self.blobs_root()) {
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
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{:02x}", byte));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_path_format() {
        let store = ContentAddressedStore::new(PathBuf::from("/tmp/opentu-test"));
        let hash = "abcdef1234567890";
        let path = store.content_path(hash, "png");
        assert!(path.to_string_lossy().contains("/blobs/ab/"));
        assert!(path.to_string_lossy().ends_with("abcdef1234567890.png"));
    }

    #[test]
    fn thumbnail_path_format() {
        let store = ContentAddressedStore::new(PathBuf::from("/tmp/opentu-test"));
        let hash = "abcdef1234567890";
        let path = store.thumbnail_path(hash, "small");
        assert!(path.to_string_lossy().contains("/thumbs/ab/"));
        assert!(path.to_string_lossy().ends_with("abcdef1234567890.small.webp"));
    }

    #[test]
    fn roundtrip_store_and_dedupe() {
        let dir = std::env::temp_dir().join(format!(
            "opentu-cas-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        let store = ContentAddressedStore::new(dir.clone());

        let source = dir.join("input.dat");
        fs::create_dir_all(&dir).unwrap();
        let data = b"hello world content";
        fs::write(&source, data).unwrap();

        let result1 = store.store_from_file(&source, "bin").unwrap();
        assert!(!result1.reused);
        assert_eq!(result1.size, data.len() as u64);
        assert!(result1.local_path.exists());

        let result2 = store.store_from_file(&source, "bin").unwrap();
        assert!(result2.reused);
        assert_eq!(result1.content_hash, result2.content_hash);

        let _ = fs::remove_dir_all(&dir);
    }
}
