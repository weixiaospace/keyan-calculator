//! 算码工具 —— 库入口

mod commands;
mod hasher;
mod scanner;
mod store;
mod types;

use rusqlite::Connection;
use store::{init_db, AppState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // 桌面端启用自动更新插件
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let dir = app
                .path()
                .app_data_dir()
                .expect("无法获取应用数据目录");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("attestations.db");
            let conn = Connection::open(&db_path).expect("无法打开本地数据库");
            init_db(&conn).expect("初始化数据库失败");
            app.manage(AppState::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_folders,
            commands::add_folder,
            commands::remove_folder,
            commands::scan_folder,
            commands::compute_folder,
            commands::compute_file,
            commands::get_file_detail,
            commands::upload_pending,
            commands::get_config,
            commands::set_config,
        ])
        .run(tauri::generate_context!())
        .expect("运行算码工具时出错");
}
