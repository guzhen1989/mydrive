import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type BucketInfo } from '../api'

export const useBucketStore = defineStore('bucket', () => {
  const buckets = ref<BucketInfo[]>([])
  const currentBucket = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchBuckets() {
    loading.value = true
    error.value = null
    try {
      buckets.value = await api.listBuckets()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function createBucket(name: string) {
    loading.value = true
    error.value = null
    try {
      await api.createBucket(name)
      await fetchBuckets()
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      loading.value = false
    }
  }

  async function deleteBucket(name: string) {
    loading.value = true
    error.value = null
    try {
      await api.deleteBucket(name)
      await fetchBuckets()
      if (currentBucket.value === name) {
        currentBucket.value = null
      }
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      loading.value = false
    }
  }

  function selectBucket(name: string) {
    currentBucket.value = name
  }

  return {
    buckets,
    currentBucket,
    loading,
    error,
    fetchBuckets,
    createBucket,
    deleteBucket,
    selectBucket
  }
})
