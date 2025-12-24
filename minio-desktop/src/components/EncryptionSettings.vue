<template>
  <div class="encryption-settings">
    <div class="header">
      <h3>SSE-C 加密设置</h3>
      <div class="status" :class="{ enabled: hasKey }">
        {{ hasKey ? '已启用' : '未启用' }}
      </div>
    </div>

    <div class="content">
      <div v-if="!hasKey" class="setup">
        <p class="description">
          SSE-C (Server-Side Encryption with Customer-Provided Keys) 
          允许您使用自己的加密密钥来加密上传到 MinIO 的对象。
        </p>

        <div class="key-input-section">
          <h4>密钥配置</h4>
          
          <div class="form-group">
            <label>加密密钥 (Base64, 32字节)</label>
            <div class="input-with-button">
              <input
                v-model="keyInput"
                type="text"
                placeholder="粘贴或生成加密密钥"
                @input="validateKey"
              />
              <button @click="generateKey" class="generate-btn">
                生成随机密钥
              </button>
            </div>
            <span v-if="keyValidation" class="validation-message" :class="{ error: !keyValidation.valid }">
              {{ keyValidation.message }}
            </span>
          </div>

          <div class="actions">
            <button @click="saveKey" :disabled="!isKeyValid || saving">
              {{ saving ? '保存中...' : '保存密钥' }}
            </button>
          </div>

          <div class="warning">
            ⚠️ 请妥善保管您的加密密钥。丢失密钥将无法解密已加密的对象。
          </div>
        </div>
      </div>

      <div v-else class="key-info">
        <div class="info-item">
          <label>密钥 ID</label>
          <span>{{ currentKey?.key_id }}</span>
        </div>
        <div class="info-item">
          <label>创建时间</label>
          <span>{{ formatDate(currentKey?.created_at) }}</span>
        </div>
        <div class="info-item">
          <label>状态</label>
          <span class="status-badge enabled">已启用</span>
        </div>

        <div class="actions">
          <button @click="exportKey" class="secondary">导出密钥</button>
          <button @click="showRemoveConfirm = true" class="danger">移除密钥</button>
        </div>
      </div>
    </div>

    <!-- Remove confirmation dialog -->
    <div v-if="showRemoveConfirm" class="dialog-overlay" @click="showRemoveConfirm = false">
      <div class="dialog" @click.stop>
        <h4>确认移除密钥</h4>
        <p>移除密钥后,将无法访问使用此密钥加密的对象。确定要继续吗?</p>
        <div class="dialog-actions">
          <button @click="showRemoveConfirm = false">取消</button>
          <button @click="removeKey" class="danger">确认移除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

interface EncryptionKey {
  key_id: string
  key_value: string
  key_md5: string
  enabled: boolean
  created_at: string
}

const keyInput = ref('')
const keyValidation = ref<{ valid: boolean; message: string } | null>(null)
const currentKey = ref<EncryptionKey | null>(null)
const saving = ref(false)
const showRemoveConfirm = ref(false)

const hasKey = computed(() => currentKey.value !== null)
const isKeyValid = computed(() => keyValidation.value?.valid === true)

onMounted(async () => {
  await loadKey()
})

async function loadKey() {
  try {
    currentKey.value = await invoke<EncryptionKey | null>('get_encryption_key')
  } catch (error) {
    console.error('Failed to load encryption key:', error)
  }
}

async function generateKey() {
  try {
    keyInput.value = await invoke<string>('generate_encryption_key')
    await validateKey()
  } catch (error) {
    console.error('Failed to generate key:', error)
    alert('生成密钥失败: ' + error)
  }
}

async function validateKey() {
  if (!keyInput.value) {
    keyValidation.value = null
    return
  }

  try {
    const valid = await invoke<boolean>('validate_encryption_key', {
      keyBase64: keyInput.value
    })
    
    keyValidation.value = valid
      ? { valid: true, message: '✓ 密钥格式正确' }
      : { valid: false, message: '✗ 密钥格式无效' }
  } catch (error) {
    keyValidation.value = {
      valid: false,
      message: '✗ ' + error
    }
  }
}

async function saveKey() {
  if (!isKeyValid.value) return

  saving.value = true
  try {
    await invoke('save_encryption_key', {
      keyBase64: keyInput.value
    })
    
    await loadKey()
    keyInput.value = ''
    keyValidation.value = null
    alert('密钥保存成功!')
  } catch (error) {
    console.error('Failed to save key:', error)
    alert('保存密钥失败: ' + error)
  } finally {
    saving.value = false
  }
}

function exportKey() {
  if (!currentKey.value) return
  
  const blob = new Blob([currentKey.value.key_value], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `sse-c-key-${currentKey.value.key_id}.txt`
  a.click()
  URL.revokeObjectURL(url)
}

async function removeKey() {
  // Note: This would need a backend command to disable/remove the key
  showRemoveConfirm.value = false
  alert('移除密钥功能待实现')
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  return date.toLocaleString()
}
</script>

<style scoped>
.encryption-settings {
  padding: 20px;
  max-width: 800px;
  margin: 0 auto;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  padding-bottom: 16px;
  border-bottom: 2px solid #e0e0e0;
}

.header h3 {
  margin: 0;
  font-size: 20px;
  color: #333;
}

.status {
  padding: 6px 16px;
  border-radius: 20px;
  font-size: 14px;
  font-weight: 500;
  background-color: #f0f0f0;
  color: #666;
}

.status.enabled {
  background-color: #e8f5e9;
  color: #2e7d32;
}

.description {
  color: #666;
  line-height: 1.6;
  margin-bottom: 24px;
}

.key-input-section {
  background-color: #f8f8f8;
  padding: 20px;
  border-radius: 8px;
}

.key-input-section h4 {
  margin: 0 0 16px 0;
  font-size: 16px;
  color: #333;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
  color: #555;
  font-size: 14px;
}

.input-with-button {
  display: flex;
  gap: 12px;
}

.input-with-button input {
  flex: 1;
  padding: 10px 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
  font-family: monospace;
}

.generate-btn {
  padding: 10px 16px;
  border: none;
  border-radius: 6px;
  background-color: #667eea;
  color: white;
  font-size: 14px;
  cursor: pointer;
  white-space: nowrap;
}

.generate-btn:hover {
  background-color: #5568d3;
}

.validation-message {
  display: block;
  margin-top: 8px;
  font-size: 13px;
  color: #2e7d32;
}

.validation-message.error {
  color: #c62828;
}

.actions {
  margin-top: 20px;
  display: flex;
  gap: 12px;
}

.actions button {
  padding: 10px 24px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.actions button:first-child {
  background-color: #667eea;
  color: white;
}

.actions button:first-child:hover:not(:disabled) {
  background-color: #5568d3;
}

.actions button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.actions button.secondary {
  background-color: #f5f5f5;
  color: #333;
}

.actions button.secondary:hover {
  background-color: #e0e0e0;
}

.actions button.danger {
  background-color: #f44336;
  color: white;
}

.actions button.danger:hover {
  background-color: #d32f2f;
}

.warning {
  margin-top: 16px;
  padding: 12px;
  background-color: #fff3cd;
  border-left: 4px solid #ff9800;
  border-radius: 4px;
  font-size: 14px;
  color: #856404;
}

.key-info {
  background-color: #f8f8f8;
  padding: 20px;
  border-radius: 8px;
}

.info-item {
  display: flex;
  justify-content: space-between;
  padding: 12px 0;
  border-bottom: 1px solid #e0e0e0;
}

.info-item:last-of-type {
  border-bottom: none;
}

.info-item label {
  font-weight: 500;
  color: #555;
}

.info-item span {
  color: #333;
  font-family: monospace;
  font-size: 14px;
}

.status-badge {
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.status-badge.enabled {
  background-color: #e8f5e9;
  color: #2e7d32;
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
  min-width: 400px;
  max-width: 500px;
}

.dialog h4 {
  margin: 0 0 16px 0;
  font-size: 18px;
  color: #333;
}

.dialog p {
  margin: 0 0 20px 0;
  color: #666;
  line-height: 1.6;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.dialog-actions button {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
}

.dialog-actions button:first-child {
  background-color: #f5f5f5;
  color: #333;
}

.dialog-actions button.danger {
  background-color: #f44336;
  color: white;
}

@media (prefers-color-scheme: dark) {
  .header {
    border-bottom-color: #444;
  }

  .header h3 {
    color: #fff;
  }

  .key-input-section,
  .key-info {
    background-color: #2a2a2a;
  }

  .key-input-section h4 {
    color: #fff;
  }

  .form-group label {
    color: #ccc;
  }

  .input-with-button input {
    background-color: #333;
    border-color: #444;
    color: #fff;
  }

  .info-item {
    border-bottom-color: #444;
  }

  .info-item label {
    color: #ccc;
  }

  .info-item span {
    color: #fff;
  }

  .dialog {
    background: #2a2a2a;
  }

  .dialog h4 {
    color: #fff;
  }

  .dialog p {
    color: #ccc;
  }

  .dialog-actions button:first-child {
    background-color: #333;
    color: #fff;
  }
}
</style>
