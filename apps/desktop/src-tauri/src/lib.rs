mod cache_cleanup;
mod cas;
mod commands;
mod database;
mod path_grants;
mod thumbnail;

use database::Database;
use path_grants::PathGrantStore;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
    pub path_grants: Mutex<PathGrantStore>,
}

pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("opentu-asset", |ctx, request| {
            commands::media::handle_opentu_asset_protocol(ctx.app_handle(), request)
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let db = Database::new(app.handle())?;
            app.manage(AppState {
                db: Mutex::new(db),
                path_grants: Mutex::new(PathGrantStore::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::storage::get_local,
            commands::storage::set_local,
            commands::storage::remove_local,
            commands::storage::get_stats,
            commands::media::save_file,
            commands::media::get_file_path,
            commands::media::delete_file,
            commands::media::get_media_dir,
            commands::media::get_media_root_path,
            commands::media::set_media_root_path,
            commands::media::reset_media_root_path,
            commands::media::pick_media_folder,
            commands::media::pick_media_files,
            commands::media::pick_save_location,
            commands::media::write_file_to_path,
            commands::media::write_file_chunk_to_path,
            commands::media::download_url_to_path,
            commands::media::download_url_to_media_file,
            commands::media::copy_media_file_to_path,
            commands::media::get_default_save_path,
            commands::media::get_cached_media_file,
            commands::media::import_local_asset,
            commands::export::show_save_dialog,
            commands::file_manager::move_file_to_media,
            commands::file_manager::copy_file_to_media,
            commands::file_manager::delete_media_file,
            commands::file_manager::verify_file_accessible,
            commands::file_manager::list_media_files,
            commands::import_local_media_asset,
            commands::download_url_to_media_asset,
            commands::import_blob_to_media_asset,
            commands::get_media_cache_stats,
            commands::set_media_cache_max_size,
            commands::run_media_cache_cleanup,
            commands::get_asset_runtime_url,
            commands::get_thumbnail_runtime_url,
            commands::list_media_assets_paginated,
            commands::run_media_migration,
            commands::scan_legacy_media_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
