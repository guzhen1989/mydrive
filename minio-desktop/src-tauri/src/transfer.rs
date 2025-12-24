use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
use tokio::sync::{Mutex, Semaphore};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use crate::error::{AppError, Result};
use crate::models::{TransferTask, TaskType, TaskStatus, CompletedPart as ModelCompletedPart};
use crate::minio::MinioClient;
use crate::db::Database;
use chrono::Utc;

const PART_SIZE: usize = 5 * 1024 * 1024; // 5MB
const MAX_CONCURRENT_PARTS: usize = 5;

pub struct TransferManager {
    db: Arc<Mutex<Database>>,
    minio_client: Arc<Mutex<Option<MinioClient>>>,
}

impl TransferManager {
    pub fn new(db: Arc<Mutex<Database>>, minio_client: Arc<Mutex<Option<MinioClient>>>) -> Self {
        Self { db, minio_client }
    }

    /// Start an upload task
    pub async fn start_upload(
        &self,
        task_id: String,
        local_path: String,
        bucket: String,
        object_key: String,
    ) -> Result<()> {
        let file_metadata = tokio::fs::metadata(&local_path).await?;
        let file_size = file_metadata.len() as i64;

        // Create task record
        let mut task = TransferTask {
            task_id: task_id.clone(),
            task_type: TaskType::Upload,
            file_name: Path::new(&local_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            local_path: local_path.clone(),
            bucket_name: bucket.clone(),
            object_key: object_key.clone(),
            file_size,
            upload_id: None,
            part_size: PART_SIZE as i64,
            total_parts: ((file_size as f64) / (PART_SIZE as f64)).ceil() as i32,
            completed_parts: vec![],
            transferred_bytes: 0,
            status: TaskStatus::Running,
            error_message: None,
            use_encryption: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        // Save initial task
        {
            let db = self.db.lock().await;
            db.save_task(&task)?;
        }

        // Perform upload
        let result = if file_size < PART_SIZE as i64 {
            self.upload_small_file(&task, &local_path, &bucket, &object_key).await
        } else {
            self.upload_large_file(&mut task, &local_path, &bucket, &object_key).await
        };

        // Update task status
        match result {
            Ok(_) => {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(Utc::now());
                task.transferred_bytes = file_size;
            }
            Err(ref e) => {
                task.status = TaskStatus::Failed;
                task.error_message = Some(e.to_string());
            }
        }

        task.updated_at = Utc::now();
        let db = self.db.lock().await;
        db.save_task(&task)?;

        result
    }

    /// Upload small file (< 5MB) using simple PUT
    async fn upload_small_file(
        &self,
        task: &TransferTask,
        local_path: &str,
        bucket: &str,
        object_key: &str,
    ) -> Result<()> {
        let client_guard = self.minio_client.lock().await;
        let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;

        let body = ByteStream::from_path(Path::new(local_path))
            .await
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        client
            .get_client()
            .put_object()
            .bucket(bucket)
            .key(object_key)
            .body(body)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        Ok(())
    }

    /// Upload large file using multipart upload
    async fn upload_large_file(
        &self,
        task: &mut TransferTask,
        local_path: &str,
        bucket: &str,
        object_key: &str,
    ) -> Result<()> {
        let client_guard = self.minio_client.lock().await;
        let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;

        // Initiate multipart upload
        let multipart_upload = client
            .get_client()
            .create_multipart_upload()
            .bucket(bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        let upload_id = multipart_upload
            .upload_id()
            .ok_or(AppError::S3("No upload ID".to_string()))?
            .to_string();

        task.upload_id = Some(upload_id.clone());
        
        // Save task with upload_id
        {
            let db = self.db.lock().await;
            db.save_task(task)?;
        }

        // Upload parts
        let mut file = File::open(local_path).await?;
        let mut part_number = 1;
        let mut completed_parts = Vec::new();
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_PARTS));

        loop {
            let mut buffer = vec![0u8; PART_SIZE];
            let bytes_read = file.read(&mut buffer).await?;
            
            if bytes_read == 0 {
                break;
            }

            buffer.truncate(bytes_read);
            
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client_clone = client.get_client().clone();
            let bucket_clone = bucket.to_string();
            let key_clone = object_key.to_string();
            let upload_id_clone = upload_id.clone();
            let part_num = part_number;

            let upload_result = tokio::spawn(async move {
                let _permit = permit;
                let result = client_clone
                    .upload_part()
                    .bucket(&bucket_clone)
                    .key(&key_clone)
                    .upload_id(&upload_id_clone)
                    .part_number(part_num)
                    .body(ByteStream::from(buffer))
                    .send()
                    .await;
                
                result.map(|output| (part_num, output.e_tag().unwrap_or("").to_string()))
            })
            .await
            .map_err(|e| AppError::Other(e.to_string()))?
            .map_err(|e| AppError::S3(e.to_string()))?;

            completed_parts.push(CompletedPart::builder()
                .part_number(upload_result.0)
                .e_tag(upload_result.1.clone())
                .build());

            // Update task progress
            task.completed_parts.push(ModelCompletedPart {
                part_number: upload_result.0,
                etag: upload_result.1,
                size: bytes_read as i64,
            });
            task.transferred_bytes += bytes_read as i64;
            task.updated_at = Utc::now();

            {
                let db = self.db.lock().await;
                db.save_task(task)?;
            }

            part_number += 1;
        }

        // Complete multipart upload
        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        client
            .get_client()
            .complete_multipart_upload()
            .bucket(bucket)
            .key(object_key)
            .upload_id(&upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        Ok(())
    }

    /// Resume an upload task
    pub async fn resume_upload(&self, task_id: &str) -> Result<()> {
        let mut task = {
            let db = self.db.lock().await;
            db.get_task(task_id)?
                .ok_or(AppError::TaskNotFound(task_id.to_string()))?
        };

        if task.task_type != TaskType::Upload {
            return Err(AppError::Other("Not an upload task".to_string()));
        }

        if task.upload_id.is_none() {
            return Err(AppError::Other("No upload ID found".to_string()));
        }

        task.status = TaskStatus::Running;
        task.updated_at = Utc::now();

        {
            let db = self.db.lock().await;
            db.save_task(&task)?;
        }

        // Continue uploading remaining parts
        self.continue_multipart_upload(&mut task).await
    }

    async fn continue_multipart_upload(&self, task: &mut TransferTask) -> Result<()> {
        let client_guard = self.minio_client.lock().await;
        let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;

        let upload_id = task.upload_id.as_ref()
            .ok_or(AppError::Other("No upload ID".to_string()))?;

        let mut file = File::open(&task.local_path).await?;
        let completed_part_numbers: Vec<i32> = task.completed_parts
            .iter()
            .map(|p| p.part_number)
            .collect();

        let mut part_number = 1;
        let mut completed_parts_for_s3 = Vec::new();

        // Convert existing completed parts
        for cp in &task.completed_parts {
            completed_parts_for_s3.push(CompletedPart::builder()
                .part_number(cp.part_number)
                .e_tag(&cp.etag)
                .build());
        }

        loop {
            let mut buffer = vec![0u8; PART_SIZE];
            let offset = ((part_number - 1) as usize) * PART_SIZE;
            file.seek(std::io::SeekFrom::Start(offset as u64)).await?;
            
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            // Skip already uploaded parts
            if completed_part_numbers.contains(&part_number) {
                part_number += 1;
                continue;
            }

            buffer.truncate(bytes_read);

            let upload_result = client
                .get_client()
                .upload_part()
                .bucket(&task.bucket_name)
                .key(&task.object_key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(ByteStream::from(buffer))
                .send()
                .await
                .map_err(|e| AppError::S3(e.to_string()))?;

            let etag = upload_result.e_tag().unwrap_or("").to_string();

            completed_parts_for_s3.push(CompletedPart::builder()
                .part_number(part_number)
                .e_tag(&etag)
                .build());

            task.completed_parts.push(ModelCompletedPart {
                part_number,
                etag,
                size: bytes_read as i64,
            });
            task.transferred_bytes += bytes_read as i64;
            task.updated_at = Utc::now();

            {
                let db = self.db.lock().await;
                db.save_task(task)?;
            }

            part_number += 1;
        }

        // Complete multipart upload
        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts_for_s3))
            .build();

        client
            .get_client()
            .complete_multipart_upload()
            .bucket(&task.bucket_name)
            .key(&task.object_key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        task.status = TaskStatus::Completed;
        task.completed_at = Some(Utc::now());
        task.updated_at = Utc::now();

        let db = self.db.lock().await;
        db.save_task(task)?;

        Ok(())
    }

    /// Start a download task
    pub async fn start_download(
        &self,
        task_id: String,
        bucket: String,
        object_key: String,
        local_path: String,
    ) -> Result<()> {
        let client_guard = self.minio_client.lock().await;
        let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;

        // Get object size
        let head_result = client
            .get_client()
            .head_object()
            .bucket(&bucket)
            .key(&object_key)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        let file_size = head_result.content_length().unwrap_or(0);

        let mut task = TransferTask {
            task_id: task_id.clone(),
            task_type: TaskType::Download,
            file_name: Path::new(&object_key)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            local_path: local_path.clone(),
            bucket_name: bucket.clone(),
            object_key: object_key.clone(),
            file_size,
            upload_id: None,
            part_size: PART_SIZE as i64,
            total_parts: ((file_size as f64) / (PART_SIZE as f64)).ceil() as i32,
            completed_parts: vec![],
            transferred_bytes: 0,
            status: TaskStatus::Running,
            error_message: None,
            use_encryption: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        {
            let db = self.db.lock().await;
            db.save_task(&task)?;
        }

        // Download file
        let result = if file_size < PART_SIZE as i64 {
            println!("Starting small file download for task {}: {} bytes", task.task_id, file_size);
            let bytes_downloaded = self.download_small_file(&task, &bucket, &object_key, &local_path).await?;
            println!("Small file download completed for task {}: {} bytes", task.task_id, bytes_downloaded);
            // For small files, we already updated the progress in download_small_file
            Ok(())
        } else {
            println!("Starting large file download for task {}: {} bytes", task.task_id, file_size);
            self.download_large_file(&mut task, &bucket, &object_key, &local_path).await
        };

        match result {
            Ok(_) => {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(Utc::now());
                task.transferred_bytes = file_size;
            }
            Err(ref e) => {
                task.status = TaskStatus::Failed;
                task.error_message = Some(e.to_string());
            }
        }

        task.updated_at = Utc::now();
        let db = self.db.lock().await;
        db.save_task(&task)?;

        result
    }

    async fn download_small_file(
        &self,
        task: &TransferTask,
        bucket: &str,
        object_key: &str,
        local_path: &str,
    ) -> Result<i64> {
        // Check if task is cancelled before starting
        {
            let db = self.db.lock().await;
            if let Some(current_task) = db.get_task(&task.task_id)? {
                if current_task.status == TaskStatus::Cancelled {
                    println!("Task {} is cancelled, aborting download", task.task_id);
                    return Err(AppError::Other("Task cancelled".to_string()));
                }
            }
        }

        let client_guard = self.minio_client.lock().await;
        let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;

        println!("Starting download for task {}: {} bytes", task.task_id, task.file_size);

        let result = client
            .get_client()
            .get_object()
            .bucket(bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|e| AppError::S3(e.to_string()))?;

        let data = result.body.collect().await
            .map_err(|e| AppError::Other(e.to_string()))?;

        let bytes = data.into_bytes();
        let mut file = File::create(local_path).await?;
        file.write_all(&bytes).await?;

        // Update task progress in the database
        let mut updated_task = task.clone();
        updated_task.transferred_bytes = bytes.len() as i64;
        updated_task.updated_at = Utc::now();
        
        let db = self.db.lock().await;
        db.save_task(&updated_task)?;
        println!("Saved task progress to database: {} bytes", updated_task.transferred_bytes);

        println!("Download completed for task {}: {} bytes", task.task_id, bytes.len());

        Ok(bytes.len() as i64)
    }

    async fn download_large_file(
        &self,
        task: &mut TransferTask,
        bucket: &str,
        object_key: &str,
        local_path: &str,
    ) -> Result<()> {
        let client_guard = self.minio_client.lock().await;
        let client = client_guard.as_ref().ok_or(AppError::NotConnected)?;

        let mut file = File::create(local_path).await?;
        let mut offset = 0i64;

        println!("Starting large file download for task {}: {} bytes", task.task_id, task.file_size);

        // Check if task is cancelled before starting the loop
        {
            let db = self.db.lock().await;
            if let Some(current_task) = db.get_task(&task.task_id)? {
                if current_task.status == TaskStatus::Cancelled {
                    println!("Task {} is cancelled before download loop", task.task_id);
                    return Err(AppError::Other("Task cancelled".to_string()));
                }
            }
        }

        while offset < task.file_size {
            // Check if task is cancelled
            {
                let db = self.db.lock().await;
                if let Some(current_task) = db.get_task(&task.task_id)? {
                    if current_task.status == TaskStatus::Cancelled {
                        println!("Task {} is cancelled, aborting download", task.task_id);
                        return Err(AppError::Other("Task cancelled".to_string()));
                    }
                }
            }

            let end = std::cmp::min(offset + PART_SIZE as i64 - 1, task.file_size - 1);
            let range = format!("bytes={}-{}", offset, end);

            println!("Downloading range {} for task {}", range, task.task_id);

            println!("About to send range request: {}", range);
            
            // 使用预签名URL进行下载
            let presigned_config = aws_sdk_s3::presigning::PresigningConfig::builder()
                .expires_in(std::time::Duration::from_secs(3600)) // 1小时过期
                .build()
                .map_err(|e| AppError::S3(e.to_string()))?;
            
            let presigned_request = client
                .get_client()
                .get_object()
                .bucket(bucket)
                .key(object_key)
                .range(&range)
                .presigned(presigned_config)
                .await
                .map_err(|e| {
                    println!("Failed to create presigned URL for range {}: {}", range, e);
                    AppError::S3(e.to_string())
                })?;
            
            // 使用reqwest库下载预签名URL的内容
            let http_client = reqwest::Client::builder()
                .build()
                .map_err(|e| AppError::Other(e.to_string()))?;
            
            let response = http_client
                .get(presigned_request.uri())
                .send()
                .await
                .map_err(|e| {
                    println!("HTTP request failed for range {}: {}", range, e);
                    AppError::Other(e.to_string())
                })?;
            
            if !response.status().is_success() {
                return Err(AppError::Other(format!("HTTP request failed with status: {}", response.status())));
            }
            
            let bytes = response
                .bytes()
                .await
                .map_err(|e| {
                    println!("Failed to read response bytes for range {}: {}", range, e);
                    AppError::Other(e.to_string())
                })?;
            
            println!("Successfully downloaded {} bytes for range {}", bytes.len(), range);

            println!("Range request completed for task {}: {}", task.task_id, range);
            
            file.write_all(&bytes).await?;
            println!("Wrote {} bytes to file", bytes.len());

            task.transferred_bytes += bytes.len() as i64;
            task.updated_at = Utc::now();

            {
                let db = self.db.lock().await;
                db.save_task(task)?;
                println!("Progress updated for task {}: {}/{} bytes ({}%)", task.task_id, task.transferred_bytes, task.file_size, (task.transferred_bytes as f64 / task.file_size as f64 * 100.0) as i32);
            }

            offset = end + 1;
            println!("Updated offset to {}, total file size: {}", offset, task.file_size);
        }

        println!("Large file download completed for task {}", task.task_id);

        Ok(())
    }
}
