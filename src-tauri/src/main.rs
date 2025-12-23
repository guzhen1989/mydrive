#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use tauri::{Manager, State};
use serde_json::Value;
use std::sync::Mutex;

mod s3;
use s3::{S3ClientWrapper, ConnectConfig, CompletedPartResp};

#[derive(Default)]
struct AppState {
  client: Mutex<Option<S3ClientWrapper>>,
}

#[tauri::command]
async fn s3_connect_and_list(state: State<'_, AppState>, cfg: Value) -> Result<Value, String> {
  let cfg_map: ConnectConfig = serde_json::from_value(cfg).map_err(|e| e.to_string())?;
  let mut guard = state.client.lock().unwrap();
  let client = S3ClientWrapper::new(cfg_map).await.map_err(|e| e.to_string())?;
  let buckets = client.list_buckets().await.map_err(|e| e.to_string())?;
  *guard = Some(client);
  Ok(serde_json::to_value(buckets).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn s3_list_objects(state: State<'_, AppState>, bucket: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let objs = client.list_objects(&bucket).await.map_err(|e| e.to_string())?;
  Ok(serde_json::to_value(objs).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn s3_start_multipart_upload(state: State<'_, AppState>, bucket: String, key: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let upload_id = client.start_multipart_upload(&bucket, &key).await.map_err(|e| e.to_string())?;
  // persist record
  client.save_upload_record(&bucket, &key, &upload_id).await.map_err(|e| e.to_string())?;
  Ok(serde_json::json!({ "upload_id": upload_id }))
}

#[tauri::command]
async fn s3_list_parts(state: State<'_, AppState>, bucket: String, key: String, upload_id: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let parts = client.list_parts(&bucket, &key, &upload_id).await.map_err(|e| e.to_string())?;
  Ok(serde_json::to_value(parts).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn s3_upload_part(state: State<'_, AppState>, bucket: String, key: String, upload_id: String, part_number: i32, data_b64: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let bytes = base64::decode(data_b64).map_err(|e| e.to_string())?;
  let etag = client.upload_part(&bucket, &key, &upload_id, part_number, bytes).await.map_err(|e| e.to_string())?;
  // persist part info
  client.append_part_record(&bucket, &key, &upload_id, part_number, &etag).await.map_err(|e| e.to_string())?;
  Ok(serde_json::json!({ "etag": etag }))
}

#[tauri::command]
async fn s3_complete_multipart_upload(state: State<'_, AppState>, bucket: String, key: String, upload_id: String, parts: Vec<CompletedPartResp>) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let res = client.complete_multipart_upload(&bucket, &key, &upload_id, parts).await.map_err(|e| e.to_string())?;
  // remove persisted record on success
  client.remove_upload_record(&bucket, &key).await.map_err(|e| e.to_string())?;
  Ok(res)
}

#[tauri::command]
async fn s3_abort_multipart_upload(state: State<'_, AppState>, bucket: String, key: String, upload_id: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  client.abort_multipart_upload(&bucket, &key, &upload_id).await.map_err(|e| e.to_string())?;
  client.remove_upload_record(&bucket, &key).await.map_err(|e| e.to_string())?;
  Ok(serde_json::json!({"result":"ok"}))
}

#[tauri::command]
async fn s3_download_range(state: State<'_, AppState>, bucket: String, key: String, start: Option<u64>, end: Option<u64>) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let bytes = client.download_range(&bucket, &key, start, end).await.map_err(|e| e.to_string())?;
  let b64 = base64::encode(&bytes);
  Ok(serde_json::json!({ "data_b64": b64 }))
}

#[tauri::command]
async fn s3_download_range_append(state: State<'_, AppState>, bucket: String, key: String, start: Option<u64>, end: Option<u64>, temp_path: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let written = client.download_range_append(&bucket, &key, start, end, &temp_path).await.map_err(|e| e.to_string())?;
  Ok(serde_json::json!({ "written": written, "path": temp_path }))
}

#[tauri::command]
async fn s3_stream_to_temp(window: tauri::Window, state: State<'_, AppState>, bucket: String, key: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap().clone_for_streaming();
  let key_clone = key.clone();
  let bucket_clone = bucket.clone();
  tauri::async_runtime::spawn(async move {
    if let Err(e) = client.stream_object_to_temp_emit(&window, &bucket_clone, &key_clone).await {
      let _ = window.emit_all("stream-error", Some(serde_json::json!({ "key": key_clone, "error": format!("{}", e) })));
    }
  });
  let temp_path = client.temp_path_for(&key);
  Ok(serde_json::json!({ "path": temp_path }))
}

// persistence helpers: list persisted uploads and get one
#[tauri::command]
async fn s3_list_persisted_uploads(state: State<'_, AppState>) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let list = client.list_persisted_uploads().await.map_err(|e| e.to_string())?;
  Ok(serde_json::to_value(list).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn s3_get_persisted_upload(state: State<'_, AppState>, bucket: String, key: String) -> Result<Value, String> {
  let guard = state.client.lock().unwrap();
  if guard.is_none() { return Err("未连接 S3 客户端".into()); }
  let client = guard.as_ref().unwrap();
  let rec = client.get_persisted_upload(&bucket, &key).await.map_err(|e| e.to_string())?;
  Ok(serde_json::to_value(rec).map_err(|e| e.to_string())?)
}

fn main() {
  tauri::Builder::default()
    .manage(AppState::default())
    .invoke_handler(tauri::generate_handler![
      s3_connect_and_list,
      s3_list_objects,
      s3_start_multipart_upload,
      s3_list_parts,
      s3_upload_part,
      s3_complete_multipart_upload,
      s3_abort_multipart_upload,
      s3_download_range,
      s3_download_range_append,
      s3_stream_to_temp,
      s3_list_persisted_uploads,
      s3_get_persisted_upload
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}