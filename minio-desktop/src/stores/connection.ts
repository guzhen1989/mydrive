import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type ConnectionConfig } from '../api'

export const useConnectionStore = defineStore('connection', () => {
  const connected = ref(false)
  const config = ref<ConnectionConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function testConnection(newConfig: Omit<ConnectionConfig, 'lastConnected'>) {
    loading.value = true
    error.value = null
    try {
      await api.testConnection(newConfig)
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      loading.value = false
    }
  }

  async function saveConnection(newConfig: Omit<ConnectionConfig, 'lastConnected'>) {
    loading.value = true
    error.value = null
    try {
      await api.saveConnection(newConfig)
      config.value = { ...newConfig, lastConnected: new Date().toISOString() }
      connected.value = true
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      loading.value = false
    }
  }

  async function loadConnection() {
    try {
      const savedConfig = await api.getConnection()
      if (savedConfig) {
        config.value = savedConfig
        connected.value = true
      }
    } catch (e) {
      console.error('Failed to load connection:', e)
    }
  }

  function disconnect() {
    connected.value = false
    config.value = null
  }

  return {
    connected,
    config,
    loading,
    error,
    testConnection,
    saveConnection,
    loadConnection,
    disconnect
  }
})
