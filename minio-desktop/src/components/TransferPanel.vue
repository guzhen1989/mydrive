<template>
  <div class="transfer-panel">
    <div class="header">
      <div class="header-left">
        <h4>传输任务</h4>
        <span class="count">{{ tasks.length }}</span>
      </div>
      <div class="header-actions">
        <button 
          v-if="hasCompletedTasks" 
          @click="deleteCompletedTasks" 
          class="delete-btn"
          title="清空已完成、已失败和已取消的任务"
        >
          清空已完成
        </button>
        <button 
          v-if="hasActiveTasks" 
          @click="cancelAllTasks" 
          class="cancel-all-btn"
        >
          取消所有任务
        </button>
      </div>
    </div>

    <div class="tasks-list">
      <div
        v-for="task in tasks"
        :key="task.task_id"
        class="task-item"
      >
        <div class="task-icon">
          {{ task.task_type === 'upload' ? '⬆️' : '⬇️' }}
        </div>
        <div class="task-info">
          <div class="task-name">{{ task.file_name }}</div>
          <div class="task-progress">
            <div class="progress-bar">
              <div
                class="progress-fill"
                :style="{ width: getProgress(task) + '%' }"
                :class="getStatusClass(task.status)"
              ></div>
            </div>
            <div class="progress-text">
              {{ formatBytes(task.transferred_bytes) }} / {{ formatBytes(task.file_size) }}
              ({{ getProgress(task) }}%)
            </div>
          </div>
          <div class="task-status" :class="'status-' + task.status">
            {{ getStatusText(task.status) }}
          </div>
        </div>
        <div class="task-actions">
          <button
            v-if="task.status === 'running'"
            @click="pauseTask(task.task_id)"
            class="action-btn"
          >
            暂停
          </button>
          <button
            v-else-if="task.status === 'paused'"
            @click="resumeTask(task.task_id)"
            class="action-btn"
          >
            继续
          </button>
          <button
            v-if="['running', 'paused', 'pending'].includes(task.status)"
            @click="cancelTask(task.task_id)"
            class="action-btn cancel"
          >
            取消
          </button>
          <button
            v-if="['completed', 'failed', 'cancelled'].includes(task.status)"
            @click="deleteTask(task.task_id)"
            class="action-btn delete"
            title="删除此任务"
          >
            删除
          </button>
        </div>
      </div>

      <div v-if="tasks.length === 0" class="empty">
        暂无传输任务
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { api, type TransferTask } from '../api'

const tasks = ref<TransferTask[]>([])

const hasActiveTasks = computed(() => {
  return tasks.value.some(task => 
    ['running', 'paused', 'pending'].includes(task.status)
  )
})

const hasCompletedTasks = computed(() => {
  return tasks.value.some(task => 
    ['completed', 'failed', 'cancelled'].includes(task.status)
  )
})

onMounted(async () => {
  await loadTasks()
  // 设置定时器定期更新任务状态
  const interval = setInterval(async () => {
    await loadTasks()
  }, 1000) // 每秒更新一次
  
  // 在组件卸载时清除定时器
  onUnmounted(() => {
    clearInterval(interval)
  })
})

async function loadTasks() {
  try {
    tasks.value = await api.getTransferTasks()
  } catch (e) {
    console.error('Failed to load tasks:', e)
  }
}

function getProgress(task: TransferTask): number {
  if (task.file_size === 0) return 0
  return Math.round((task.transferred_bytes / task.file_size) * 100)
}

function getStatusClass(status: string): string {
  return `status-${status}`
}

function getStatusText(status: string): string {
  const statusMap: Record<string, string> = {
    pending: '等待中',
    running: '进行中',
    paused: '已暂停',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消'
  }
  return statusMap[status] || status
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}

async function pauseTask(taskId: string) {
  try {
    await api.pauseTask(taskId)
    await loadTasks()
  } catch (e) {
    console.error('Failed to pause task:', e)
  }
}

async function resumeTask(taskId: string) {
  try {
    await api.resumeTask(taskId)
    await loadTasks()
  } catch (e) {
    console.error('Failed to resume task:', e)
  }
}

async function cancelTask(taskId: string) {
  if (confirm('确定要取消此任务吗?')) {
    try {
      await api.cancelTask(taskId)
      await loadTasks()
    } catch (e) {
      console.error('Failed to cancel task:', e)
    }
  }
}

async function cancelAllTasks() {
  if (confirm('确定要取消所有进行中的任务吗?')) {
    try {
      await api.cancelAllTasks()
      await loadTasks()
    } catch (e) {
      console.error('Failed to cancel all tasks:', e)
      alert('取消任务失败: ' + (e as Error).message)
    }
  }
}

async function deleteTask(taskId: string) {
  try {
    await api.deleteTask(taskId)
    await loadTasks()
  } catch (e) {
    console.error('Failed to delete task:', e)
    alert('删除任务失败: ' + (e as Error).message)
  }
}

async function deleteCompletedTasks() {
  if (confirm('确定要清空所有已完成、已失败和已取消的任务吗?')) {
    try {
      await api.deleteCompletedTasks()
      await loadTasks()
    } catch (e) {
      console.error('Failed to delete completed tasks:', e)
      alert('清空任务失败: ' + (e as Error).message)
    }
  }
}
</script>

<style scoped>
.transfer-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #e0e0e0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-actions {
  display: flex;
  gap: 8px;
}

h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.count {
  background-color: #667eea;
  color: white;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
}

.cancel-all-btn {
  padding: 6px 16px;
  border: 1px solid #e74c3c;
  border-radius: 6px;
  background-color: white;
  color: #e74c3c;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
  font-weight: 500;
}

.cancel-all-btn:hover {
  background-color: #e74c3c;
  color: white;
}

.delete-btn {
  padding: 6px 16px;
  border: 1px solid #95a5a6;
  border-radius: 6px;
  background-color: white;
  color: #7f8c8d;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
  font-weight: 500;
}

.delete-btn:hover {
  background-color: #95a5a6;
  color: white;
}

.tasks-list {
  flex: 1;
  overflow-y: auto;
}

.task-item {
  display: flex;
  align-items: flex-start;
  padding: 12px 16px;
  border-bottom: 1px solid #f0f0f0;
  gap: 12px;
}

.task-icon {
  font-size: 24px;
  flex-shrink: 0;
}

.task-info {
  flex: 1;
  min-width: 0;
}

.task-name {
  font-size: 14px;
  font-weight: 500;
  color: #333;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-bottom: 8px;
}

.task-progress {
  margin-bottom: 4px;
}

.progress-bar {
  height: 6px;
  background-color: #e0e0e0;
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: 4px;
}

.progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

.progress-fill.status-running {
  background-color: #667eea;
}

.progress-fill.status-completed {
  background-color: #4caf50;
}

.progress-fill.status-failed {
  background-color: #e74c3c;
}

.progress-fill.status-paused {
  background-color: #ff9800;
}

.progress-text {
  font-size: 12px;
  color: #999;
}

.task-status {
  font-size: 12px;
  font-weight: 500;
  margin-top: 4px;
}

.task-status.status-running {
  color: #667eea;
}

.task-status.status-completed {
  color: #4caf50;
}

.task-status.status-failed {
  color: #e74c3c;
}

.task-status.status-paused {
  color: #ff9800;
}

.task-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.action-btn {
  padding: 4px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: white;
  color: #667eea;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

.action-btn:hover {
  background-color: #f5f5f5;
}

.action-btn.cancel {
  color: #e74c3c;
  border-color: #e74c3c;
}

.action-btn.cancel:hover {
  background-color: #e74c3c;
  color: white;
}

.action-btn.delete {
  color: #95a5a6;
  border-color: #95a5a6;
}

.action-btn.delete:hover {
  background-color: #95a5a6;
  color: white;
}

.empty {
  padding: 40px;
  text-align: center;
  color: #999;
  font-size: 14px;
}

@media (prefers-color-scheme: dark) {
  .header {
    border-bottom-color: #444;
  }

  h4 {
    color: #fff;
  }

  .task-item {
    border-bottom-color: #333;
  }

  .task-name {
    color: #fff;
  }

  .progress-bar {
    background-color: #444;
  }

  .action-btn {
    border-color: #444;
    background-color: #2a2a2a;
  }

  .action-btn:hover {
    background-color: #333;
  }

  .action-btn.cancel {
    background-color: #2a2a2a;
  }
}
</style>
