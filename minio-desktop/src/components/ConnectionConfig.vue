<template>
  <div class="connection-config">
    <div class="config-card">
      <h2>连接到 MinIO</h2>
      <form @submit.prevent="handleConnect">
        <div class="form-group">
          <label>服务器地址</label>
          <input
            v-model="form.endpoint"
            type="text"
            placeholder="例如: localhost 或 minio.example.com"
            required
          />
        </div>

        <div class="form-group">
          <label>端口</label>
          <input
            v-model.number="form.port"
            type="number"
            placeholder="9000"
            required
          />
        </div>

        <div class="form-group">
          <label>Access Key</label>
          <input
            v-model="form.accessKey"
            type="text"
            placeholder="访问密钥"
            required
          />
        </div>

        <div class="form-group">
          <label>Secret Key</label>
          <input
            v-model="form.secretKey"
            type="password"
            placeholder="私密密钥"
            required
          />
        </div>

        <div class="form-group checkbox">
          <label>
            <input v-model="form.useSsl" type="checkbox" />
            使用 SSL/TLS
          </label>
        </div>

        <div class="form-group checkbox">
          <label>
            <input v-model="form.showDeleteButton" type="checkbox" />
            显示删除按钮
          </label>
        </div>

        <div v-if="error" class="error-message">
          {{ error }}
        </div>

        <div class="form-actions">
          <button type="button" @click="handleTest" :disabled="loading">
            {{ loading ? '测试中...' : '测试连接' }}
          </button>
          <button type="submit" :disabled="loading">
            {{ loading ? '连接中...' : '连接' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useConnectionStore } from '../stores/connection'
import { useBucketStore } from '../stores/bucket'
import { useSettingsStore } from '../stores/settings'

const connectionStore = useConnectionStore()
const bucketStore = useBucketStore()
const settingsStore = useSettingsStore()

const form = ref({
  endpoint: '192.168.3.92',
  port: 39000,
  accessKey: 'minioadmin',
  secretKey: 'minioadmin',
  useSsl: false,
  showDeleteButton: false
})

const loading = ref(false)
const error = ref<string | null>(null)

async function handleTest() {
  loading.value = true
  error.value = null
  const success = await connectionStore.testConnection(form.value)
  loading.value = false
  
  if (success) {
    alert('连接测试成功!')
  } else {
    error.value = connectionStore.error || '连接测试失败'
  }
}

async function handleConnect() {
  loading.value = true
  error.value = null
  
  // 保存删除按钮显示设置
  settingsStore.showDeleteButton = form.value.showDeleteButton
  settingsStore.saveSettings()
  
  const success = await connectionStore.saveConnection(form.value)
  loading.value = false
  
  if (success) {
    await bucketStore.fetchBuckets()
  } else {
    error.value = connectionStore.error || '连接失败'
  }
}
</script>

<style scoped>
.connection-config {
  padding: 40px;
  max-width: 500px;
  width: 100%;
}

.config-card {
  background: white;
  border-radius: 12px;
  padding: 32px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.1);
}

h2 {
  margin: 0 0 24px 0;
  color: #333;
  font-size: 24px;
  font-weight: 600;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  color: #555;
  font-weight: 500;
  font-size: 14px;
}

.form-group input[type="text"],
.form-group input[type="password"],
.form-group input[type="number"] {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  transition: border-color 0.2s;
}

.form-group input:focus {
  outline: none;
  border-color: #667eea;
}

.form-group.checkbox label {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.form-group.checkbox input {
  margin-right: 8px;
  cursor: pointer;
}

.error-message {
  padding: 12px;
  background-color: #fee;
  color: #c33;
  border-radius: 6px;
  margin-bottom: 20px;
  font-size: 14px;
}

.form-actions {
  display: flex;
  gap: 12px;
}

button {
  flex: 1;
  padding: 12px 24px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

button[type="button"] {
  background-color: #f5f5f5;
  color: #333;
}

button[type="button"]:hover:not(:disabled) {
  background-color: #e0e0e0;
}

button[type="submit"] {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

button[type="submit"]:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (prefers-color-scheme: dark) {
  .config-card {
    background: #2a2a2a;
  }

  h2 {
    color: #fff;
  }

  .form-group label {
    color: #ccc;
  }

  .form-group input[type="text"],
  .form-group input[type="password"],
  .form-group input[type="number"] {
    background-color: #333;
    border-color: #444;
    color: #fff;
  }

  button[type="button"] {
    background-color: #333;
    color: #fff;
  }

  button[type="button"]:hover:not(:disabled) {
    background-color: #444;
  }
}
</style>
