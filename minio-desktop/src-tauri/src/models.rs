use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub endpoint: String,
    pub port: u16,
    pub access_key: String,
    pub secret_key: String,
    pub use_ssl: bool,
    pub last_connected: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub name: String,
    pub creation_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<DateTime<Utc>>,
    pub content_type: Option<String>,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferTask {
    pub task_id: String,
    pub task_type: TaskType,
    pub file_name: String,
    pub local_path: String,
    pub bucket_name: String,
    pub object_key: String,
    pub file_size: i64,
    pub upload_id: Option<String>,
    pub part_size: i64,
    pub total_parts: i32,
    pub completed_parts: Vec<CompletedPart>,
    pub transferred_bytes: i64,
    pub status: TaskStatus,
    pub error_message: Option<String>,
    pub use_encryption: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Upload,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedPart {
    pub part_number: i32,
    pub etag: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub key_id: String,
    pub key_value: String,
    pub key_md5: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToken {
    pub token: String,
    pub bucket: String,
    pub object_key: String,
    pub expires_at: DateTime<Utc>,
}
