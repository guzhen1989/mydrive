<template>
  <div class="app-container">
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useConnectionStore } from './stores/connection'

const connectionStore = useConnectionStore()

onMounted(async () => {
  // Load saved connection on startup
  await connectionStore.loadConnection()
})
</script>

<style scoped>
.app-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}
</style>
