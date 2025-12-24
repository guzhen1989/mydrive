use tauri::State;
use crate::{AppState, error::{Result, AppError}, models::BucketInfo};

#[tauri::command]
pub async fn list_buckets(state: State<'_, AppState>) -> Result<Vec<BucketInfo>> {
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref()
        .ok_or(AppError::NotConnected)?;
    
    client.list_buckets().await
}

#[tauri::command]
pub async fn create_bucket(state: State<'_, AppState>, name: String) -> Result<()> {
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref()
        .ok_or(AppError::NotConnected)?;
    
    client.create_bucket(&name).await
}

#[tauri::command]
pub async fn delete_bucket(state: State<'_, AppState>, name: String) -> Result<()> {
    let client_guard = state.minio_client.lock().await;
    let client = client_guard.as_ref()
        .ok_or(AppError::NotConnected)?;
    
    client.delete_bucket(&name).await
}
