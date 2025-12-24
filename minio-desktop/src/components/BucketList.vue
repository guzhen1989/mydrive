<template>
  <div class="bucket-list">
    <div class="header">
      <h3>存储桶</h3>
      <button @click="showCreateDialog = true" class="create-btn">+</button>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="buckets">
      <div
        v-for="bucket in buckets"
        :key="bucket.name"
        class="bucket-item"
        :class="{ active: bucket.name === currentBucket }"
        @click="selectBucket(bucket.name)"
      >
        <span class="bucket-icon">🗂</span>
        <span class="bucket-name">{{ bucket.name }}</span>
      </div>

      <div v-if="buckets.length === 0" class="empty">
        暂无存储桶
      </div>
    </div>

    <!-- Create Bucket Dialog -->
    <div v-if="showCreateDialog" class="dialog-overlay" @click="showCreateDialog = false">
      <div class="dialog" @click.stop>
        <h4>创建存储桶</h4>
        <input
          v-model="newBucketName"
          type="text"
          placeholder="存储桶名称"
          @keyup.enter="handleCreate"
        />
        <div class="dialog-actions">
          <button @click="showCreateDialog = false">取消</button>
          <button @click="handleCreate" class="primary">创建</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useBucketStore } from '../stores/bucket'
import { useObjectStore } from '../stores/object'

const bucketStore = useBucketStore()
const objectStore = useObjectStore()
const { buckets, currentBucket, loading } = storeToRefs(bucketStore)

const showCreateDialog = ref(false)
const newBucketName = ref('')

async function selectBucket(name: string) {
  bucketStore.selectBucket(name)
  await objectStore.fetchObjects(name)
}

async function handleCreate() {
  if (!newBucketName.value) {
    alert('请输入存储桶名称')
    return
  }
  
  // S3标准要求存储桶名称只能包含小写字母、数字和连字符
  const bucketNameRegex = /^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$/;
  if (!bucketNameRegex.test(newBucketName.value)) {
    alert('存储桶名称只能包含小写字母、数字和连字符，长度为3-63个字符，且必须以字母或数字开头和结尾')
    return
  }
  
  try {
    const success = await bucketStore.createBucket(newBucketName.value)
    if (success) {
      showCreateDialog.value = false
      newBucketName.value = ''
      alert('存储桶创建成功')
    }
  } catch (error) {
    console.error('创建存储桶失败:', error)
    alert('创建存储桶失败: ' + (error as Error).message)
  }
}
</script>

<style scoped>
.bucket-list {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid #e0e0e0;
}

h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #333;
}

.create-btn {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 50%;
  background-color: #667eea;
  color: white;
  font-size: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.create-btn:hover {
  transform: scale(1.1);
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.4);
}

.loading {
  padding: 20px;
  text-align: center;
  color: #999;
}

.buckets {
  flex: 1;
  overflow-y: auto;
}

.bucket-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.bucket-item:hover {
  background-color: #f0f0f0;
}

.bucket-item.active {
  background-color: #e8eaf6;
  color: #667eea;
}

.bucket-icon {
  margin-right: 12px;
  font-size: 20px;
}

.bucket-name {
  flex: 1;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty {
  padding: 20px;
  text-align: center;
  color: #999;
  font-size: 14px;
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: white;
  border-radius: 8px;
  padding: 24px;
  min-width: 300px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
}

.dialog h4 {
  margin: 0 0 16px 0;
  font-size: 18px;
  color: #333;
}

.dialog input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  margin-bottom: 16px;
}

.dialog input:focus {
  outline: none;
  border-color: #667eea;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.dialog button {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.dialog button:first-child {
  background-color: #f5f5f5;
  color: #333;
}

.dialog button.primary {
  background-color: #667eea;
  color: white;
}

.dialog button:hover {
  opacity: 0.9;
}

@media (prefers-color-scheme: dark) {
  .header {
    border-bottom-color: #444;
  }

  h3 {
    color: #fff;
  }

  .bucket-item:hover {
    background-color: #333;
  }

  .bucket-item.active {
    background-color: #3a3a5a;
    color: #8b9eff;
  }

  .dialog {
    background: #2a2a2a;
  }

  .dialog h4 {
    color: #fff;
  }

  .dialog input {
    background-color: #333;
    border-color: #444;
    color: #fff;
  }

  .dialog button:first-child {
    background-color: #333;
    color: #fff;
  }
}
</style>
