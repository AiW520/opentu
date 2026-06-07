use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use tauri::State;

use crate::database::Database;
use crate::AppState;

#[derive(Debug, serde::Serialize)]
pub struct FileOperationResult {
    pub success: bool,
    pub message: String,
    pub target_path: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub enum ConflictStrategy {
    Overwrite,
    Rename,
    Skip,
    Fail,
}

#[tauri::command]
pub async fn move_file_to_media(
    state: State<'_, AppState>,
    source_path: String,
    file_type: Option<String>,
    conflict_strategy: Option<ConflictStrategy>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    match move_file_to_media_internal(
        &db,
        &source_path,
        file_type.as_deref(),
        conflict_strategy.as_ref(),
    ) {
        Ok((target_path, file_size)) => Ok(success_result(
            "文件移动成功",
            Some(target_path),
            Some(file_size),
        )),
        Err(error) => Ok(fail_result(error)),
    }
}

#[tauri::command]
pub async fn copy_file_to_media(
    state: State<'_, AppState>,
    source_path: String,
    file_type: Option<String>,
    conflict_strategy: Option<ConflictStrategy>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    match copy_file_to_media_internal(
        &db,
        &source_path,
        file_type.as_deref(),
        conflict_strategy.as_ref(),
    ) {
        Ok((target_path, file_size)) => Ok(success_result(
            "文件复制成功",
            Some(target_path),
            Some(file_size),
        )),
        Err(error) => Ok(fail_result(error)),
    }
}

#[tauri::command]
pub async fn delete_media_file(
    state: State<'_, AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    match delete_media_file_internal(&db, &file_name, file_type.as_deref()) {
        Ok(_) => Ok(success_result("文件删除成功", None, None)),
        Err(error) => Ok(fail_result(error)),
    }
}

#[tauri::command]
pub async fn verify_file_accessible(
    state: State<'_, AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let file_path = db
        .media_root
        .join(get_media_subdir(file_type.as_deref().unwrap_or("image")))
        .join(sanitize_file_name(&file_name)?);

    match fs::metadata(&file_path) {
        Ok(metadata) if metadata.is_file() => Ok(success_result(
            "文件存在且可访问",
            Some(file_path.to_string_lossy().to_string()),
            Some(metadata.len()),
        )),
        Ok(_) => Ok(FileOperationResult {
            success: false,
            message: "路径不是文件".to_string(),
            target_path: Some(file_path.to_string_lossy().to_string()),
            file_size: None,
        }),
        Err(error) => Ok(FileOperationResult {
            success: false,
            message: format!("文件不存在或无法访问: {}", error),
            target_path: Some(file_path.to_string_lossy().to_string()),
            file_size: None,
        }),
    }
}

#[tauri::command]
pub async fn list_media_files(
    state: State<'_, AppState>,
    file_type: Option<String>,
) -> Result<Vec<MediaFileInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let media_dir = db
        .media_root
        .join(get_media_subdir(file_type.as_deref().unwrap_or("image")));

    let entries = fs::read_dir(&media_dir).map_err(|error| format!("无法读取目录: {}", error))?;
    let mut files = Vec::new();

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                files.push(MediaFileInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    size: metadata.len(),
                    modified_at: metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|duration| duration.as_secs()),
                });
            }
        }
    }

    Ok(files)
}

#[derive(Debug, serde::Serialize)]
pub struct MediaFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: Option<u64>,
}

fn success_result(
    message: &str,
    target_path: Option<String>,
    file_size: Option<u64>,
) -> FileOperationResult {
    FileOperationResult {
        success: true,
        message: message.to_string(),
        target_path,
        file_size,
    }
}

fn fail_result(message: String) -> FileOperationResult {
    FileOperationResult {
        success: false,
        message,
        target_path: None,
        file_size: None,
    }
}

fn move_file_to_media_internal(
    db: &Database,
    source_path: &str,
    file_type: Option<&str>,
    conflict_strategy: Option<&ConflictStrategy>,
) -> Result<(String, u64), String> {
    let source_path = validate_source_file(source_path)?;
    let file_size = fs::metadata(source_path)
        .map_err(|error| format!("无法读取源文件: {}", error))?
        .len();
    let target_path = resolve_target_path(db, source_path, file_type, conflict_strategy)?;
    ensure_parent_dir(&target_path)?;

    if let Err(error) = fs::rename(source_path, &target_path) {
        if error.kind() != std::io::ErrorKind::CrossesDevices {
            return Err(format!("移动文件失败: {}", error));
        }

        copy_file_internal(source_path, &target_path)?;
        fs::remove_file(source_path)
            .map_err(|delete_error| format!("文件复制成功，但删除原文件失败: {}", delete_error))?;
    }

    Ok((target_path.to_string_lossy().to_string(), file_size))
}

fn copy_file_to_media_internal(
    db: &Database,
    source_path: &str,
    file_type: Option<&str>,
    conflict_strategy: Option<&ConflictStrategy>,
) -> Result<(String, u64), String> {
    let source_path = validate_source_file(source_path)?;
    let file_size = fs::metadata(source_path)
        .map_err(|error| format!("无法读取源文件: {}", error))?
        .len();
    let target_path = resolve_target_path(db, source_path, file_type, conflict_strategy)?;
    ensure_parent_dir(&target_path)?;
    copy_file_internal(source_path, &target_path)?;

    Ok((target_path.to_string_lossy().to_string(), file_size))
}

fn delete_media_file_internal(
    db: &Database,
    file_name: &str,
    file_type: Option<&str>,
) -> Result<(), String> {
    let file_path = db
        .media_root
        .join(get_media_subdir(file_type.unwrap_or("image")))
        .join(sanitize_file_name(file_name)?);

    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }

    fs::remove_file(&file_path).map_err(|error| format!("删除文件失败: {}", error))
}

fn validate_source_file(source_path: &str) -> Result<&Path, String> {
    let source_path = Path::new(source_path);
    if !source_path.exists() {
        return Err("源文件不存在".to_string());
    }
    if !source_path.is_file() {
        return Err("源路径不是文件".to_string());
    }
    Ok(source_path)
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("无法创建目标目录: {}", error))?;
    }
    Ok(())
}

fn copy_file_internal(source: &Path, target: &Path) -> Result<(), String> {
    let mut source_file =
        File::open(source).map_err(|error| format!("无法打开源文件: {}", error))?;
    let mut target_file =
        File::create(target).map_err(|error| format!("无法创建目标文件: {}", error))?;
    io::copy(&mut source_file, &mut target_file)
        .map_err(|error| format!("复制文件失败: {}", error))?;
    Ok(())
}

fn resolve_target_path(
    db: &Database,
    source_path: &Path,
    file_type: Option<&str>,
    conflict_strategy: Option<&ConflictStrategy>,
) -> Result<PathBuf, String> {
    let file_name = source_path
        .file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?
        .to_string_lossy()
        .to_string();
    let mut target_path = db
        .media_root
        .join(get_media_subdir(file_type.unwrap_or("image")))
        .join(sanitize_file_name(&file_name)?);

    if target_path.exists() {
        match conflict_strategy.unwrap_or(&ConflictStrategy::Rename) {
            ConflictStrategy::Overwrite => {}
            ConflictStrategy::Rename => target_path = generate_unique_path(&target_path),
            ConflictStrategy::Skip => return Err("目标文件已存在，跳过操作".to_string()),
            ConflictStrategy::Fail => return Err("目标文件已存在".to_string()),
        }
    }

    Ok(target_path)
}

fn generate_unique_path(base_path: &Path) -> PathBuf {
    let parent = base_path.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = base_path.extension().unwrap_or_default().to_string_lossy();

    for index in 1..1000 {
        let new_name = if extension.is_empty() {
            format!("{} ({})", file_stem, index)
        } else {
            format!("{} ({}).{}", file_stem, index, extension)
        };
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    base_path.to_path_buf()
}

fn sanitize_file_name(file_name: &str) -> Result<String, String> {
    Path::new(file_name)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "无效文件名".to_string())
}

fn get_media_subdir(file_type: &str) -> &str {
    match file_type.to_ascii_lowercase().as_str() {
        "video" => "视频",
        "audio" => "音频",
        "ppt" | "presentation" => "PPT",
        "text" | "markdown" | "json" => "文本",
        "archive" | "zip" => "压缩包",
        _ => "图片",
    }
}
