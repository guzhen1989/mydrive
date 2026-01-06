// Streaming server for video playback
// This module implements an HTTP server that proxies MinIO streams

use axum::{
    Router,
    routing::get,
    extract::{Path, State as AxumState},
    response::{IntoResponse, Response},
    http::{StatusCode, HeaderMap, header},
    body,
};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;
use crate::models::StreamToken;
use crate::minio::MinioClient;
use crate::db::Database;

pub struct StreamServer {
    port: u16,
    tokens: Arc<Mutex<HashMap<String, StreamToken>>>,
    minio_client: Arc<Mutex<Option<MinioClient>>>,
    db: Arc<Mutex<Database>>,
}

impl StreamServer {
    pub async fn new(port: u16, minio_client: Arc<Mutex<Option<MinioClient>>>, db: Arc<Mutex<Database>>) -> Self {
        let tokens = Arc::new(Mutex::new(HashMap::new()));
        
        let server = StreamServer {
            port,
            tokens: tokens.clone(),
            minio_client: minio_client.clone(),
            db: db.clone(),
        };
        
        eprintln!("[StreamServer] Starting stream server on port {}", port);
        
        // Start HTTP server in background
        let app_state = AppState {
            tokens,
            minio_client,
            db,
        };
        
        tokio::spawn(async move {
            let app = Router::new()
                .route("/stream/:token", get(stream_handler))
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any)
                )
                .with_state(app_state);
            
            // Try multiple times to bind the server
            let mut attempts = 0;
            let max_attempts = 5;
            
            loop {
                attempts += 1;
                eprintln!("[StreamServer] Attempt {} to bind to 127.0.0.1:{}", attempts, port);
                
                match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                    Ok(listener) => {
                        eprintln!("[StreamServer] ✓ Stream server successfully bound to http://127.0.0.1:{}", port);
                        // Run the server and handle any errors
                        if let Err(e) = axum::serve(listener, app).await {
                            eprintln!("[StreamServer] ERROR in serve: {:?}", e);
                        }
                        return;
                    }
                    Err(e) => {
                        eprintln!("[StreamServer] ERROR: Failed to bind to port {} (attempt {}): {:?}", port, attempts, e);
                        if attempts >= max_attempts {
                            eprintln!("[StreamServer] FATAL: Failed to start stream server after {} attempts", max_attempts);
                            return;
                        }
                        eprintln!("[StreamServer] Waiting 500ms before retry...");
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        });
        
        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        eprintln!("[StreamServer] Stream server initialization complete");
        
        server
    }
    
    pub fn get_port(&self) -> u16 {
        self.port
    }
    
    pub async fn create_token(&self, bucket: String, object_key: String) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        
        let stream_token = StreamToken {
            token: token.clone(),
            bucket,
            object_key,
            expires_at,
        };
        
        let mut tokens = self.tokens.lock().await;
        tokens.insert(token.clone(), stream_token);
        
        token
    }
    
    pub async fn revoke_token(&self, token: &str) {
        let mut tokens = self.tokens.lock().await;
        tokens.remove(token);
    }
}

#[derive(Clone)]
struct AppState {
    tokens: Arc<Mutex<HashMap<String, StreamToken>>>,
    minio_client: Arc<Mutex<Option<MinioClient>>>,
    db: Arc<Mutex<Database>>,
}

// Handler for stream requests
async fn stream_handler(
    AxumState(state): AxumState<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    eprintln!("[StreamHandler] Stream request received for token: {}", token);
    
    // Validate token
    let stream_token = {
        let tokens = state.tokens.lock().await;
        eprintln!("[StreamHandler] Looking for token: {} in {} tokens", token, tokens.len());
        match tokens.get(&token) {
            Some(t) => {
                if t.expires_at < chrono::Utc::now() {
                    eprintln!("[StreamHandler] ERROR: Token expired: {}", token);
                    return (StatusCode::UNAUTHORIZED, "Token expired").into_response();
                }
                eprintln!("[StreamHandler] ✓ Token validated: {} -> {}/{}", token, t.bucket, t.object_key);
                t.clone()
            }
            None => {
                eprintln!("[StreamHandler] ERROR: Invalid token: {}", token);
                eprintln!("[StreamHandler] Available tokens: {:?}", tokens.keys().take(5).collect::<Vec<_>>());
                return (StatusCode::NOT_FOUND, "Invalid token").into_response();
            }
        }
    };
    
    let object_key = stream_token.object_key.clone();

    // Get MinIO client
    let client_guard = state.minio_client.lock().await;
    let client = match client_guard.as_ref() {
        Some(c) => c,
        None => {
            println!("MinIO client not connected");
            return (StatusCode::SERVICE_UNAVAILABLE, "Not connected").into_response();
        }
    };

    // Try to get encryption key from database
    let encryption_key = {
        let db = state.db.lock().await;
        match db.get_encryption_key() {
            Ok(key) => key,
            Err(e) => {
                println!("Failed to get encryption key: {:?}", e);
                None
            }
        }
    };
    
    if let Some(ref key) = encryption_key {
        println!("Encryption key found, enabled: {}", key.enabled);
        if !key.enabled {
            println!("WARNING: Encryption is disabled but key exists - will NOT add SSE-C headers");
        }
    } else {
        println!("No encryption key configured");
    }

    // Parse Range header
    let range_header = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    if let Some(range) = range_header {
        println!("Range request: {}", range);
    }

    // Get object from MinIO
    let mut request = client
        .get_client()
        .get_object()
        .bucket(&stream_token.bucket)
        .key(&stream_token.object_key);

    // Add SSE-C headers if encryption key exists
    if let Some(ref key) = encryption_key {
        if key.enabled {
            println!("Adding SSE-C headers to stream request");
            use crate::encryption::SseCEncryption;
            match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &key.key_value) {
                Ok(key_bytes) => {
                    match SseCEncryption::new(key_bytes) {
                        Ok(sse_c) => {
                            request = request
                                .sse_customer_algorithm(sse_c.get_algorithm())
                                .sse_customer_key(sse_c.get_key_base64())
                                .sse_customer_key_md5(sse_c.get_key_md5());
                            println!("SSE-C headers added successfully");
                        }
                        Err(e) => {
                            println!("Failed to create SSE-C encryption: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to decode encryption key: {:?}", e);
                }
            }
        }
    }

    if let Some(range) = range_header {
        println!("Requesting range: {}", range);
        request = request.range(range);
    } else {
        println!("Requesting full object");
    }

    println!("Making request to MinIO for bucket: {}, key: {}", stream_token.bucket, stream_token.object_key);
    
    let result = match request.send().await {
        Ok(r) => {
            println!("Successfully received response from MinIO");
            r
        },
        Err(e) => {
            println!("Failed to get object from MinIO: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Get content info
    let content_length = result.content_length().unwrap_or(0);
    println!("Content length: {}", content_length);
    
    // For Range requests, we need the TOTAL file size, not the chunk size
    // MinIO returns content-range header like "bytes 0-1/12345678" for Range requests
    let content_range_header = result.content_range();
    println!("MinIO content-range: {:?}", content_range_header);
    
    // Extract total file size from content-range header if present
    let total_file_size = if let Some(range_str) = content_range_header {
        // Parse "bytes 0-1/12345678" to extract 12345678
        if let Some(slash_pos) = range_str.rfind('/') {
            range_str[slash_pos + 1..].parse::<i64>().unwrap_or(content_length)
        } else {
            content_length
        }
    } else {
        content_length
    };
    println!("Total file size: {}", total_file_size);
    
    // Try to get content type from MinIO, but prefer inferring from file extension
    let minio_content_type = result.content_type();
    println!("MinIO content type: {:?}", minio_content_type);
    
    // Always infer content type from file extension for better compatibility
    let content_type = if object_key.to_lowercase().ends_with(".mp4") {
        "video/mp4".to_string()
    } else if object_key.to_lowercase().ends_with(".avi") {
        "video/x-msvideo".to_string()
    } else if object_key.to_lowercase().ends_with(".mov") {
        "video/quicktime".to_string()
    } else if object_key.to_lowercase().ends_with(".wmv") {
        "video/x-ms-wmv".to_string()
    } else if object_key.to_lowercase().ends_with(".flv") {
        "video/x-flv".to_string()
    } else if object_key.to_lowercase().ends_with(".webm") {
        "video/webm".to_string()
    } else if object_key.to_lowercase().ends_with(".mkv") {
        "video/x-matroska".to_string()
    } else if object_key.to_lowercase().ends_with(".m4v") {
        "video/x-m4v".to_string()
    } else if object_key.to_lowercase().ends_with(".ogg") {
        "video/ogg".to_string()
    } else {
        // Fall back to MinIO's content type if available
        minio_content_type.map(|s| s.to_string()).unwrap_or_else(|| "application/octet-stream".to_string())
    };
    
    println!("Final content type: {}", content_type);
    
    // Check if content length is too small for a video file
    // BUT: Only warn for full requests, not Range requests
    // (browsers often request tiny ranges to probe the file)
    // Use total_file_size for this check, not content_length (which is chunk size for Range requests)
    if content_type.starts_with("video/") && total_file_size < 1000 && range_header.is_none() {
        eprintln!("ERROR: Video file is suspiciously small ({} bytes)!", content_length);
        eprintln!("This usually means:");
        eprintln!("  1. File upload was incomplete or failed");
        eprintln!("  2. Encryption mismatch: file was uploaded without encryption but trying to decrypt");
        eprintln!("  3. Encryption mismatch: file was uploaded with encryption but trying to read without decryption");
        eprintln!("Suggestion: Check upload task status and re-upload the file");
        
        // Try to detect encryption mismatch
        if let Some(ref key) = encryption_key {
            if key.enabled {
                eprintln!("Currently trying to decrypt with SSE-C. If file was uploaded WITHOUT encryption, this will fail.");
            }
        } else {
            eprintln!("Currently NOT using SSE-C decryption. If file was uploaded WITH encryption, this will fail.");
        }
    }
    
    // Create a stream from the response body
    let stream = result.body.into_async_read();
    let body_stream = ReaderStream::new(stream);
    
    // Debug: Check if we can read the stream
    println!("Creating stream from MinIO response");

    // Build response headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    response_headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    response_headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    
    // Add CORS headers for video content
    response_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
    response_headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, HEAD, OPTIONS".parse().unwrap());
    response_headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, "Range, Content-Type, Accept-Ranges".parse().unwrap());
    response_headers.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, "Content-Range, Accept-Ranges".parse().unwrap());

    // Calculate content range if range request
    if let Some(range_str) = range_header {
        println!("Processing range request: {}", range_str);
        // Use total_file_size for Content-Range header, NOT content_length (which is just the chunk size)
        if let Some(content_range) = calculate_content_range(range_str, total_file_size) {
            // For range requests, we need to determine the actual length of the range
            let actual_range_length = parse_range_length(range_str, total_file_size).unwrap_or(content_length);
            response_headers.insert(header::CONTENT_LENGTH, actual_range_length.to_string().parse().unwrap());
            response_headers.insert(header::CONTENT_RANGE, content_range.parse().unwrap());
            println!("Sending 206 Partial Content response with range: {}, length: {}", content_range, actual_range_length);
                    // Return 206 Partial Content for range requests
            let mut response_builder = axum::response::Response::builder()
                .status(StatusCode::PARTIAL_CONTENT);
            
            // Add all headers
            for (key, value) in response_headers {
                if let Some(header_name) = key {
                    response_builder = response_builder.header(header_name, value);
                }
            }
            
            match response_builder.body(body::Body::from_stream(body_stream)) {
                Ok(response) => response.into_response(),
                Err(_) => {
                    println!("Failed to build response");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response()
                }
            }
        } else {
            println!("Range not satisfiable: {}", range_str);
            Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(body::Body::from("Range not satisfiable"))
                .unwrap()
        }
    } else {
        // For full content requests
        response_headers.insert(header::CONTENT_LENGTH, content_length.to_string().parse().unwrap());
        println!("Sending 200 OK response with content length: {}", content_length);
        // Return 200 OK for full content
        let mut response_builder = axum::response::Response::builder()
            .status(StatusCode::OK);
        
        // Add all headers
        for (key, value) in response_headers {
            if let Some(header_name) = key {
                response_builder = response_builder.header(header_name, value);
            }
        }
        
        match response_builder.body(body::Body::from_stream(body_stream)) {
            Ok(response) => response.into_response(),
            Err(_) => {
                println!("Failed to build response");
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response()
            }
        }
    }
}

// Helper function to calculate content range
fn calculate_content_range(range_header: &str, total_length: i64) -> Option<String> {
    // Parse range header: "bytes=start-end"
    if range_header.starts_with("bytes=") {
        let range_part = &range_header[6..]; // Remove "bytes="
        if let Some((start_str, end_str)) = range_part.split_once('-') {
            let start = start_str.parse::<i64>().ok()?;
            let end = if end_str.is_empty() {
                total_length - 1
            } else {
                end_str.parse::<i64>().ok()?
            };
            
            if start >= total_length || end >= total_length || start > end {
                return None;
            }
            
            return Some(format!("bytes {}-{}/{}", start, end, total_length));
        }
    }
    None
}

// Helper function to parse the actual length of a range request
fn parse_range_length(range_header: &str, total_length: i64) -> Option<i64> {
    if range_header.starts_with("bytes=") {
        let range_part = &range_header[6..]; // Remove "bytes="
        if let Some((start_str, end_str)) = range_part.split_once('-') {
            let start = start_str.parse::<i64>().ok()?;
            let end = if end_str.is_empty() {
                total_length - 1
            } else {
                end_str.parse::<i64>().ok()?
            };
            
            if start >= total_length || end >= total_length || start > end {
                return None;
            }
            
            return Some(end - start + 1);
        }
    }
    None
}