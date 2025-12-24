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
        <div class="upload-dropdown">
          <button @click="toggleUploadMenu" class="upload-btn">
            上传 ▼
          </button>
          <div v-if="showUploadMenu" class="upload-menu">
            <div @click="handleUploadFiles" class="menu-item">上传文件</div>
            <div @click="handleUploadFolder" class="menu-item">上传文件夹</div>
            <div class="menu-divider"></div>
            <div @click="toggleEncryption" class="menu-item encryption-toggle">
              <span>🔒 {{ settingsStore.enableEncryption ? '禁用加密' : '启用加密' }}</span>
              <span v-if="settingsStore.enableEncryption" class="enabled-badge">✓</span>
            </div>
          </div>
        </div>
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
          <button v-if="settingsStore.showDeleteButton" @click.stop="handleDeleteWithoutConfirm(obj)" class="delete-btn">删除</button>
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
import { computed, watch, ref, onMounted, onUnmounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useBucketStore } from '../stores/bucket'
import { useObjectStore } from '../stores/object'
import { useSettingsStore } from '../stores/settings'
import { api, type ObjectInfo } from '../api'
import MediaViewer from './MediaViewer.vue'

const bucketStore = useBucketStore()
const objectStore = useObjectStore()
const settingsStore = useSettingsStore()

const { currentBucket } = storeToRefs(bucketStore)
const { objects, currentPrefix, loading } = storeToRefs(objectStore)

const showMediaViewer = ref(false)
const selectedObject = ref<ObjectInfo | null>(null)
const showUploadMenu = ref(false)

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
    try {
      if (currentBucket.value) {
        await objectStore.deleteObject(currentBucket.value, obj.key)
        alert('文件已成功删除')
      }
    } catch (error) {
      console.error('删除文件失败:', error)
      alert('删除文件失败: ' + (error as Error).message)
    }
  } else {
    // 用户点击了取消
    console.log('删除操作已取消')
  }
}

async function handleDeleteWithoutConfirm(obj: ObjectInfo) {
  try {
    if (currentBucket.value) {
      await objectStore.deleteObject(currentBucket.value, obj.key)
    }
  } catch (error) {
    console.error('删除文件失败:', error)
    alert('删除文件失败: ' + (error as Error).message)
  }
}



function toggleUploadMenu() {
  showUploadMenu.value = !showUploadMenu.value
}

async function toggleEncryption() {
  try {
    if (!settingsStore.enableEncryption) {
      // 尝试启用加密，首先检查是否有密钥
      const { invoke } = await import('@tauri-apps/api/tauri')
      const encryptionKey = await invoke('get_encryption_key')
      
      if (!encryptionKey || !(encryptionKey as any).key_value) {
        alert('请先在加密设置中配置加密密钥')
        showUploadMenu.value = false
        return
      }
    }
    
    // 使用 store 的 toggle 方法来切换加密状态
    await settingsStore.toggleEncryption()
    showUploadMenu.value = false
  } catch (error) {
    console.error('Failed to toggle encryption:', error)
    alert('切换加密状态失败: ' + error)
    showUploadMenu.value = false
  }
}

// 点击外部关闭菜单
function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (!target.closest('.upload-dropdown')) {
    showUploadMenu.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})

async function handleUploadFiles() {
  showUploadMenu.value = false
  await performUpload(false)
}

async function handleUploadFolder() {
  showUploadMenu.value = false
  await performUpload(true)
}

async function performUpload(isDirectory: boolean) {
  try {
    // 使用系统文件选择对话框
    const selected = await import('@tauri-apps/api/dialog').then(dialog => 
      dialog.open({
        title: isDirectory ? '选择文件夹上传' : '选择文件上传',
        multiple: true, // 允许选择多个
        directory: isDirectory // 根据用户选择决定是否只能选择文件夹
      })
    );
    
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      
      for (const path of paths) {
        // 检查路径是否为文件夹
        if (await checkIsDirectory(path)) {
          // 如果是文件夹，递归上传文件夹内容
          await uploadFolder(path);
        } else {
          // 如果是文件，直接上传
          const fileName = path.split('/').pop() || path.split('\\').pop() || 'unknown';
          const objectKey = currentPrefix.value ? `${currentPrefix.value}${fileName}` : fileName;
          
          try {
            // 调用API上传文件
            const taskId = await api.uploadFile(
              path, 
              currentBucket.value!, 
              objectKey,
              settingsStore.enableEncryption,
              settingsStore.enableEncryption ? settingsStore.encryptionKey : undefined
            );
            console.log('上传任务已启动，任务ID:', taskId);
            
            // 等待上传完成，定期检查任务状态
            await waitForUploadCompletion(taskId);
            
          } catch (error) {
            console.error('文件上传失败:', error);
            alert(`文件上传失败: ${fileName} - ${(error as Error).message}`);
          }
        }
      }
      
      // 上传完成后刷新列表
      refresh();
      alert(`成功上传 ${paths.length} 个项目`);
    }
  } catch (error) {
    console.error('选择文件失败:', error);
    alert('选择文件失败: ' + (error as Error).message);
  }
}

// 检查路径是否为文件夹
async function checkIsDirectory(path: string): Promise<boolean> {
  try {
    // 尝试导入fs插件检查路径类型
    const { readDir } = await import('@tauri-apps/api/fs');
    try {
      // 尝试读取目录，如果成功则为文件夹
      await readDir(path);
      return true;
    } catch (e) {
      // 如果读取目录失败，则为文件
      return false;
    }
  } catch (e) {
    // 如果无法使用fs插件，则通过路径格式判断
    return path.endsWith('/') || path.endsWith('\\');
  }
}

// 上传整个文件夹
async function uploadFolder(folderPath: string) {
  try {
    // 导入fs插件来处理文件夹
    const { readDir } = await import('@tauri-apps/api/fs');
    
    // 递归读取文件夹内容
    const entries = await readDirRecursive(folderPath);
    
    for (const entry of entries) {
      if (entry.kind === 'file') { // 只处理文件
        // 计算相对于根文件夹的路径
        const relativePath = entry.path.replace(folderPath, '').replace(/^[/\\]/, '');
        const fullObjectKey = currentPrefix.value ? `${currentPrefix.value}${relativePath}` : relativePath;
        
        try {
          const taskId = await api.uploadFile(
            entry.path, 
            currentBucket.value!, 
            fullObjectKey,
            settingsStore.enableEncryption,
            settingsStore.enableEncryption ? settingsStore.encryptionKey : undefined
          );
          console.log('上传任务已启动，任务ID:', taskId, '路径:', fullObjectKey);
          
          // 等待上传完成，定期检查任务状态
          await waitForUploadCompletion(taskId);
        } catch (error) {
          console.error('文件上传失败:', entry.path, error);
          alert(`文件上传失败: ${entry.path} - ${(error as Error).message}`);
        }
      }
    }
  } catch (error) {
    console.error('上传文件夹失败:', error);
    alert('上传文件夹失败: ' + (error as Error).message);
  }
}

// 递归读取目录
async function readDirRecursive(dirPath: string): Promise<{path: string, kind: 'file' | 'dir'}[]> {
  const result = [];
  try {
    const { readDir } = await import('@tauri-apps/api/fs');
    
    const entries = await readDir(dirPath, { recursive: false });
    
    for (const entry of entries) {
      if (entry.children) { // 是文件夹
        const subEntries = await readDirRecursive(entry.path);
        result.push(...subEntries);
      } else { // 是文件
        result.push({ path: entry.path, kind: 'file' as const });
      }
    }
  } catch (error) {
    console.error('读取目录失败:', dirPath, error);
  }
  
  return result;
}


// 等待上传任务完成
async function waitForUploadCompletion(taskId: string) {
  const maxWaitTime = 300000; // 最大等待时间5分钟
  const checkInterval = 1000; // 检查间隔1秒
  const startTime = Date.now();
  
  while (Date.now() - startTime < maxWaitTime) {
    try {
      const tasks = await api.getTransferTasks();
      const task = tasks.find(t => t.task_id === taskId);
      
      if (!task) {
        console.error('未找到任务:', taskId);
        throw new Error('未找到上传任务');
      }
      
      if (task.status === 'completed') {
        console.log('上传任务完成:', taskId);
        return;
      } else if (task.status === 'failed') {
        console.error('上传任务失败:', taskId, task.error_message);
        throw new Error(task.error_message || '上传任务失败');
      } else if (task.status === 'cancelled') {
        console.error('上传任务已取消:', taskId);
        throw new Error('上传任务已取消');
      }
      
      // 等待一段时间再检查
      await new Promise(resolve => setTimeout(resolve, checkInterval));
      
    } catch (error) {
      console.error('检查上传任务状态失败:', error);
      throw error;
    }
  }
  
  throw new Error('上传任务超时');
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

.upload-btn {
  margin-right: 8px;
  background-color: #4CAF50;
  color: white;
  border: 1px solid #4CAF50;
}

.upload-btn:hover {
  background-color: #45a049;
}

.upload-dropdown {
  position: relative;
  display: inline-block;
  margin-right: 8px;
}

.upload-menu {
  position: absolute;
  top: 100%;
  left: 0;
  background-color: white;
  border: 1px solid #ddd;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  z-index: 1000;
  min-width: 120px;
  margin-top: 4px;
}

.menu-item {
  padding: 8px 16px;
  cursor: pointer;
  transition: background-color 0.2s;
  white-space: nowrap;
}

.menu-item:hover {
  background-color: #f5f5f5;
}

.menu-item:first-child {
  border-radius: 4px 4px 0 0;
}

.menu-item:last-child {
  border-radius: 0 0 4px 4px;
}

.menu-divider {
  height: 1px;
  background-color: #e0e0e0;
  margin: 4px 0;
}

.encryption-toggle {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.enabled-badge {
  color: #4CAF50;
  font-weight: bold;
  font-size: 16px;
}

@media (prefers-color-scheme: dark) {
  .upload-btn {
    background-color: #4CAF50;
    border-color: #4CAF50;
  }
  
  .upload-menu {
    background-color: #2a2a2a;
    border-color: #444;
  }
  
  .menu-item {
    color: #fff;
  }
  
  .menu-item:hover {
    background-color: #333;
  }
  
  .menu-divider {
    background-color: #444;
  }
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