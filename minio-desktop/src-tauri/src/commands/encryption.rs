use tauri::State;
use crate::{AppState, error::Result, models::EncryptionKey, encryption};

#[tauri::command]
pub async fn generate_encryption_key() -> Result<String> {
    let key = encryption::generate_random_key();
    let key_base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &key);
    Ok(key_base64)
}

#[tauri::command]
pub async fn save_encryption_key(
    state: State<'_, AppState>,
    key_base64: String,
) -> Result<()> {
    // Decode base64 key
    let key = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &key_base64)
        .map_err(|e| crate::error::AppError::Encryption(e.to_string()))?;
    
    // Validate key
    encryption::validate_key(&key)?;
    
    // Create encryption object to get MD5
    let sse_c = encryption::SseCEncryption::new(key)?;
    
    // Create key record
    let encryption_key = EncryptionKey {
        key_id: uuid::Uuid::new_v4().to_string(),
        key_value: key_base64,
        key_md5: sse_c.get_key_md5().to_string(),
        enabled: true,
        created_at: chrono::Utc::now(),
    };
    
    // Save to database
    let db = state.db.lock().await;
    db.save_encryption_key(&encryption_key)?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_encryption_key(
    state: State<'_, AppState>,
) -> Result<Option<EncryptionKey>> {
    let db = state.db.lock().await;
    db.get_encryption_key()
}

#[tauri::command]
pub async fn validate_encryption_key(key_base64: String) -> Result<bool> {
    match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &key_base64) {
        Ok(key) => {
            encryption::validate_key(&key)?;
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}
