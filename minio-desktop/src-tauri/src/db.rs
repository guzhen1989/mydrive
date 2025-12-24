use rusqlite::{Connection, params};
use crate::error::Result;
use crate::models::{ConnectionConfig, TransferTask, TaskType, TaskStatus, CompletedPart, EncryptionKey};
use chrono::Utc;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init()?;
        Ok(db)
    }
    
    fn init(&self) -> Result<()> {
        // Create connection_config table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS connection_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                endpoint TEXT NOT NULL,
                port INTEGER NOT NULL,
                access_key TEXT NOT NULL,
                secret_key TEXT NOT NULL,
                use_ssl INTEGER NOT NULL,
                last_connected TEXT
            )",
            [],
        )?;
        
        // Create transfer_tasks table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS transfer_tasks (
                task_id TEXT PRIMARY KEY,
                task_type TEXT NOT NULL,
                file_name TEXT NOT NULL,
                local_path TEXT NOT NULL,
                bucket_name TEXT NOT NULL,
                object_key TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                upload_id TEXT,
                part_size INTEGER NOT NULL,
                total_parts INTEGER NOT NULL,
                completed_parts TEXT NOT NULL,
                transferred_bytes INTEGER NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                use_encryption INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            )",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_task_status ON transfer_tasks(status)",
            [],
        )?;
        
        // Create encryption_keys table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS encryption_keys (
                key_id TEXT PRIMARY KEY,
                key_value TEXT NOT NULL,
                key_md5 TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        
        Ok(())
    }
    
    // Connection config methods
    pub fn save_connection(&self, config: &ConnectionConfig) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO connection_config (id, endpoint, port, access_key, secret_key, use_ssl, last_connected)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                config.endpoint,
                config.port,
                config.access_key,
                config.secret_key,
                config.use_ssl as i32,
                config.last_connected.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }
    
    pub fn get_connection(&self) -> Result<Option<ConnectionConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT endpoint, port, access_key, secret_key, use_ssl, last_connected FROM connection_config WHERE id = 1"
        )?;
        
        let result = stmt.query_row([], |row| {
            Ok(ConnectionConfig {
                endpoint: row.get(0)?,
                port: row.get(1)?,
                access_key: row.get(2)?,
                secret_key: row.get(3)?,
                use_ssl: row.get::<_, i32>(4)? != 0,
                last_connected: row.get::<_, Option<String>>(5)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        });
        
        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    // Transfer task methods
    pub fn save_task(&self, task: &TransferTask) -> Result<()> {
        let task_type = match task.task_type {
            TaskType::Upload => "upload",
            TaskType::Download => "download",
        };
        
        let status = match task.status {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Paused => "paused",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        };
        
        let completed_parts_json = serde_json::to_string(&task.completed_parts)
            .map_err(|e| crate::error::AppError::Other(e.to_string()))?;
        
        self.conn.execute(
            "INSERT OR REPLACE INTO transfer_tasks 
             (task_id, task_type, file_name, local_path, bucket_name, object_key, file_size, 
              upload_id, part_size, total_parts, completed_parts, transferred_bytes, status, 
              error_message, use_encryption, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                task.task_id,
                task_type,
                task.file_name,
                task.local_path,
                task.bucket_name,
                task.object_key,
                task.file_size,
                task.upload_id,
                task.part_size,
                task.total_parts,
                completed_parts_json,
                task.transferred_bytes,
                status,
                task.error_message,
                task.use_encryption as i32,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        
        Ok(())
    }
    
    pub fn get_task(&self, task_id: &str) -> Result<Option<TransferTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, task_type, file_name, local_path, bucket_name, object_key, file_size,
                    upload_id, part_size, total_parts, completed_parts, transferred_bytes, status,
                    error_message, use_encryption, created_at, updated_at, completed_at
             FROM transfer_tasks WHERE task_id = ?1"
        )?;
        
        let result = stmt.query_row([task_id], |row| {
            let completed_parts_json: String = row.get(10)?;
            let completed_parts: Vec<CompletedPart> = serde_json::from_str(&completed_parts_json)
                .unwrap_or_default();
            
            Ok(TransferTask {
                task_id: row.get(0)?,
                task_type: match row.get::<_, String>(1)?.as_str() {
                    "upload" => TaskType::Upload,
                    _ => TaskType::Download,
                },
                file_name: row.get(2)?,
                local_path: row.get(3)?,
                bucket_name: row.get(4)?,
                object_key: row.get(5)?,
                file_size: row.get(6)?,
                upload_id: row.get(7)?,
                part_size: row.get(8)?,
                total_parts: row.get(9)?,
                completed_parts,
                transferred_bytes: row.get(11)?,
                status: match row.get::<_, String>(12)?.as_str() {
                    "running" => TaskStatus::Running,
                    "paused" => TaskStatus::Paused,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    "cancelled" => TaskStatus::Cancelled,
                    _ => TaskStatus::Pending,
                },
                error_message: row.get(13)?,
                use_encryption: row.get::<_, i32>(14)? != 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(16)?)
                    .unwrap()
                    .with_timezone(&Utc),
                completed_at: row.get::<_, Option<String>>(17)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        });
        
        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    pub fn get_all_tasks(&self) -> Result<Vec<TransferTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, task_type, file_name, local_path, bucket_name, object_key, file_size,
                    upload_id, part_size, total_parts, completed_parts, transferred_bytes, status,
                    error_message, use_encryption, created_at, updated_at, completed_at
             FROM transfer_tasks ORDER BY created_at DESC"
        )?;
        
        let tasks = stmt.query_map([], |row| {
            let completed_parts_json: String = row.get(10)?;
            let completed_parts: Vec<CompletedPart> = serde_json::from_str(&completed_parts_json)
                .unwrap_or_default();
            
            Ok(TransferTask {
                task_id: row.get(0)?,
                task_type: match row.get::<_, String>(1)?.as_str() {
                    "upload" => TaskType::Upload,
                    _ => TaskType::Download,
                },
                file_name: row.get(2)?,
                local_path: row.get(3)?,
                bucket_name: row.get(4)?,
                object_key: row.get(5)?,
                file_size: row.get(6)?,
                upload_id: row.get(7)?,
                part_size: row.get(8)?,
                total_parts: row.get(9)?,
                completed_parts,
                transferred_bytes: row.get(11)?,
                status: match row.get::<_, String>(12)?.as_str() {
                    "running" => TaskStatus::Running,
                    "paused" => TaskStatus::Paused,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    "cancelled" => TaskStatus::Cancelled,
                    _ => TaskStatus::Pending,
                },
                error_message: row.get(13)?,
                use_encryption: row.get::<_, i32>(14)? != 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(16)?)
                    .unwrap()
                    .with_timezone(&Utc),
                completed_at: row.get::<_, Option<String>>(17)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })?;
        
        tasks.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }
    
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM transfer_tasks WHERE task_id = ?1", [task_id])?;
        Ok(())
    }
    
    pub fn update_task_status(&self, task_id: &str, status: crate::models::TaskStatus) -> Result<()> {
        let status_str = match status {
            crate::models::TaskStatus::Pending => "pending",
            crate::models::TaskStatus::Running => "running",
            crate::models::TaskStatus::Paused => "paused",
            crate::models::TaskStatus::Completed => "completed",
            crate::models::TaskStatus::Failed => "failed",
            crate::models::TaskStatus::Cancelled => "cancelled",
        };
        
        self.conn.execute(
            "UPDATE transfer_tasks SET status = ?1, updated_at = ?2 WHERE task_id = ?3",
            params![status_str, chrono::Utc::now().to_rfc3339(), task_id],
        )?;
        
        Ok(())
    }

    // Encryption key methods
    pub fn save_encryption_key(&self, key: &EncryptionKey) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO encryption_keys (key_id, key_value, key_md5, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                key.key_id,
                key.key_value,
                key.key_md5,
                key.enabled as i32,
                key.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }
    
    pub fn get_encryption_key(&self) -> Result<Option<EncryptionKey>> {
        let mut stmt = self.conn.prepare(
            "SELECT key_id, key_value, key_md5, enabled, created_at 
             FROM encryption_keys WHERE enabled = 1 LIMIT 1"
        )?;
        
        let result = stmt.query_row([], |row| {
            Ok(EncryptionKey {
                key_id: row.get(0)?,
                key_value: row.get(1)?,
                key_md5: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        });
        
        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
