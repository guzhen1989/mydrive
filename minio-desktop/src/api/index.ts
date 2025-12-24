import { invoke } from '@tauri-apps/api/tauri'

export interface ConnectionConfig {
  endpoint: string
  port: number
  accessKey: string
  secretKey: string
  useSsl: boolean
  lastConnected?: string
}

export interface BucketInfo {
  name: string
  creation_date?: string
}

export interface ObjectInfo {
  key: string
  size: number
  last_modified?: string
  content_type?: string
  is_dir: boolean
}

export interface TransferTask {
  task_id: string
  task_type: 'upload' | 'download'
  file_name: string
  local_path: string
  bucket_name: string
  object_key: string
  file_size: number
  transferred_bytes: number
  status: 'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'
  error_message?: string
  created_at: string
  updated_at: string
}

export const api = {
  // Connection
  async testConnection(config: Omit<ConnectionConfig, 'lastConnected'>): Promise<BucketInfo[]> {
    return invoke('test_connection', {
      endpoint: config.endpoint,
      port: config.port,
      accessKey: config.accessKey,
      secretKey: config.secretKey,
      useSsl: config.useSsl
    })
  },

  async saveConnection(config: Omit<ConnectionConfig, 'lastConnected'>): Promise<void> {
    return invoke('save_connection', {
      endpoint: config.endpoint,
      port: config.port,
      accessKey: config.accessKey,
      secretKey: config.secretKey,
      useSsl: config.useSsl
    })
  },

  async getConnection(): Promise<ConnectionConfig | null> {
    return invoke('get_connection')
  },

  // Buckets
  async listBuckets(): Promise<BucketInfo[]> {
    return invoke('list_buckets')
  },

  async createBucket(name: string): Promise<void> {
    return invoke('create_bucket', { name })
  },

  async deleteBucket(name: string): Promise<void> {
    return invoke('delete_bucket', { name })
  },

  // Objects
  async listObjects(bucket: string, prefix?: string): Promise<ObjectInfo[]> {
    return invoke('list_objects', { bucket, prefix })
  },

  async uploadFile(local_path: string, bucket: string, object_key: string): Promise<string> {
    return invoke('upload_file', { local_path, bucket, object_key })
  },

  async downloadFile(bucket: string, objectKey: string, localPath: string): Promise<string> {
    return invoke('download_file', { bucket, objectKey, localPath })
  },

  async deleteObject(bucket: string, key: string): Promise<void> {
    return invoke('delete_object', { bucket, key })
  },

  // Transfers
  async getTransferTasks(): Promise<TransferTask[]> {
    return invoke('get_transfer_tasks')
  },

  async pauseTask(task_id: string): Promise<void> {
    return invoke('pause_task', { task_id })
  },

  async resumeTask(task_id: string): Promise<void> {
    return invoke('resume_task', { task_id })
  },

  async cancelTask(task_id: string): Promise<void> {
    return invoke('cancel_task', { task_id })
  },

  async cancelAllTasks(): Promise<void> {
    return invoke('cancel_all_tasks')
  },

  async deleteTask(task_id: string): Promise<void> {
    return invoke('delete_task', { task_id })
  },

  async deleteCompletedTasks(): Promise<void> {
    return invoke('delete_completed_tasks')
  },

  // Streaming
  async getStreamUrl(bucket: string, objectKey: string): Promise<string> {
    return invoke('get_stream_url', { bucket, objectKey })
  },

  async getPresignedUrl(bucket: string, objectKey: string, expiresInSeconds?: number): Promise<string> {
    return invoke('get_presigned_url', { bucket, objectKey, expiresInSeconds })
  }
}
