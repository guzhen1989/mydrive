use tauri::State;
use crate::{AppState, error::Result};

#[tauri::command]
pub async fn get_stream_url(
    state: State<'_, AppState>,
    bucket: String,
    object_key: String,
) -> Result<String> {
    eprintln!("[get_stream_url] Getting stream URL for: {}/{}", bucket, object_key);
    
    // Check if MinIO client is connected
    {
        let client_guard = state.minio_client.lock().await;
        if client_guard.is_none() {
            eprintln!("[get_stream_url] ERROR: MinIO client not connected");
            return Err(crate::error::AppError::NotConnected);
        }
        eprintln!("[get_stream_url] MinIO client is connected");
    }
    
    // Get or create stream server
    let mut server_guard = state.stream_server.lock().await;
    
    if server_guard.is_none() {
        eprintln!("[get_stream_url] Creating new stream server on port 8080");
        let server = crate::streaming::StreamServer::new(
            8080,
            state.minio_client.clone(),
            state.db.clone()
        ).await;
        *server_guard = Some(server);
        eprintln!("[get_stream_url] Stream server created successfully");
    } else {
        eprintln!("[get_stream_url] Using existing stream server");
    }
    
    let server = server_guard.as_ref().unwrap();
    let token = server.create_token(bucket, object_key).await;
    let port = server.get_port();
    let url = format!("http://localhost:{}/stream/{}", port, token);
    
    eprintln!("[get_stream_url] Generated stream URL: {}", url);
    
    Ok(url)
}

#[tauri::command]
pub async fn check_stream_server(
    state: State<'_, AppState>,
) -> Result<bool> {
    eprintln!("[check_stream_server] Checking stream server status");
    
    let server_guard = state.stream_server.lock().await;
    let is_running = server_guard.is_some();
    
    eprintln!("[check_stream_server] Stream server running: {}", is_running);
    
    if is_running {
        // Try to make a test request
        match reqwest::get("http://localhost:8080/stream/test").await {
            Ok(response) => {
                eprintln!("[check_stream_server] Test request status: {}", response.status());
                Ok(true)
            }
            Err(e) => {
                eprintln!("[check_stream_server] Test request failed: {:?}", e);
                Ok(false)
            }
        }
    } else {
        Ok(false)
    }
}
