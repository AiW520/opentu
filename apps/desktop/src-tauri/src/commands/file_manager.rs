use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose, Engine as _};
use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::database::Database;

/// 文件操作结果
#[derive(Debug, serde::Serialize)]
pub struct FileOperationResult {
    pub success: bool,
    pub message: String,
    pub target_path: Option<String>,
    pub file_size: Option<u64>,
}

/// 文件冲突处理策略
#[derive(Debug, serde::Deserialize)]
pub enum ConflictStrategy {
    Overwrite,      // 覆盖
    Rename,         // 重命名（添加数字后缀）
    Skip,           // 跳过
    Fail,           // 失败
}

/// 移动文件到媒体目录
#[tauri::command]
pub async fn move_file_to_media(
    state: State<'_, AppState>,
    source_path: String,
    file_type: Option<String>,
    conflict_strategy: Option<ConflictStrategy>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    let result = move_file_to_media_internal(&db, &source_path, file_type.as_deref(), conflict_strategy.as_ref());
    match result {
        Ok((target_path, file_size)) => Ok(FileOperationResult {
            success: true,
            message: "文件移动成功".to_string(),
            target_path: Some(target_path),
            file_size: Some(file_size),
        }),
        Err(e) => Ok(FileOperationResult {
            success: false,
            message: e,
            target_path: None,
            file_size: None,
        }),
    }
}

/// 复制文件到媒体目录
#[tauri::command]
pub async fn copy_file_to_media(
    state: State<'_, AppState>,
    source_path: String,
    file_type: Option<String>,
    conflict_strategy: Option<ConflictStrategy>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    let result = copy_file_to_media_internal(&db, &source_path, file_type.as_deref(), conflict_strategy.as_ref());
    match result {
        Ok((target_path, file_size)) => Ok(FileOperationResult {
            success: true,
            message: "文件复制成功".to_string(),
            target_path: Some(target_path),
            file_size: Some(file_size),
        }),
        Err(e) => Ok(FileOperationResult {
            success: false,
            message: e,
            target_path: None,
            file_size: None,
        }),
    }
}

/// 删除媒体目录中的文件
#[tauri::command]
pub async fn delete_media_file(
    state: State<'_, AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    let result = delete_media_file_internal(&db, &file_name, file_type.as_deref());
    match result {
        Ok(_) => Ok(FileOperationResult {
            success: true,
            message: "文件删除成功".to_string(),
            target_path: None,
            file_size: None,
        }),
        Err(e) => Ok(FileOperationResult {
            success: false,
            message: e,
            target_path: None,
            file_size: None,
        }),
    }
}

/// 验证文件是否存在并可访问
#[tauri::command]
pub async fn verify_file_accessible(
    state: State<'_, AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<FileOperationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    let subdir = get_media_subdir(file_type.as_deref().unwrap_or("image"));
    let file_path = db.media_root.join(subdir).join(&file_name);
    
    match fs::metadata(&file_path) {
        Ok(metadata) => {
            if metadata.is_file() {
                Ok(FileOperationResult {
                    success: true,
                    message: "文件存在且可访问".to_string(),
                    target_path: Some(file_path.to_string_lossy().to_string()),
                    file_size: Some(metadata.len()),
                })
            } else {
                Ok(FileOperationResult {
                    success: false,
                    message: "路径不是文件".to_string(),
                    target_path: Some(file_path.to_string_lossy().to_string()),
                    file_size: None,
                })
            }
        }
        Err(e) => Ok(FileOperationResult {
            success: false,
            message: format!("文件不存在或无法访问: {}", e),
            target_path: Some(file_path.to_string_lossy().to_string()),
            file_size: None,
        }),
    }
}

/// 获取媒体目录中所有文件列表
#[tauri::command]
pub async fn list_media_files(
    state: State<'_, AppState>,
    file_type: Option<String>,
) -> Result<Vec<MediaFileInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    
    let subdir = get_media_subdir(file_type.as_deref().unwrap_or("image"));
    let media_dir = db.media_root.join(subdir);
    
    match fs::read_dir(&media_dir) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        files.push(MediaFileInfo {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: entry.path().to_string_lossy().to_string(),
                            size: metadata.len(),
                            modified_at: metadata.modified().ok().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
                        });
                    }
                }
            }
            Ok(files)
        }
        Err(e) => Err(format!("无法读取目录: {}", e)),
    }
}

/// 媒体文件信息
#[derive(Debug, serde::Serialize)]
pub struct MediaFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: Option<u64>,
}

// === 内部实现函数 ===

fn move_file_to_media_internal(
    db: &Database,
    source_path: &str,
    file_type: Option<&str>,
    conflict_strategy: Option<&ConflictStrategy>,
) -> Result<(String, u64), String> {
    let source_path = Path::new(source_path);
    
    // 验证源文件存在
    if !source_path.exists() {
        return Err("源文件不存在".to_string());
    }
    
    if !source_path.is_file() {
        return Err("源路径不是文件".to_string());
    }
    
    // 获取文件大小
    let metadata = fs::metadata(source_path).map_err(|e| format!("无法读取源文件: {}", e))?;
    let file_size = metadata.len();
    
    // 获取目标路径
    let (target_path, _) = resolve_target_path(db, source_path, file_type, conflict_strategy)?;
    
    // 确保目标目录存在
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建目标目录: {}", e))?;
    }
    
    // 执行移动操作
    fs::rename(source_path, &target_path).map_err(|e| {
        // 如果重命名失败（跨分区移动），尝试复制后删除
        if e.kind() == std::io::ErrorKind::CrossesDevices {
            if let Err(copy_err) = copy_file_internal(source_path, &target_path) {
                return format!("跨分区移动失败，复制也失败: {}", copy_err);
            }
            if let Err(delete_err) = fs::remove_file(source_path) {
                return format!("文件复制成功，但删除原文件失败: {}", delete_err);
            }
            return "".to_string();
        }
        format!("移动文件失败: {}", e)
    })?;
    
    Ok((target_path.to_string_lossy().to_string(), file_size))
}

fn copy_file_to_media_internal(
    db: &Database,
    source_path: &str,
    file_type: Option<&str>,
    conflict_strategy: Option<&ConflictStrategy>,
) -> Result<(String, u64), String> {
    let source_path = Path::new(source_path);
    
    // 验证源文件存在
    if !source_path.exists() {
        return Err("源文件不存在".to_string());
    }
    
    if !source_path.is_file() {
        return Err("源路径不是文件".to_string());
    }
    
    // 获取文件大小
    let metadata = fs::metadata(source_path).map_err(|e| format!("无法读取源文件: {}", e))?;
    let file_size = metadata.len();
    
    // 获取目标路径
    let (target_path, _) = resolve_target_path(db, source_path, file_type, conflict_strategy)?;
    
    // 确保目标目录存在
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建目标目录: {}", e))?;
    }
    
    // 执行复制操作
    copy_file_internal(source_path, &target_path)?;
    
    Ok((target_path.to_string_lossy().to_string(), file_size))
}

fn delete_media_file_internal(
    db: &Database,
    file_name: &str,
    file_type: Option<&str>,
) -> Result<(), String> {
    let subdir = get_media_subdir(file_type.unwrap_or("image"));
    let file_path = db.media_root.join(subdir).join(file_name);
    
    if !file_path.exists() {
        return Err("文件不存在".to_string());
    }
    
    fs::remove_file(&file_path).map_err(|e| format!("删除文件失败: {}", e))
}

fn copy_file_internal(source: &Path, target: &Path) -> Result<(), String> {
    let mut source_file = File::open(source).map_err(|e| format!("无法打开源文件: {}", e))?;
    let mut target_file = File::create(target).map_err(|e| format!("无法创建目标文件: {}", e))?;
    
    let mut buffer = Vec::new();
    source_file.read_to_end(&mut buffer).map_err(|e| format!("读取源文件失败: {}", e))?;
    target_file.write_all(&buffer).map_err(|e| format!("写入目标文件失败: {}", e))?;
    
    Ok(())
}

fn resolve_target_path(
    db: &Database,
    source_path: &Path,
    file_type: Option<&str>,
    conflict_strategy: Option<&ConflictStrategy>,
) -> Result<(PathBuf, bool), String> {
    let subdir = get_media_subdir(file_type.unwrap_or("image"));
    let file_name = source_path.file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?
        .to_string_lossy()
        .to_string();
    
    let mut target_path = db.media_root.join(subdir).join(&file_name);
    
    // 处理文件冲突
    if target_path.exists() {
        match conflict_strategy.unwrap_or(&ConflictStrategy::Rename) {
            ConflictStrategy::Overwrite => {
                // 直接覆盖，不需要修改路径
            }
            ConflictStrategy::Rename => {
                target_path = generate_unique_path(&target_path);
            }
            ConflictStrategy::Skip => {
                return Err("目标文件已存在，跳过操作".to_string());
            }
            ConflictStrategy::Fail => {
                return Err("目标文件已存在".to_string());
            }
        }
    }
    
    Ok((target_path, target_path.exists()))
}

fn generate_unique_path(base_path: &Path) -> PathBuf {
    let parent = base_path.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = base_path.extension().unwrap_or_default().to_string_lossy();
    
    for i in 1..1000 {
        let new_name = if extension.is_empty() {
            format!("{} ({})", file_stem, i)
        } else {
            format!("{} ({}).{}", file_stem, i, extension)
        };
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }
    
    // 如果找不到可用的文件名，返回原路径（应该不会走到这里）
    base_path.to_path_buf()
}

fn get_media_subdir(file_type: &str) -> &str {
    match file_type {
        "video" => "视频",
        "audio" => "音频",
        _ => "图片",
    }
}
