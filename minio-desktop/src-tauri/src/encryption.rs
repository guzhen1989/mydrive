use base64::{Engine as _, engine::general_purpose};
use md5::{Md5, Digest};
use crate::error::{AppError, Result};

#[derive(Clone)]
pub struct SseCEncryption {
    key: Vec<u8>,
    key_md5: String,
}

impl SseCEncryption {
    /// Create new SSE-C encryption with a 32-byte key
    pub fn new(key: Vec<u8>) -> Result<Self> {
        if key.len() != 32 {
            return Err(AppError::Encryption(
                "SSE-C key must be exactly 32 bytes (256 bits)".to_string()
            ));
        }

        // Calculate MD5 of the key
        let mut hasher = Md5::new();
        hasher.update(&key);
        let key_md5 = general_purpose::STANDARD.encode(hasher.finalize());

        Ok(Self { key, key_md5 })
    }

    /// Get base64-encoded key for S3 headers
    pub fn get_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(&self.key)
    }

    /// Get MD5 hash of the key
    pub fn get_key_md5(&self) -> &str {
        &self.key_md5
    }

    /// Get algorithm name
    pub fn get_algorithm(&self) -> &str {
        "AES256"
    }
}

/// Generate a random 32-byte key for SSE-C
pub fn generate_random_key() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32).map(|_| rng.gen()).collect()
}

/// Validate if a key is valid for SSE-C
pub fn validate_key(key: &[u8]) -> Result<()> {
    if key.len() != 32 {
        return Err(AppError::Encryption(
            format!("Invalid key length: {} bytes (expected 32)", key.len())
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_c_encryption() {
        let key = vec![0u8; 32];
        let encryption = SseCEncryption::new(key).unwrap();
        
        assert_eq!(encryption.get_algorithm(), "AES256");
        assert!(!encryption.get_key_base64().is_empty());
        assert!(!encryption.get_key_md5().is_empty());
    }

    #[test]
    fn test_invalid_key_length() {
        let key = vec![0u8; 16]; // Invalid: only 16 bytes
        let result = SseCEncryption::new(key);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_random_key() {
        let key = generate_random_key();
        assert_eq!(key.len(), 32);
    }
}
