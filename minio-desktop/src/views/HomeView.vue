<template>
  <div class="home-view">
    <div v-if="!connectionStore.connected" class="connection-required">
      <ConnectionConfig />
    </div>
    <div v-else class="main-layout">
      <div class="header">
        <div class="header-content">
          <h2>{{ connectionStore.config?.endpoint }}:{{ connectionStore.config?.port }}</h2>
          <button @click="handleDisconnect" class="disconnect-btn">断开连接</button>
        </div>
      </div>
      <div class="sidebar">
        <BucketList />
      </div>
      <div class="content">
        <ObjectBrowser v-if="bucketStore.currentBucket" />
        <div v-else class="select-bucket-prompt">
          <p>请选择一个存储桶以查看对象</p>
        </div>
      </div>
      <div class="transfer-panel">
        <TransferPanel />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useConnectionStore } from '../stores/connection'
import { useBucketStore } from '../stores/bucket'
import ConnectionConfig from '../components/ConnectionConfig.vue'
import BucketList from '../components/BucketList.vue'
import ObjectBrowser from '../components/ObjectBrowser.vue'
import TransferPanel from '../components/TransferPanel.vue'

const connectionStore = useConnectionStore()
const bucketStore = useBucketStore()

onMounted(async () => {
  if (connectionStore.connected) {
    await bucketStore.fetchBuckets()
  }
})

function handleDisconnect() {
  connectionStore.disconnect()
}
</script>

<style scoped>
.home-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.connection-required {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.header {
  background-color: #fff;
  border-bottom: 1px solid #e0e0e0;
  padding: 0 20px;
  height: 60px;
  display: flex;
  align-items: center;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}

.header h2 {
  margin: 0;
  font-size: 16px;
  color: #333;
}

.disconnect-btn {
  padding: 6px 12px;
  background-color: #f5f5f5;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.disconnect-btn:hover {
  background-color: #e0e0e0;
}

.main-layout {
  display: grid;
  grid-template-columns: 250px 1fr;
  grid-template-rows: 60px 1fr 200px;
  height: 100%;
  gap: 1px;
  background-color: #e0e0e0;
}

.sidebar {
  grid-row: 2;
  background-color: #f5f5f5;
  overflow-y: auto;
}

.content {
  grid-row: 2;
  background-color: white;
  overflow-y: auto;
}

.transfer-panel {
  grid-row: 3;
  background-color: #fafafa;
  border-top: 1px solid #e0e0e0;
  overflow-y: auto;
}

.select-bucket-prompt {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  color: #999;
  font-size: 16px;
}

@media (prefers-color-scheme: dark) {
  .header {
    background-color: #2a2a2a;
    border-bottom-color: #444;
  }

  .header h2 {
    color: #fff;
  }

  .disconnect-btn {
    background-color: #333;
    border-color: #444;
    color: #fff;
  }

  .disconnect-btn:hover {
    background-color: #444;
  }

  .main-layout {
    background-color: #333;
  }

  .sidebar {
    background-color: #2a2a2a;
  }

  .content {
    background-color: #1e1e1e;
  }

  .transfer-panel {
    background-color: #252525;
    border-top-color: #444;
  }

  .select-bucket-prompt {
    color: #666;
  }
}
</style>
