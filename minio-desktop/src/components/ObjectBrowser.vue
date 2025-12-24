<template>
  <div class="object-browser">
    <div class="toolbar">
      <div class="breadcrumb">
        <span @click="navigateToRoot" class="breadcrumb-item">{{ currentBucket }}</span>
        <template v-for="(part, index) in pathParts" :key="index">
          <span class="separator">/</span>
          <span @click="navigateToPart(index)" class="breadcrumb-item">{{ part }}</span>
        </template>
      </div>
      <div class="actions">
        <button @click="refresh">刷新</button>
      </div>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="objects-list">
      <div
        v-for="obj in objects"
        :key="obj.key"
        class="object-item"
        @click="handleClick(obj)"
        @dblclick="handleDoubleClick(obj)"
      >
        <span class="icon">{{ obj.is_dir ? '📁' : getFileIcon(obj.key) }}</span>
        <div class="info">
          <div class="name">{{ getDisplayName(obj.key) }}</div>
          <div class="meta" v-if="!obj.is_dir">
            {{ formatSize(obj.size) }} · {{ formatDate(obj.last_modified) }}
          </div>
        </div>
        <div class="actions-menu" v-if="!obj.is_dir">
          <button @click.stop="handleDownload(obj)" class="download-btn">下载</button>
          <button @click.stop="handleDelete(obj)" class="delete-btn">删除</button>
        </div>
      </div>

      <div v-if="objects.length === 0" class="empty">
        此文件夹为空
      </div>
    </div>

    <MediaViewer
      :visible="showMediaViewer"
      :bucket="currentBucket || ''"
      :object-key="selectedObject?.key || ''"
      :file-name="selectedObject ? getDisplayName(selectedObject.key) : ''"
      @close="showMediaViewer = false"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useBucketStore } from '../stores/bucket'
import { useObjectStore } from '../stores/object'
import { api, type ObjectInfo } from '../api'
import MediaViewer from './MediaViewer.vue'

const bucketStore = useBucketStore()
const objectStore = useObjectStore()

const { currentBucket } = storeToRefs(bucketStore)
const { objects, currentPrefix, loading } = storeToRefs(objectStore)

const showMediaViewer = ref(false)
const selectedObject = ref<ObjectInfo | null>(null)

const pathParts = computed(() => {
  if (!currentPrefix.value) return []
  return currentPrefix.value.split('/').filter(p => p)
})

watch(currentBucket, async (newBucket) => {
  if (newBucket) {
    await objectStore.fetchObjects(newBucket)
  }
})

function getDisplayName(key: string): string {
  const parts = key.split('/')
  return parts[parts.length - 1] || parts[parts.length - 2]
}

// 修复文件路径显示问题 - 正确处理包含"/"的文件路径
function getFileIcon(key: string): string {
  // 检查是否是目录
  if (key.endsWith('/')) {
    return '📁'
  }
  
  // 获取文件扩展名
  const ext = key.split('.').pop()?.toLowerCase()
  const iconMap: Record<string, string> = {
    'jpg': '🖼',
    'jpeg': '🖼',
    'png': '🖼',
    'gif': '🖼',
    'webp': '🖼',
    'bmp': '🖼',
    'svg': '🖼',
    'mp4': '🎬',
    'webm': '🎬',
    'ogg': '🎬',
    'mov': '🎬',
    'avi': '🎬',
    'pdf': '📄',
    'doc': '📝',
    'docx': '📝',
    'txt': '📝',
    'zip': '📦',
    'rar': '📦',
  }
  return iconMap[ext || ''] || '📄'
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  return date.toLocaleDateString()
}

function navigateToRoot() {
  if (currentBucket.value) {
    objectStore.fetchObjects(currentBucket.value)
  }
}

function navigateToPart(index: number) {
  const prefix = pathParts.value.slice(0, index + 1).join('/') + '/'
  if (currentBucket.value) {
    objectStore.fetchObjects(currentBucket.value, prefix)
  }
}

function handleClick(obj: ObjectInfo) {
  // Single click - select
}

function handleDoubleClick(obj: ObjectInfo) {
  if (obj.is_dir) {
    // Navigate into directory
    if (currentBucket.value) {
      objectStore.fetchObjects(currentBucket.value, obj.key)
    }
  } else {
    // Open media viewer for preview
    const ext = obj.key.split('.').pop()?.toLowerCase()
    const previewableTypes = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'mp4', 'webm', 'ogg', 'mov']
    
    if (ext && previewableTypes.includes(ext)) {
      selectedObject.value = obj
      showMediaViewer.value = true
    } else {
      console.log('File type not previewable:', ext)
    }
  }
}

async function handleDelete(obj: ObjectInfo) {
  if (confirm(`确定要删除 "${getDisplayName(obj.key)}" 吗?`)) {
    if (currentBucket.value) {
      await objectStore.deleteObject(currentBucket.value, obj.key)
    }
  }
}

async function handleDownload(obj: ObjectInfo) {
  try {
    // 使用系统对话框选择保存路径
    const path = await import('@tauri-apps/api/dialog').then(dialog => 
      dialog.save({ 
        title: '保存文件',
        defaultPath: getDisplayName(obj.key)
      })
    );
    
    if (path) {
      // 调用下载函数
      await api.downloadFile(currentBucket.value!, obj.key, path);
      alert('下载任务已开始');
    }
  } catch (error) {
    console.error('下载失败:', error);
    alert('下载失败: ' + (error as Error).message);
  }
}

function refresh() {
  if (currentBucket.value) {
    objectStore.fetchObjects(currentBucket.value, currentPrefix.value || undefined)
  }
}
</script>

<style scoped>
.object-browser {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #e0e0e0;
  background-color: #fafafa;
}

.breadcrumb {
  display: flex;
  align-items: center;
  flex: 1;
  overflow-x: auto;
}

.breadcrumb-item {
  cursor: pointer;
  color: #667eea;
  white-space: nowrap;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.breadcrumb-item:hover {
  background-color: #e8eaf6;
}

.separator {
  margin: 0 4px;
  color: #999;
}

.actions button {
  padding: 6px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: white;
  cursor: pointer;
  font-size: 14px;
}

.actions button:hover {
  background-color: #f5f5f5;
}

.loading {
  padding: 40px;
  text-align: center;
  color: #999;
}

.objects-list {
  flex: 1;
  overflow-y: auto;
}

.object-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #f0f0f0;
  cursor: pointer;
  transition: background-color 0.2s;
}

.object-item:hover {
  background-color: #f8f8f8;
}

.object-item .icon {
  font-size: 24px;
  margin-right: 12px;
}

.object-item .info {
  flex: 1;
  min-width: 0;
}

.object-item .name {
  font-size: 14px;
  color: #333;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.object-item .meta {
  font-size: 12px;
  color: #999;
  margin-top: 2px;
}

.actions-menu {
  opacity: 0;
  transition: opacity 0.2s;
}

.object-item:hover .actions-menu {
  opacity: 1;
}

.download-btn {
  padding: 4px 12px;
  border: 1px solid #667eea;
  border-radius: 4px;
  background-color: white;
  color: #667eea;
  cursor: pointer;
  font-size: 12px;
  margin-right: 8px;
}

.delete-btn {
  padding: 4px 12px;
  border: 1px solid #e74c3c;
  border-radius: 4px;
  background-color: white;
  color: #e74c3c;
  cursor: pointer;
  font-size: 12px;
}

.delete-btn:hover {
  background-color: #e74c3c;
  color: white;
}

.empty {
  padding: 40px;
  text-align: center;
  color: #999;
  font-size: 14px;
}

@media (prefers-color-scheme: dark) {
  .toolbar {
    border-bottom-color: #444;
    background-color: #252525;
  }

  .breadcrumb-item:hover {
    background-color: #3a3a5a;
  }

  .actions button {
    border-color: #444;
    background-color: #2a2a2a;
    color: #fff;
  }

  .actions button:hover {
    background-color: #333;
  }

  .object-item {
    border-bottom-color: #333;
  }

  .object-item:hover {
    background-color: #252525;
  }

  .object-item .name {
    color: #fff;
  }

  .delete-btn {
    background-color: #2a2a2a;
  }

  .delete-btn:hover {
    background-color: #e74c3c;
  }
}
</style>