mod database;
mod commands;

use database::Database;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let db = Database::new(app.handle())?;
            app.manage(AppState {
                db: Mutex::new(db),
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
            commands::media::pick_save_location,
            commands::media::get_cached_media_file,
            commands::export::show_save_dialog,
            commands::file_manager::move_file_to_media,
            commands::file_manager::copy_file_to_media,
            commands::file_manager::delete_media_file,
            commands::file_manager::verify_file_accessible,
            commands::file_manager::list_media_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}