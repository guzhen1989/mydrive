use tauri::State;
use crate::{AppState, error::Result};

#[tauri::command]
pub async fn get_stream_url(
    state: State<'_, AppState>,
    bucket: String,
    object_key: String,
) -> Result<String> {
    println!("Getting stream URL for: {}/{}", bucket, object_key);
    
    // Check if MinIO client is connected
    {
        let client_guard = state.minio_client.lock().await;
        if client_guard.is_none() {
            println!("ERROR: MinIO client not connected");
            return Err(crate::error::AppError::NotConnected);
        }
        println!("MinIO client is connected");
    }
    
    // Get or create stream server
    let mut server_guard = state.stream_server.lock().await;
    
    if server_guard.is_none() {
        println!("Creating new stream server on port 8080");
        let server = crate::streaming::StreamServer::new(
            8080,
            state.minio_client.clone()
        ).await;
        *server_guard = Some(server);
        println!("Stream server created successfully");
    } else {
        println!("Using existing stream server");
    }
    
    let server = server_guard.as_ref().unwrap();
    let token = server.create_token(bucket, object_key).await;
    let port = server.get_port();
    let url = format!("http://localhost:{}/stream/{}", port, token);
    
    println!("Generated stream URL: {}", url);
    
    Ok(url)
}
