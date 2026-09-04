pub mod cache;
pub mod config;
pub mod grade;
pub mod model;
pub mod server;
pub mod sidecar;
pub mod sync;

use std::sync::Arc;

use crate::cache::CacheManager;
use crate::config::AppConfig;
use crate::server::start_local_server;

#[tauri::command]
fn get_app_status() -> serde_json::Value {
    serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "model": "Llama-3.2-3B-Instruct",
        "zero_telemetry": true,
        "local_api_port": 4343
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();
    let cache = Arc::new(
        CacheManager::open(&config.cache_db_path())
            .unwrap_or_else(|_| CacheManager::in_memory().expect("in-memory sqlite failed")),
    );

    // Start local extension HTTP server
    let _ = start_local_server(config.server_port, Arc::clone(&cache));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Setup system tray
            #[cfg(desktop)]
            {
                use tauri::tray::TrayIconBuilder;
                let _tray = TrayIconBuilder::new()
                    .tooltip("Crixata - AI Policy Grader")
                    .show_menu_on_left_click(true)
                    .build(app)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_app_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
