<template>
  <div class="home-view">
    <div v-if="!connectionStore.connected" class="connection-required">
      <ConnectionConfig />
    </div>
    <div v-else class="main-layout">
      <div class="header">
        <div class="header-content">
          <h2 v-if="!showSettings">{{ connectionStore.config?.endpoint }}:{{ connectionStore.config?.port }}</h2>
          <h2 v-else>
            <button @click="showSettings = false" class="back-btn">← 返回</button>
            加密设置
          </h2>
          <div class="header-actions">
            <button @click="showSettings = !showSettings" class="settings-btn">
              {{ showSettings ? '关闭设置' : '⚙️ 设置' }}
            </button>
            <button @click="handleDisconnect" class="disconnect-btn">断开连接</button>
          </div>
        </div>
      </div>
      <div class="sidebar" v-show="!showSettings">
        <BucketList />
      </div>
      <div class="content" :class="{ 'full-width': showSettings }">
        <EncryptionSettings v-if="showSettings" />
        <ObjectBrowser v-else-if="bucketStore.currentBucket" />
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
import { onMounted, ref } from 'vue'
import { useConnectionStore } from '../stores/connection'
import { useBucketStore } from '../stores/bucket'
import ConnectionConfig from '../components/ConnectionConfig.vue'
import BucketList from '../components/BucketList.vue'
import ObjectBrowser from '../components/ObjectBrowser.vue'
import TransferPanel from '../components/TransferPanel.vue'
import EncryptionSettings from '../components/EncryptionSettings.vue'

const connectionStore = useConnectionStore()
const bucketStore = useBucketStore()
const showSettings = ref(false)

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
  display: flex;
  align-items: center;
  gap: 12px;
}

.back-btn {
  padding: 4px 12px;
  background-color: transparent;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  color: #333;
  transition: all 0.2s;
}

.back-btn:hover {
  background-color: #f5f5f5;
  border-color: #999;
}

.header-actions {
  display: flex;
  gap: 12px;
}

.settings-btn {
  padding: 6px 12px;
  background-color: #667eea;
  color: white;
  border: 1px solid #667eea;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: background-color 0.2s;
}

.settings-btn:hover {
  background-color: #5568d3;
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

.content.full-width {
  grid-column: 1 / -1;
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
  
  .back-btn {
    background-color: transparent;
    border-color: #444;
    color: #fff;
  }
  
  .back-btn:hover {
    background-color: #333;
    border-color: #666;
  }
  
  .settings-btn {
    background-color: #667eea;
    border-color: #667eea;
  }
  
  .settings-btn:hover {
    background-color: #5568d3;
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
