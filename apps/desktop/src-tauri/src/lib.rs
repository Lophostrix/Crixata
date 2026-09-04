pub mod cache;
pub mod config;
pub mod grade;
pub mod llm;
pub mod model;
pub mod server;
pub mod sidecar;
pub mod sync;

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::Mutex as AsyncMutex;

use crate::cache::CacheManager;
use crate::config::AppConfig;
use crate::model::ModelManager;
use crate::server::start_local_server;
use crate::sidecar::{SidecarManager, SidecarStatus};
use crate::sync::{CacheSync, SyncReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub percentage: f32,
    pub status: String,
    pub message: String,
}

#[tauri::command]
fn get_app_status(
    config: tauri::State<'_, AppConfig>,
    cache: tauri::State<'_, Arc<CacheManager>>,
    sidecar: tauri::State<'_, Arc<AsyncMutex<SidecarManager>>>,
    model_manager: tauri::State<'_, Arc<ModelManager>>,
) -> serde_json::Value {
    let (sidecar_status, sidecar_ready) = match sidecar.try_lock() {
        Ok(sc) => (sc.status(), sc.is_ready()),
        Err(_) => (SidecarStatus::Starting, false),
    };
    let model_present = model_manager.is_model_present();
    let sidecar_present = model_manager.is_sidecar_present();
    let cached_count = cache.count_grades().unwrap_or(0);
    let last_sync_time = cache.get_last_sync_time().ok().flatten().map(|dt| dt.to_rfc3339());

    serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "model": config.model_filename,
        "model_present": model_present,
        "sidecar_present": sidecar_present,
        "sidecar_status": format!("{:?}", sidecar_status),
        "sidecar_ready": sidecar_ready,
        "local_api_port": config.server_port,
        "llama_server_port": config.llama_server_port,
        "cached_policies_count": cached_count,
        "cdn_shard_base_url": config.cdn_shard_base_url,
        "last_sync_time": last_sync_time,
        "zero_telemetry": true
    })
}

#[tauri::command]
fn get_download_status(
    model_manager: tauri::State<'_, Arc<ModelManager>>,
) -> serde_json::Value {
    serde_json::to_value(model_manager.get_status()).unwrap_or_default()
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    model_manager: tauri::State<'_, Arc<ModelManager>>,
    sidecar: tauri::State<'_, Arc<AsyncMutex<SidecarManager>>>,
    config: tauri::State<'_, AppConfig>,
) -> Result<(), String> {
    let app_handle = app.clone();
    let mm = Arc::clone(&model_manager);
    let sc = Arc::clone(&sidecar);
    let cfg = config.inner().clone();

    tokio::spawn(async move {
        let app_progress = app_handle.clone();
        let res = mm
            .download_all(move |pct, msg| {
                let _ = app_progress.emit(
                    "model-download-progress",
                    ProgressPayload {
                        percentage: pct,
                        status: if pct >= 100.0 {
                            "completed".into()
                        } else {
                            "downloading".into()
                        },
                        message: msg.to_string(),
                    },
                );
            })
            .await;

        match res {
            Ok(_) => {
                let _ = app_handle.emit(
                    "model-download-progress",
                    ProgressPayload {
                        percentage: 100.0,
                        status: "completed".into(),
                        message: "Download complete. Starting inference engine...".into(),
                    },
                );

                // Update sidecar paths in case they were just downloaded
                {
                    if let Ok(mut sidecar_lock) = sc.try_lock() {
                        sidecar_lock.update_paths(
                            cfg.resolve_llama_server_bin(),
                            cfg.model_path(),
                        );
                    }
                }

                // Attempt to start sidecar
                let sc_start = Arc::clone(&sc);
                tokio::spawn(async move {
                    let mut lock = sc_start.lock().await;
                    let _ = lock.start().await;
                });
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "model-download-progress",
                    ProgressPayload {
                        percentage: 0.0,
                        status: "error".into(),
                        message: format!("Download failed: {}", e),
                    },
                );
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn trigger_cache_sync(
    sync: tauri::State<'_, Arc<CacheSync>>,
) -> Result<SyncReport, String> {
    sync.sync_all()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_last_sync_status(
    cache: tauri::State<'_, Arc<CacheManager>>,
) -> serde_json::Value {
    let last_time = cache.get_last_sync_time().ok().flatten().map(|dt| dt.to_rfc3339());
    let last_report_raw = cache.get_metadata("last_sync_report").ok().flatten();
    let report: Option<SyncReport> = last_report_raw.and_then(|r| serde_json::from_str(&r).ok());

    if let Some(rep) = report {
        serde_json::json!({
            "last_sync_time": rep.last_sync_time.or(last_time),
            "shards_fetched": rep.shards_fetched,
            "entries_added": rep.entries_added,
            "entries_updated": rep.entries_updated,
            "errors": rep.errors,
            "error_count": rep.errors.len(),
            "has_synced": true
        })
    } else {
        serde_json::json!({
            "last_sync_time": last_time,
            "shards_fetched": 0,
            "entries_added": 0,
            "entries_updated": 0,
            "errors": Vec::<String>::new(),
            "error_count": 0,
            "has_synced": last_time.is_some()
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();
    let cache = Arc::new(
        CacheManager::open(&config.cache_db_path())
            .unwrap_or_else(|_| CacheManager::in_memory().expect("in-memory sqlite failed")),
    );

    let cache_sync = Arc::new(CacheSync::new(&config, Arc::clone(&cache)));
    let model_manager = Arc::new(ModelManager::new(config.clone()));

    let sidecar = Arc::new(AsyncMutex::new(SidecarManager::new(
        config.resolve_llama_server_bin(),
        config.model_path(),
        config.llama_server_port,
    )));

    // Start local extension HTTP server
    let _ = start_local_server(
        config.server_port,
        config.llama_server_port,
        Arc::clone(&cache),
        Arc::clone(&sidecar),
        Arc::clone(&model_manager),
    );

    // Auto-start sidecar only if model and sidecar binary are already present
    if model_manager.is_model_present() && model_manager.is_sidecar_present() {
        let sc_init = Arc::clone(&sidecar);
        tauri::async_runtime::spawn(async move {
            let mut lock = sc_init.lock().await;
            let _ = lock.start().await;
        });
    }

    // Spawn non-blocking background tokio task running cache sync every 24 hours
    let sync_scheduler = Arc::clone(&cache_sync);
    let sync_interval_hours = config.sync_interval_hours;
    tauri::async_runtime::spawn(async move {
        // Startup grace period: 5s delay so startup is never blocked
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let interval = std::time::Duration::from_secs(sync_interval_hours.max(1) * 3600);
        loop {
            if sync_scheduler.is_reachable().await {
                match sync_scheduler.sync_all().await {
                    Ok(report) => {
                        println!(
                            "Background cache sync finished: {} shards, {} added, {} updated, {} errors",
                            report.shards_fetched, report.entries_added, report.entries_updated, report.errors.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("Background cache sync error: {}", e);
                    }
                }
            } else {
                println!("Cache CDN is unreachable; skipping scheduled sync.");
            }

            tokio::time::sleep(interval).await;
        }
    });

    let sidecar_for_exit = Arc::clone(&sidecar);

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .manage(config)
        .manage(cache)
        .manage(cache_sync)
        .manage(sidecar)
        .manage(model_manager)
        .setup(|app| {
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
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_download_status,
            download_model,
            trigger_cache_sync,
            get_last_sync_status,
        ]);

    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                if let Ok(mut sc) = sidecar_for_exit.try_lock() {
                    let _ = sc.stop();
                } else {
                    let mut sc = sidecar_for_exit.blocking_lock();
                    let _ = sc.stop();
                }
            }
        });
}
