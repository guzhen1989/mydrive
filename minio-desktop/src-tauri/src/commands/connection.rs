use tauri::State;
use crate::{AppState, error::Result, models::{ConnectionConfig, BucketInfo}, minio::MinioClient};

#[tauri::command]
pub async fn test_connection(
    endpoint: String,
    port: u16,
    access_key: String,
    secret_key: String,
    use_ssl: bool,
) -> Result<Vec<BucketInfo>> {
    let config = ConnectionConfig {
        endpoint,
        port,
        access_key,
        secret_key,
        use_ssl,
        last_connected: None,
    };
    
    // 首先尝试创建客户端，验证基本连接
    let client = MinioClient::new(config).await?;
    
    // 然后尝试列出存储桶，验证权限
    let buckets = client.list_buckets().await;
    
    match buckets {
        Ok(bucket_list) => {
            println!("Successfully connected and retrieved {} buckets", bucket_list.len());
            Ok(bucket_list)
        }
        Err(e) => {
            eprintln!("Failed to list buckets: {:?}", e);
            // 检查错误类型，如果是认证错误，则返回错误
            let error_str = e.to_string().to_lowercase();
            return Err(crate::error::AppError::S3(e.to_string()));
        }
    }
}

#[tauri::command]
pub async fn save_connection(
    state: State<'_, AppState>,
    endpoint: String,
    port: u16,
    access_key: String,
    secret_key: String,
    use_ssl: bool,
) -> Result<()> {
    let config = ConnectionConfig {
        endpoint,
        port,
        access_key,
        secret_key,
        use_ssl,
        last_connected: Some(chrono::Utc::now()),
    };
    
    // Save to database
    let db = state.db.lock().await;
    db.save_connection(&config)?;
    drop(db);
    
    // Create and store MinIO client
    let client = MinioClient::new(config).await?;
    let mut minio_client = state.minio_client.lock().await;
    *minio_client = Some(client);
    
    Ok(())
}

#[tauri::command]
pub async fn get_connection(state: State<'_, AppState>) -> Result<Option<ConnectionConfig>> {
    let db = state.db.lock().await;
    db.get_connection()
}
