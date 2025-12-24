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
    use_encryption: Option<bool>,
    encryption_key: Option<String>,
) -> Result<String> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let bucket_clone = bucket.clone();
    let object_key_clone = object_key.clone();
    let local_path_clone = local_path.clone();
    let use_encryption = use_encryption.unwrap_or(false);
    
    let transfer_manager = state.transfer_manager.clone();
    
    // Start upload task in background
    tokio::spawn(async move {
        if let Err(e) = transfer_manager.lock().await.start_upload(
            task_id_clone, 
            local_path_clone, 
            bucket_clone, 
            object_key_clone,
            use_encryption,
            encryption_key,
        ).await {
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

#[tauri::command]
pub async fn get_object_data(
    state: State<'_, AppState>,
    bucket: String,
    object_key: String,
) -> Result<Vec<u8>> {
    println!("[get_object_data] Starting for bucket: {}, key: {}", bucket, object_key);
    
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref()
        .ok_or(AppError::NotConnected)?;
    
    // Try to get encryption key from database
    let encryption_key = {
        let db = state.db.lock().await;
        db.get_encryption_key()?
    };
    
    if let Some(ref key) = encryption_key {
        println!("[get_object_data] Encryption key found, enabled: {}", key.enabled);
    } else {
        println!("[get_object_data] No encryption key found");
    }
    
    let mut get_request = client
        .get_client()
        .get_object()
        .bucket(&bucket)
        .key(&object_key);
    
    // Add SSE-C headers if encryption key exists
    if let Some(key) = encryption_key {
        if key.enabled {
            println!("[get_object_data] Adding SSE-C headers");
            use crate::encryption::SseCEncryption;
            let key_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &key.key_value)
                .map_err(|e| AppError::Encryption(e.to_string()))?;
            let sse_c = SseCEncryption::new(key_bytes)?;
            
            get_request = get_request
                .sse_customer_algorithm(sse_c.get_algorithm())
                .sse_customer_key(sse_c.get_key_base64())
                .sse_customer_key_md5(sse_c.get_key_md5());
        }
    }
    
    println!("[get_object_data] Sending request to S3");
    let result = get_request
        .send()
        .await
        .map_err(|e| {
            eprintln!("[get_object_data] S3 request failed: {:?}", e);
            AppError::S3(e.to_string())
        })?;
    
    println!("[get_object_data] Response received, collecting body");
    let data = result.body.collect().await
        .map_err(|e| {
            eprintln!("[get_object_data] Failed to collect body: {:?}", e);
            AppError::Other(e.to_string())
        })?;
    
    let bytes = data.into_bytes().to_vec();
    println!("[get_object_data] Data collected, size: {} bytes", bytes.len());
    println!("[get_object_data] First 20 bytes: {:?}", &bytes[..bytes.len().min(20)]);
    
    Ok(bytes)
}

