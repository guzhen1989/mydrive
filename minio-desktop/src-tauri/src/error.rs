use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("S3 error: {0}")]
    S3(String),
    
    #[error("Connection not configured")]
    NotConnected,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
