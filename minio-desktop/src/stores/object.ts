import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type ObjectInfo } from '../api'

export const useObjectStore = defineStore('object', () => {
  const objects = ref<ObjectInfo[]>([])
  const currentPrefix = ref<string>('')
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchObjects(bucket: string, prefix?: string) {
    loading.value = true
    error.value = null
    try {
      objects.value = await api.listObjects(bucket, prefix)
      currentPrefix.value = prefix || ''
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function deleteObject(bucket: string, key: string) {
    loading.value = true
    error.value = null
    try {
      await api.deleteObject(bucket, key)
      // Refresh current list
      await fetchObjects(bucket, currentPrefix.value || undefined)
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      loading.value = false
    }
  }

  function navigateToPrefix(prefix: string) {
    currentPrefix.value = prefix
  }

  return {
    objects,
    currentPrefix,
    loading,
    error,
    fetchObjects,
    deleteObject,
    navigateToPrefix
  }
})
