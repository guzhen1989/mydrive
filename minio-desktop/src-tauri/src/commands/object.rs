use tauri::State;
use crate::{AppState, error::{Result, AppError}, models::ObjectInfo};
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;



#[tauri::command]
pub async fn get_presigned_url(
    state: State<'_, AppState>,
    bucket: String,
    object_key: String,
    expires_in_seconds: Option<u64>,
) -> Result<String> {
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;
    
    let expires_in = expires_in_seconds.unwrap_or(3600); // 默认1小时
    
    let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
        .expires_in(std::time::Duration::from_secs(expires_in))
        .build()
        .map_err(|e| AppError::S3(e.to_string()))?;
    
    let presigned_request = client
        .get_client()
        .get_object()
        .bucket(&bucket)
        .key(&object_key)
        .presigned(presigning_config)
        .await
        .map_err(|e| {
            eprintln!("Presigning error: {:?}", e);
            AppError::S3(e.to_string())
        })?;
    
    Ok(presigned_request.uri().to_string())
}

#[tauri::command]
pub async fn list_objects(
    state: State<'_, AppState>,
    bucket: String,
    prefix: Option<String>,
) -> Result<Vec<ObjectInfo>> {
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref()
        .ok_or(AppError::NotConnected)?;
    
    client.list_objects(&bucket, prefix.as_deref()).await
}

#[tauri::command]
pub async fn upload_file(
    state: State<'_, AppState>,
    local_path: String,
    bucket: String,
    object_key: String,
) -> Result<String> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let bucket_clone = bucket.clone();
    let object_key_clone = object_key.clone();
    let local_path_clone = local_path.clone();
    
    let transfer_manager = state.transfer_manager.clone();
    
    // Start upload task in background
    tokio::spawn(async move {
        if let Err(e) = transfer_manager.lock().await.start_upload(task_id_clone, local_path_clone, bucket_clone, object_key_clone).await {
            eprintln!("Upload task failed: {}", e);
        }
    });
    
    Ok(task_id)
}

#[tauri::command]
pub async fn download_file(
    state: State<'_, AppState>,
    bucket: String,
    object_key: String,
    local_path: String,
) -> Result<String> {
    let task_id = uuid::Uuid::new_v4().to_string();
    
    // Clone necessary values to move into the async block
    let task_id_clone = task_id.clone();
    let bucket_clone = bucket.clone();
    let object_key_clone = object_key.clone();
    let local_path_clone = local_path.clone();
    
    // Clone the Arc to pass to the async block
    let transfer_manager = state.transfer_manager.clone();
    
    // Start download task in background
    tokio::spawn(async move {
        if let Err(e) = transfer_manager.lock().await.start_download(task_id_clone, bucket_clone, object_key_clone, local_path_clone).await {
            eprintln!("Download task failed: {}", e);
        }
    });
    
    Ok(task_id)
}

#[tauri::command]
pub async fn delete_object(
    state: State<'_, AppState>,
    bucket: String,
    key: String,
) -> Result<()> {
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref()
        .ok_or(AppError::NotConnected)?;
    
    client.delete_object(&bucket, &key).await
}

