use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const GRANT_TTL: Duration = Duration::from_secs(60 * 60);
const MAX_PATH_GRANTS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GrantKind {
    ReadFile,
    WriteFile,
    Directory,
}

#[derive(Clone, Debug)]
struct PathGrant {
    kind: GrantKind,
    path: PathBuf,
    expires_at: Instant,
}

#[derive(Default)]
pub struct PathGrantStore {
    grants: Vec<PathGrant>,
}

impl PathGrantStore {
    pub fn grant_read_file(&mut self, path: &Path) -> Result<PathBuf, String> {
        let path = canonical_existing_file(path)?;
        self.insert(GrantKind::ReadFile, path.clone());
        Ok(path)
    }

    pub fn grant_write_file(&mut self, path: &Path) -> Result<PathBuf, String> {
        let path = canonical_write_file(path)?;
        self.insert(GrantKind::WriteFile, path.clone());
        Ok(path)
    }

    pub fn grant_directory(&mut self, path: &Path) -> Result<PathBuf, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("Directory is not accessible: {}", e))?;
        if !path.is_dir() {
            return Err("Granted path is not a directory".to_string());
        }
        self.insert(GrantKind::Directory, path.clone());
        Ok(path)
    }

    pub fn allows_read_file(&mut self, path: &Path, media_root: &Path) -> bool {
        self.prune_expired();
        let Ok(path) = canonical_existing_file(path) else {
            return false;
        };
        path.starts_with(media_root)
            || self
                .grants
                .iter()
                .any(|grant| grant.kind == GrantKind::ReadFile && grant.path == path)
    }

    pub fn allows_write_file(&mut self, path: &Path, media_root: &Path) -> bool {
        self.prune_expired();
        let Ok(path) = canonical_write_file(path) else {
            return false;
        };
        path.starts_with(media_root)
            || self
                .grants
                .iter()
                .any(|grant| grant.kind == GrantKind::WriteFile && grant.path == path)
    }

    pub fn allows_directory(&mut self, path: &Path) -> bool {
        self.prune_expired();
        let Ok(path) = path.canonicalize() else {
            return false;
        };
        self.grants
            .iter()
            .any(|grant| grant.kind == GrantKind::Directory && grant.path == path)
    }

    fn insert(&mut self, kind: GrantKind, path: PathBuf) {
        self.prune_expired();
        let expires_at = Instant::now() + GRANT_TTL;
        if let Some(existing) = self
            .grants
            .iter_mut()
            .find(|grant| grant.kind == kind && grant.path == path)
        {
            existing.expires_at = expires_at;
            return;
        }
        if self.grants.len() >= MAX_PATH_GRANTS {
            let overflow = self.grants.len() + 1 - MAX_PATH_GRANTS;
            self.grants.drain(0..overflow);
        }
        self.grants.push(PathGrant {
            kind,
            path,
            expires_at,
        });
    }

    fn prune_expired(&mut self) {
        let now = Instant::now();
        self.grants.retain(|grant| grant.expires_at > now);
    }
}

pub fn canonical_existing_file(path: &Path) -> Result<PathBuf, String> {
    let path = path
        .canonicalize()
        .map_err(|e| format!("File is not accessible: {}", e))?;
    if !path.is_file() {
        return Err("Path is not a file".to_string());
    }
    Ok(path)
}

pub fn canonical_write_file(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("Save path cannot be empty".to_string());
    }
    if path.exists() && path.is_dir() {
        return Err("Save path cannot be a directory".to_string());
    }

    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("Save directory is not accessible: {}", e))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "Save path is missing a file name".to_string())?;
    Ok(parent.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_grant_allows_only_exact_file() {
        let root = std::env::temp_dir().join(format!(
            "opentu-grant-{}-{}",
            std::process::id(),
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis())
        ));
        std::fs::create_dir_all(&root).unwrap();
        let granted = root.join("a.txt");
        let other = root.join("b.txt");
        let media_root = root.join("media");
        std::fs::create_dir_all(&media_root).unwrap();

        let mut store = PathGrantStore::default();
        store.grant_write_file(&granted).unwrap();

        assert!(store.allows_write_file(&granted, &media_root));
        assert!(!store.allows_write_file(&other, &media_root));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn media_root_write_does_not_need_grant() {
        let root = std::env::temp_dir().join(format!(
            "opentu-media-root-grant-{}-{}",
            std::process::id(),
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis())
        ));
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("image.png");
        let mut store = PathGrantStore::default();

        assert!(store.allows_write_file(&target, &root.canonicalize().unwrap()));

        let _ = std::fs::remove_dir_all(&root);
    }
}
