// Allow console window to see logs in production
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod minio;
mod models;
mod streaming;
mod error;
mod transfer;
mod encryption;

use tauri::{Manager, State};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    db: Arc<Mutex<db::Database>>,
    minio_client: Arc<Mutex<Option<minio::MinioClient>>>,
    stream_server: Arc<Mutex<Option<streaming::StreamServer>>>,
    transfer_manager: Arc<Mutex<transfer::TransferManager>>,
}

fn main() {
    // 设置环境变量以允许自签名SSL证书
    std::env::set_var("RUST_LOG", "warn");
    
    // 对于自签名SSL证书，可能需要设置这个环境变量
    std::env::set_var("AWS_SDK_IMDS_DISABLED", "true");
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::connection::test_connection,
            commands::connection::save_connection,
            commands::connection::get_connection,
            commands::bucket::list_buckets,
            commands::bucket::create_bucket,
            commands::bucket::delete_bucket,
            commands::object::list_objects,

            commands::object::upload_file,
            commands::object::download_file,
            commands::object::delete_object,
            commands::object::get_presigned_url,
            commands::object::get_object_data,
            commands::transfer::get_transfer_tasks,
            commands::transfer::pause_task,
            commands::transfer::resume_task,
            commands::transfer::cancel_task,
            commands::transfer::cancel_all_tasks,
            commands::transfer::delete_task,
            commands::transfer::delete_completed_tasks,
            commands::streaming::get_stream_url,
            commands::streaming::check_stream_server,
            commands::encryption::generate_encryption_key,
            commands::encryption::save_encryption_key,
            commands::encryption::get_encryption_key,
            commands::encryption::set_encryption_enabled,
            commands::encryption::validate_encryption_key,
        ])
        .setup(|app| {
            // Initialize database
            let app_dir = app.path_resolver()
                .app_data_dir()
                .expect("Failed to get app data directory");
            
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("data.db");
            
            let db = db::Database::new(db_path.to_str().unwrap())?;
            
            // Setup app state
            let db_arc = Arc::new(Mutex::new(db));
            let minio_arc = Arc::new(Mutex::new(None));
            let transfer_manager = transfer::TransferManager::new(db_arc.clone(), minio_arc.clone());
            
            let state = AppState {
                db: db_arc,
                minio_client: minio_arc,
                stream_server: Arc::new(Mutex::new(None)),
                transfer_manager: Arc::new(Mutex::new(transfer_manager)),
            };
            
            app.manage(state);
            
            // Log application startup
            eprintln!("[App] MinIO Desktop application started");
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}