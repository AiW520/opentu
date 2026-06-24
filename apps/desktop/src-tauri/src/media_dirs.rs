use std::path::{Path, PathBuf};

struct MediaDir {
    primary: &'static str,
    legacy: &'static str,
    aliases: &'static [&'static str],
}

const MEDIA_DIRS: [MediaDir; 6] = [
    MediaDir {
        primary: "images",
        legacy: "图片",
        aliases: &["image"],
    },
    MediaDir {
        primary: "videos",
        legacy: "视频",
        aliases: &["video"],
    },
    MediaDir {
        primary: "audio",
        legacy: "音频",
        aliases: &["audio"],
    },
    MediaDir {
        primary: "ppt",
        legacy: "PPT",
        aliases: &["ppt", "presentation"],
    },
    MediaDir {
        primary: "text",
        legacy: "文本",
        aliases: &["text", "markdown", "json"],
    },
    MediaDir {
        primary: "archives",
        legacy: "压缩包",
        aliases: &["archive", "zip"],
    },
];

const ALL_MEDIA_SUBDIRS: [&str; 12] = [
    "images",
    "图片",
    "videos",
    "视频",
    "audio",
    "音频",
    "ppt",
    "PPT",
    "text",
    "文本",
    "archives",
    "压缩包",
];

pub fn primary_media_subdirs() -> impl Iterator<Item = &'static str> {
    MEDIA_DIRS.iter().map(|dir| dir.primary)
}

pub fn media_subdir(file_type: &str) -> &'static str {
    media_dir_for_type(file_type).primary
}

pub fn media_subdirs_for_type(file_type: &str) -> [&'static str; 2] {
    let dir = media_dir_for_type(file_type);
    [dir.primary, dir.legacy]
}

pub fn all_media_subdirs() -> impl Iterator<Item = &'static str> {
    ALL_MEDIA_SUBDIRS.iter().copied()
}

pub fn resolve_media_file_path(
    media_root: &Path,
    safe_file_name: &str,
    file_type: Option<&str>,
) -> PathBuf {
    if let Some(file_type) = file_type {
        for subdir in media_subdirs_for_type(file_type) {
            let candidate = media_root.join(subdir).join(safe_file_name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    for subdir in all_media_subdirs() {
        let candidate = media_root.join(subdir).join(safe_file_name);
        if candidate.exists() {
            return candidate;
        }
    }

    media_root
        .join(media_subdir(file_type.unwrap_or("image")))
        .join(safe_file_name)
}

fn media_dir_for_type(file_type: &str) -> &'static MediaDir {
    let normalized = file_type.to_ascii_lowercase();
    MEDIA_DIRS
        .iter()
        .find(|dir| dir.aliases.contains(&normalized.as_str()))
        .unwrap_or(&MEDIA_DIRS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_media_types_to_ascii_primary_subdirs() {
        assert_eq!(media_subdir("image"), "images");
        assert_eq!(media_subdir("video"), "videos");
        assert_eq!(media_subdir("audio"), "audio");
        assert_eq!(media_subdir("ppt"), "ppt");
        assert_eq!(media_subdir("presentation"), "ppt");
        assert_eq!(media_subdir("text"), "text");
        assert_eq!(media_subdir("markdown"), "text");
        assert_eq!(media_subdir("archive"), "archives");
        assert_eq!(media_subdir("zip"), "archives");
    }
}
