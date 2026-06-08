use crate::AppState;
use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

#[tauri::command]
pub fn save_file(
    state: State<AppState>,
    file_name: String,
    buffer: Vec<u8>,
    file_type: Option<String>,
) -> Result<String, String> {
    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };
    let media_dir = media_root.join(get_media_subdir(file_type.as_deref().unwrap_or("image")));
    fs::create_dir_all(&media_dir).map_err(|e| e.to_string())?;

    let file_path = media_dir.join(sanitize_file_name(&file_name)?);
    fs::write(&file_path, &buffer).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_file_path(
    state: State<AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<Option<String>, String> {
    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };
    let file_path = resolve_media_file_path(&media_root, &file_name, file_type.as_deref())?;

    if file_path.exists() {
        Ok(Some(file_path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn delete_file(
    state: State<AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<(), String> {
    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };
    let file_path = resolve_media_file_path(&media_root, &file_name, file_type.as_deref())?;

    if file_path.exists() {
        fs::remove_file(&file_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_media_dir(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db.media_root.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_media_root_path(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db.media_root.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_media_root_path(state: State<AppState>, path: String) -> Result<String, String> {
    let new_path = PathBuf::from(&path);
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_media_root(new_path).map_err(|e| e.to_string())?;
    Ok(db.media_root.to_string_lossy().to_string())
}

#[tauri::command]
pub fn reset_media_root_path(state: State<AppState>) -> Result<String, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.reset_media_root().map_err(|e| e.to_string())?;
    Ok(db.media_root.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn pick_media_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder_path = app.dialog().file().blocking_pick_folder();

    Ok(folder_path.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn pick_media_files(app: tauri::AppHandle) -> Result<Vec<PickedMediaFile>, String> {
    use tauri_plugin_dialog::DialogExt;

    let files = app
        .dialog()
        .file()
        .add_filter(
            "媒体素材",
            &[
                "png", "jpg", "jpeg", "gif", "webp", "svg", "mp4", "webm", "mov", "m4v", "ogg",
                "mp3", "wav", "m4a", "aac", "flac", "zip",
            ],
        )
        .blocking_pick_files();

    let Some(files) = files else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();
    for file_path in files {
        let path = file_path
            .into_path()
            .map_err(|e| format!("无法解析本地素材路径: {}", e))?;
        if !path.is_file() {
            continue;
        }
        let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "asset".to_string());
        let mime_type = detect_mime_type(&path);
        let file_type = infer_file_type(&mime_type, &path);
        result.push(PickedMediaFile {
            path: path.to_string_lossy().to_string(),
            name: file_name,
            mime_type,
            file_type,
            size: metadata.len(),
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn pick_save_location(
    app: tauri::AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file_path = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .blocking_save_file();

    Ok(file_path.map(|p| p.to_string()))
}

#[tauri::command]
pub fn write_file_to_path(save_path: String, buffer: Vec<u8>) -> Result<(), String> {
    let path = PathBuf::from(save_path);
    validate_save_path(&path)?;
    fs::write(&path, buffer).map_err(|e| format!("无法写入文件: {}", e))
}

#[tauri::command]
pub async fn download_url_to_path(url: String, save_path: String) -> Result<(), String> {
    let path = PathBuf::from(save_path);
    validate_save_path(&path)?;
    let parsed_url = reqwest::Url::parse(&url).map_err(|e| format!("无效下载地址: {}", e))?;
    if !matches!(parsed_url.scheme(), "http" | "https") {
        return Err("仅支持下载 HTTP/HTTPS 资源".to_string());
    }

    let mut response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("无法初始化下载器: {}", e))?
        .get(parsed_url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Opentu Desktop",
        )
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status()));
    }

    let mut file = File::create(&path).map_err(|e| format!("无法写入文件: {}", e))?;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("读取下载内容失败: {}", e))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("写入下载内容失败: {}", e))?;
    }
    file.flush().map_err(|e| format!("保存文件失败: {}", e))
}

#[tauri::command]
pub fn get_default_save_path(
    state: State<AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<String, String> {
    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };
    let media_dir = media_root.join(get_media_subdir(file_type.as_deref().unwrap_or("image")));
    fs::create_dir_all(&media_dir).map_err(|e| e.to_string())?;

    Ok(media_dir
        .join(sanitize_file_name(&file_name)?)
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn get_cached_media_file(
    state: State<AppState>,
    file_name: String,
    file_type: Option<String>,
) -> Result<String, String> {
    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };
    let file_path = resolve_media_file_path(&media_root, &file_name, file_type.as_deref())?;

    if file_path.exists() {
        let data = fs::read(&file_path).map_err(|e| e.to_string())?;
        Ok(general_purpose::STANDARD.encode(&data))
    } else {
        Err(format!(
            "File does not exist: {}",
            file_path.to_string_lossy()
        ))
    }
}

#[tauri::command]
pub fn import_local_asset(
    state: State<AppState>,
    source_path: String,
    file_type: Option<String>,
    original_name: Option<String>,
    mime_type: Option<String>,
) -> Result<AssetImportResult, String> {
    let source = validate_source_file(&source_path)?;
    let metadata = fs::metadata(source).map_err(|e| format!("无法读取源文件: {}", e))?;
    let source_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or_else(|| "无法获取源文件名".to_string())?;
    let original_name = sanitize_file_name(original_name.as_deref().unwrap_or(&source_name))?;
    let detected_mime_type = mime_type.unwrap_or_else(|| detect_mime_type(source));
    let inferred_file_type = infer_file_type(&detected_mime_type, source);
    let normalized_file_type = normalize_media_type(
        file_type
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(inferred_file_type.as_str()),
    );

    let media_root = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.media_root.clone()
    };
    let media_dir = media_root.join(get_media_subdir(&normalized_file_type));
    fs::create_dir_all(&media_dir).map_err(|e| e.to_string())?;

    let extension = resolve_extension(
        source,
        &original_name,
        &detected_mime_type,
        &normalized_file_type,
    );
    let temp_name = format!(
        ".opentu-import-{}-{}.tmp",
        chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
        std::process::id()
    );
    let temp_path = media_dir.join(temp_name);
    let content_hash = match copy_and_hash(source, &temp_path) {
        Ok(hash) => hash,
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    let file_name = format!("content-{}.{}", content_hash, extension);
    let final_path = media_dir.join(sanitize_file_name(&file_name)?);

    if final_path.exists() {
        let _ = fs::remove_file(&temp_path);
    } else if let Err(error) = fs::rename(&temp_path, &final_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("保存素材文件失败: {}", error));
    }

    Ok(AssetImportResult {
        content_hash,
        file_name,
        original_name,
        local_path: final_path.to_string_lossy().to_string(),
        file_type: normalized_file_type,
        mime_type: detected_mime_type,
        size: metadata.len(),
        created_at: chrono::Utc::now().timestamp_millis(),
    })
}

pub fn handle_opentu_asset_protocol(
    app_handle: &tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Cow<'static, [u8]>> {
    #[cfg(debug_assertions)]
    eprintln!("[opentu-asset] request: {}", request.uri());
    match serve_opentu_asset(app_handle, request) {
        Ok(response) => response,
        Err((status, message)) => {
            #[cfg(debug_assertions)]
            eprintln!("[opentu-asset] error {}: {}", status, message);
            text_response(status, &message)
        }
    }
}

fn sanitize_file_name(file_name: &str) -> Result<String, String> {
    Path::new(file_name)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "Invalid file name".to_string())
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

fn validate_save_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("保存路径不能为空".to_string());
    }
    if path.exists() && path.is_dir() {
        return Err("保存路径不能是文件夹".to_string());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("无法创建保存目录: {}", e))?;
        }
    }
    Ok(())
}

fn copy_and_hash(source: &Path, target: &Path) -> Result<String, String> {
    let mut source_file = File::open(source).map_err(|e| format!("无法打开源文件: {}", e))?;
    let mut target_file = File::create(target).map_err(|e| format!("无法创建目标文件: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];

    loop {
        let bytes_read = source_file
            .read(&mut buffer)
            .map_err(|e| format!("读取源文件失败: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        target_file
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("写入目标文件失败: {}", e))?;
    }

    target_file
        .flush()
        .map_err(|e| format!("刷新目标文件失败: {}", e))?;
    Ok(bytes_to_hex(&hasher.finalize()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{:02x}", byte));
    }
    output
}

fn resolve_extension(
    source: &Path,
    original_name: &str,
    mime_type: &str,
    file_type: &str,
) -> String {
    Path::new(original_name)
        .extension()
        .or_else(|| source.extension())
        .map(|value| {
            value
                .to_string_lossy()
                .trim_start_matches('.')
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| extension_from_mime(mime_type, file_type).to_string())
}

fn extension_from_mime(mime_type: &str, file_type: &str) -> &'static str {
    match mime_type.to_ascii_lowercase().as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        "video/x-m4v" => "m4v",
        "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/mp4" => "m4a",
        "audio/aac" => "aac",
        "audio/flac" => "flac",
        "application/zip" => "zip",
        _ if file_type == "video" => "mp4",
        _ if file_type == "audio" => "mp3",
        _ => "png",
    }
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

fn infer_file_type(mime_type: &str, path: &Path) -> String {
    if mime_type.starts_with("video/") {
        return "video".to_string();
    }
    if mime_type.starts_with("audio/") {
        return "audio".to_string();
    }
    if mime_type == "application/zip" {
        return "archive".to_string();
    }

    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(extension.as_str(), "mp4" | "webm" | "mov" | "m4v") {
        return "video".to_string();
    }
    if matches!(
        extension.as_str(),
        "mp3" | "wav" | "ogg" | "m4a" | "aac" | "flac"
    ) {
        return "audio".to_string();
    }
    "image".to_string()
}

fn normalize_media_type(file_type: &str) -> String {
    match file_type.to_ascii_lowercase().as_str() {
        "video" => "video",
        "audio" => "audio",
        "ppt" | "presentation" => "ppt",
        "text" | "markdown" | "json" => "text",
        "archive" | "zip" => "archive",
        _ => "image",
    }
    .to_string()
}

fn resolve_media_file_path(
    media_root: &Path,
    file_name: &str,
    file_type: Option<&str>,
) -> Result<PathBuf, String> {
    let safe_file_name = sanitize_file_name(file_name)?;

    if let Some(file_type) = file_type {
        let typed_path = media_root
            .join(get_media_subdir(file_type))
            .join(&safe_file_name);
        if typed_path.exists() {
            return Ok(typed_path);
        }
    }

    for subdir in get_all_media_subdirs() {
        let candidate = media_root.join(subdir).join(&safe_file_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(media_root
        .join(get_media_subdir(file_type.unwrap_or("image")))
        .join(safe_file_name))
}

fn serve_opentu_asset(
    app_handle: &tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
) -> Result<tauri::http::Response<Cow<'static, [u8]>>, (tauri::http::StatusCode, String)> {
    if request.method() == tauri::http::Method::OPTIONS {
        return asset_response_builder()
            .status(tauri::http::StatusCode::NO_CONTENT)
            .body(Vec::new().into())
            .map_err(|e| {
                (
                    tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                    e.to_string(),
                )
            });
    }

    if request.method() != tauri::http::Method::GET && request.method() != tauri::http::Method::HEAD
    {
        return Err((
            tauri::http::StatusCode::METHOD_NOT_ALLOWED,
            "Unsupported asset request method".to_string(),
        ));
    }

    let media_root = {
        let state = app_handle.state::<AppState>();
        let db = state.db.lock().map_err(|e| {
            (
                tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?;
        db.media_root.clone()
    };
    let media_root = media_root.canonicalize().map_err(|e| {
        (
            tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("媒体目录不可访问: {}", e),
        )
    })?;
    let requested_path = resolve_asset_request_path(request.uri(), &media_root)?;

    let mut file = File::open(&requested_path).map_err(|e| {
        (
            tauri::http::StatusCode::NOT_FOUND,
            format!("素材文件不可读取: {}", e),
        )
    })?;
    let file_size = file
        .metadata()
        .map_err(|e| {
            (
                tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?
        .len();
    let mime_type = detect_mime_type(&requested_path);

    if request.method() == tauri::http::Method::HEAD {
        return asset_response_builder()
            .status(tauri::http::StatusCode::OK)
            .header(tauri::http::header::CONTENT_TYPE, mime_type)
            .header(tauri::http::header::ACCEPT_RANGES, "bytes")
            .header(tauri::http::header::CONTENT_LENGTH, file_size.to_string())
            .body(Vec::new().into())
            .map_err(|e| {
                (
                    tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                    e.to_string(),
                )
            });
    }

    if let Some(range_header) = request
        .headers()
        .get(tauri::http::header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        let (start, end) = parse_single_range(range_header, file_size)?;
        let len = end - start + 1;
        let mut buffer = vec![0_u8; len as usize];
        file.seek(SeekFrom::Start(start)).map_err(|e| {
            (
                tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?;
        file.read_exact(&mut buffer).map_err(|e| {
            (
                tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?;

        return asset_response_builder()
            .status(tauri::http::StatusCode::PARTIAL_CONTENT)
            .header(tauri::http::header::CONTENT_TYPE, mime_type)
            .header(tauri::http::header::ACCEPT_RANGES, "bytes")
            .header(
                tauri::http::header::CONTENT_RANGE,
                format!("bytes {}-{}/{}", start, end, file_size),
            )
            .header(tauri::http::header::CONTENT_LENGTH, len.to_string())
            .body(buffer.into())
            .map_err(|e| {
                (
                    tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                    e.to_string(),
                )
            });
    }

    if file_size > MAX_FULL_RESPONSE_BYTES {
        return Err((
            tauri::http::StatusCode::PAYLOAD_TOO_LARGE,
            "素材文件过大，请使用范围请求读取".to_string(),
        ));
    }

    let mut buffer = Vec::with_capacity(file_size as usize);
    file.read_to_end(&mut buffer).map_err(|e| {
        (
            tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;

    asset_response_builder()
        .status(tauri::http::StatusCode::OK)
        .header(tauri::http::header::CONTENT_TYPE, mime_type)
        .header(tauri::http::header::ACCEPT_RANGES, "bytes")
        .header(tauri::http::header::CONTENT_LENGTH, file_size.to_string())
        .body(buffer.into())
        .map_err(|e| {
            (
                tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })
}

fn resolve_asset_request_path(
    uri: &tauri::http::Uri,
    media_root: &Path,
) -> Result<PathBuf, (tauri::http::StatusCode, String)> {
    let candidates = decode_asset_request_path_candidates(uri)?;
    let mut not_found_message = "素材文件不存在".to_string();
    let mut saw_forbidden_path = false;

    for candidate in candidates {
        let requested_path = PathBuf::from(candidate);
        let requested_path = match requested_path.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                not_found_message = format!("素材文件不存在: {}", error);
                continue;
            }
        };

        if !requested_path.starts_with(media_root) || !requested_path.is_file() {
            saw_forbidden_path = true;
            continue;
        }

        return Ok(requested_path);
    }

    if saw_forbidden_path {
        return Err((
            tauri::http::StatusCode::FORBIDDEN,
            "不允许访问该素材路径".to_string(),
        ));
    }

    Err((tauri::http::StatusCode::NOT_FOUND, not_found_message))
}

fn decode_asset_request_path_candidates(
    uri: &tauri::http::Uri,
) -> Result<Vec<String>, (tauri::http::StatusCode, String)> {
    let path = uri.path();
    let encoded_path = path.trim_start_matches('/');
    if encoded_path.is_empty() {
        return Err((
            tauri::http::StatusCode::BAD_REQUEST,
            "素材路径为空".to_string(),
        ));
    }

    let mut encoded_candidates = vec![encoded_path.to_string()];
    if let Some((first_segment, rest)) = encoded_path.split_once('/') {
        if is_asset_protocol_host(first_segment) && !rest.is_empty() {
            encoded_candidates.push(rest.to_string());
        }
    }

    if let Some(authority) = uri.authority() {
        let authority = authority.as_str();
        if !is_asset_protocol_host(authority) && !authority.is_empty() {
            encoded_candidates.push(format!("{authority}{path}"));
        }
    }

    let mut decoded_candidates = Vec::new();
    for encoded_candidate in encoded_candidates {
        let decoded = percent_decode(&encoded_candidate).map_err(|e| {
            (
                tauri::http::StatusCode::BAD_REQUEST,
                format!("素材路径无效: {}", e),
            )
        })?;
        let decoded = normalize_decoded_asset_path(decoded);
        if !decoded.is_empty() && !decoded_candidates.iter().any(|item| item == &decoded) {
            decoded_candidates.push(decoded);
        }
    }

    Ok(decoded_candidates)
}

fn is_asset_protocol_host(value: &str) -> bool {
    let host = value
        .rsplit('@')
        .next()
        .unwrap_or(value)
        .split(':')
        .next()
        .unwrap_or(value)
        .to_ascii_lowercase();

    matches!(host.as_str(), "localhost" | "opentu-asset.localhost")
}

fn normalize_decoded_asset_path(value: String) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 3
        && matches!(bytes[0], b'/' | b'\\')
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
    {
        value[1..].to_string()
    } else {
        value
    }
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("不完整的百分号编码".to_string());
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| "百分号编码不是 UTF-8".to_string())?;
            let byte =
                u8::from_str_radix(hex, 16).map_err(|_| "百分号编码不是十六进制".to_string())?;
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).map_err(|_| "路径不是有效 UTF-8".to_string())
}

fn parse_single_range(
    range_header: &str,
    file_size: u64,
) -> Result<(u64, u64), (tauri::http::StatusCode, String)> {
    if file_size == 0 {
        return Err((
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
            "空文件不支持范围读取".to_string(),
        ));
    }
    let range = range_header.strip_prefix("bytes=").ok_or_else(|| {
        (
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
            "Range 头无效".to_string(),
        )
    })?;
    let (start_text, end_text) = range.split_once('-').ok_or_else(|| {
        (
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
            "Range 头无效".to_string(),
        )
    })?;

    let (start, mut end) = if start_text.is_empty() {
        let suffix = end_text.parse::<u64>().map_err(|_| {
            (
                tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
                "Range 后缀无效".to_string(),
            )
        })?;
        let suffix = suffix.min(file_size);
        (file_size - suffix, file_size - 1)
    } else {
        let start = start_text.parse::<u64>().map_err(|_| {
            (
                tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
                "Range 起点无效".to_string(),
            )
        })?;
        let end = if end_text.is_empty() {
            start.saturating_add(MAX_RANGE_BYTES - 1).min(file_size - 1)
        } else {
            end_text.parse::<u64>().map_err(|_| {
                (
                    tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
                    "Range 终点无效".to_string(),
                )
            })?
        };
        (start, end)
    };

    if start >= file_size {
        return Err((
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
            "Range 超出文件大小".to_string(),
        ));
    }
    end = end.min(file_size - 1);
    end = end.min(start.saturating_add(MAX_RANGE_BYTES - 1));
    if end < start {
        return Err((
            tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
            "Range 区间无效".to_string(),
        ));
    }

    Ok((start, end))
}

fn text_response(
    status: tauri::http::StatusCode,
    message: &str,
) -> tauri::http::Response<Cow<'static, [u8]>> {
    asset_response_builder()
        .status(status)
        .header(
            tauri::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )
        .body(message.as_bytes().to_vec().into())
        .unwrap_or_else(|_| tauri::http::Response::new(Vec::new().into()))
}

fn asset_response_builder() -> tauri::http::response::Builder {
    tauri::http::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
        .header("Access-Control-Allow-Headers", "Range, Content-Type")
        .header(
            "Access-Control-Expose-Headers",
            "Content-Length, Content-Range, Accept-Ranges",
        )
        .header("Cache-Control", "no-store")
}

fn get_all_media_subdirs() -> [&'static str; 6] {
    ["图片", "视频", "音频", "PPT", "文本", "压缩包"]
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

const COPY_BUFFER_BYTES: usize = 128 * 1024;
const MAX_RANGE_BYTES: u64 = 1024 * 1024;
const MAX_FULL_RESPONSE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedMediaFile {
    pub path: String,
    pub name: String,
    pub mime_type: String,
    pub file_type: String,
    pub size: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportResult {
    pub content_hash: String,
    pub file_name: String,
    pub original_name: String,
    pub local_path: String,
    pub file_type: String,
    pub mime_type: String,
    pub size: u64,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_tauri_windows_http_asset_url() {
        let uri = "http://opentu-asset.localhost/D%3A%5COpenTu%5C%E5%9B%BE%E7%89%87%5Cdemo.png"
            .parse::<tauri::http::Uri>()
            .unwrap();

        let candidates = decode_asset_request_path_candidates(&uri).unwrap();

        assert!(candidates.contains(&"D:\\OpenTu\\图片\\demo.png".to_string()));
    }

    #[test]
    fn decodes_custom_protocol_localhost_url() {
        let uri = "opentu-asset://localhost/D%3A%5COpenTu%5C%E5%9B%BE%E7%89%87%5Cdemo.png"
            .parse::<tauri::http::Uri>()
            .unwrap();

        let candidates = decode_asset_request_path_candidates(&uri).unwrap();

        assert!(candidates.contains(&"D:\\OpenTu\\图片\\demo.png".to_string()));
    }

    #[test]
    fn normalizes_leading_slash_before_windows_drive() {
        assert_eq!(
            normalize_decoded_asset_path("/D:\\OpenTu\\图片\\demo.png".to_string()),
            "D:\\OpenTu\\图片\\demo.png"
        );
    }
}
