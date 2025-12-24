import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

export const useSettingsStore = defineStore('settings', () => {
  const showDeleteButton = ref(true)
  const enableEncryption = ref(false)
  const encryptionKey = ref('')

  // 从本地存储加载设置
  async function loadSettings() {
    // 从数据库加载加密密钥和状态
    try {
      const key = await invoke('get_encryption_key')
      if (key) {
        encryptionKey.value = (key as any).key_value
        enableEncryption.value = (key as any).enabled
      } else {
        enableEncryption.value = false
      }
    } catch (e) {
      console.error('Failed to load encryption key from database:', e)
      enableEncryption.value = false
    }
    
    // 从本地存储加载其他设置
    const saved = localStorage.getItem('app-settings')
    if (saved) {
      try {
        const settings = JSON.parse(saved)
        showDeleteButton.value = settings.showDeleteButton ?? true
      } catch (e) {
        console.error('Failed to load settings:', e)
        showDeleteButton.value = true
      }
    }
  }

  // 保存设置到本地存储
  function saveSettings() {
    const settings = {
      showDeleteButton: showDeleteButton.value
      // 不保存enableEncryption和encryptionKey到localStorage，它们在数据库中
    }
    localStorage.setItem('app-settings', JSON.stringify(settings))
  }
  
  // 切换加密功能
  async function toggleEncryption() {
    try {
      await invoke('set_encryption_enabled', { enabled: !enableEncryption.value })
      enableEncryption.value = !enableEncryption.value
      console.log('Encryption toggled:', enableEncryption.value)
    } catch (e) {
      console.error('Failed to toggle encryption:', e)
      throw e
    }
  }

  // 切换删除按钮显示状态
  function toggleDeleteButton() {
    showDeleteButton.value = !showDeleteButton.value
    saveSettings()
  }

  // 初始化时加载设置
  loadSettings()

  return {
    showDeleteButton,
    enableEncryption,
    encryptionKey,
    toggleDeleteButton,
    toggleEncryption,
    saveSettings,
    loadSettings
  }
})