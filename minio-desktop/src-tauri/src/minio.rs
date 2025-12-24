use aws_sdk_s3::{Client, Config, config::Region, config::Credentials, config::Builder as S3ConfigBuilder, config::BehaviorVersion};
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use std::time::Duration;
use crate::error::{AppError, Result};
use crate::models::{ConnectionConfig, BucketInfo, ObjectInfo};
use rustls::client::{ServerCertVerifier, ServerCertVerified};
use rustls::{Certificate, ServerName, Error as RustlsError};

// Custom certificate verifier that accepts all certificates (including self-signed)
struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }
}

pub struct MinioClient {
    client: Client,
    config: ConnectionConfig,
}

impl MinioClient {
    pub async fn new(config: ConnectionConfig) -> Result<Self> {
        let endpoint_url = format!(
            "{}://{}:{}",
            if config.use_ssl { "https" } else { "http" },
            config.endpoint,
            config.port
        );
        
        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "minio-desktop"
        );
        
        // Configure timeouts for streaming video
        let timeout_config = TimeoutConfig::builder()
            .operation_timeout(Duration::from_secs(300)) // 5 minutes for operations
            .operation_attempt_timeout(Duration::from_secs(300)) // 5 minutes per attempt
            .connect_timeout(Duration::from_secs(30)) // 30 seconds for connection
            .read_timeout(Duration::from_secs(300)) // 5 minutes for read
            .build();
        
        // Create a custom TLS connector that accepts self-signed certificates
        let tls_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(std::sync::Arc::new(NoCertificateVerification))
            .with_no_client_auth();
        
        let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();
        
        let hyper_client = HyperClientBuilder::new().build(https_connector);
        
        let s3_config_builder = S3ConfigBuilder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials.clone())
            .timeout_config(timeout_config)
            .http_client(hyper_client)
            .force_path_style(true); // 强制使用路径样式寻址，这对 MinIO 很重要
        
        let s3_config_builder = if !endpoint_url.is_empty() {
            s3_config_builder.endpoint_url(&endpoint_url)
        } else {
            s3_config_builder
        };
        
        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);
        
        Ok(MinioClient { client, config })
    }
    
    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
        let resp = self.client
            .list_buckets()
            .send()
            .await
            .map_err(|e| {
                eprintln!("S3 ListBuckets Error: {:?}", e);
                AppError::S3(e.to_string())
            })?;
        
        let buckets = resp.buckets()
            .iter()
            .map(|b| BucketInfo {
                name: b.name().unwrap_or("").to_string(),
                creation_date: b.creation_date()
                    .and_then(|dt| chrono::DateTime::parse_from_rfc3339(&dt.to_string()).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
            })
            .collect();
        
        Ok(buckets)
    }
    
    pub async fn create_bucket(&self, bucket_name: &str) -> Result<()> {
        // 验证存储桶名称
        if bucket_name.is_empty() {
            return Err(AppError::S3("存储桶名称不能为空".to_string()));
        }
        
        // S3标准要求存储桶名称只能包含小写字母、数字和连字符，长度为3-63个字符
        // 且必须以字母或数字开头和结尾
        if bucket_name.len() < 3 || bucket_name.len() > 63 {
            return Err(AppError::S3("存储桶名称长度必须在3-63个字符之间".to_string()));
        }
        
        // 检查是否只包含允许的字符
        if !bucket_name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(AppError::S3("存储桶名称只能包含小写字母、数字和连字符".to_string()));
        }
        
        // 检查是否以字母或数字开头和结尾
        if let Some(first_char) = bucket_name.chars().next() {
            if !first_char.is_ascii_alphanumeric() {
                return Err(AppError::S3("存储桶名称必须以字母或数字开头".to_string()));
            }
        }
        
        if let Some(last_char) = bucket_name.chars().last() {
            if !last_char.is_ascii_alphanumeric() {
                return Err(AppError::S3("存储桶名称必须以字母或数字结尾".to_string()));
            }
        }
        
        self.client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|e| {
                eprintln!("S3 CreateBucket Error: {:?}", e);
                AppError::S3(e.to_string())
            })?;
        
        Ok(())
    }
    
    pub async fn delete_bucket(&self, bucket_name: &str) -> Result<()> {
        self.client
            .delete_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;
        
        Ok(())
    }
    
    pub async fn list_objects(&self, bucket: &str, prefix: Option<&str>) -> Result<Vec<ObjectInfo>> {
        println!("Listing objects in bucket: {}, prefix: {:?}", bucket, prefix);
        
        let mut request = self.client
            .list_objects_v2()
            .bucket(bucket);

        // 设置 prefix 参数
        if let Some(p) = prefix {
            request = request.prefix(p);
            println!("Using prefix: {}", p);
        }

        // 设置 fetch-owner 为 false，避免额外开销
        request = request.fetch_owner(false);
        
        // 设置 delimiter 为 "/"，以实现文件夹功能
        request = request.delimiter("/");

        let resp = request
            .send()
            .await
            .map_err(|e| {
                eprintln!("Failed to list objects: {:?}", e);
                AppError::S3(e.to_string())
            })?;

        let mut objects = Vec::new();

        // Add directories (common prefixes)
        for prefix in resp.common_prefixes() {
            if let Some(p) = prefix.prefix() {
                println!("Found directory: {}", p);
                objects.push(ObjectInfo {
                    key: p.to_string(),
                    size: 0,
                    last_modified: None,
                    content_type: None,
                    is_dir: true,
                });
            }
        }

        // Add files
        for obj in resp.contents() {
            let key = obj.key().unwrap_or("");
            let size = obj.size().unwrap_or(0);
            println!("Found file: {}, size: {}", key, size);
            objects.push(ObjectInfo {
                key: key.to_string(),
                size,
                last_modified: obj.last_modified()
                    .and_then(|dt| chrono::DateTime::parse_from_rfc3339(&dt.to_string()).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                content_type: None, // content_type method doesn't exist on this Object type
                is_dir: false,
            });
        }

        println!("Total objects found: {}", objects.len());
        Ok(objects)
    }
    
    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;
        
        Ok(())
    }
    
    pub fn get_client(&self) -> &Client {
        &self.client
    }
}
