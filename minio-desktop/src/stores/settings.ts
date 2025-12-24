import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useSettingsStore = defineStore('settings', () => {
  const showDeleteButton = ref(true)

  // 从本地存储加载设置
  function loadSettings() {
    const saved = localStorage.getItem('app-settings')
    if (saved) {
      try {
        const settings = JSON.parse(saved)
        showDeleteButton.value = settings.showDeleteButton ?? true
      } catch (e) {
        console.error('Failed to load settings:', e)
        // 使用默认值
        showDeleteButton.value = true
      }
    }
  }

  // 保存设置到本地存储
  function saveSettings() {
    const settings = {
      showDeleteButton: showDeleteButton.value
    }
    localStorage.setItem('app-settings', JSON.stringify(settings))
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
    toggleDeleteButton,
    saveSettings,
    loadSettings
  }
})