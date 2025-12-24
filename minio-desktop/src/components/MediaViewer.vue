<template>
  <div v-if="visible" class="media-viewer-overlay" @click="close">
    <div ref="mediaViewerRef" class="media-viewer" @click.stop>
      <div class="header">
        <h3>{{ fileName }}</h3>
        <button @click="close" class="close-btn">✕</button>
      </div>

      <div class="content">
        <div v-if="loading" class="loading">
          加载中...
        </div>
        <img v-else-if="isImage && imageUrl" :src="imageUrl" :alt="fileName" @load="onImageLoad" @error="onImageError" />
        <video
          v-else-if="isVideo && videoUrl"
          :src="videoUrl"
          controls
          playsinline
          preload="metadata"
          @error="onVideoError"
          @canplay="onVideoCanPlay"
          @loadedmetadata="onVideoMetadataLoaded"
          @loadstart="onVideoLoadStart"
          @waiting="onVideoWaiting"
          @playing="onVideoPlaying"
          @ended="onVideoEnded"
        ></video>
        <div v-else-if="!loading" class="unsupported">
          不支持此文件类型的预览
        </div>
      </div>

      <div class="actions">
        <button @click="toggleFullscreen" class="fullscreen-btn" title="全屏">
          <span>{{ isFullscreen ? '❐' : '☐' }}</span>
        </button>
        <button @click="playNext" class="next-btn" title="下一个" v-if="showNextButton">
          <span>⏭</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { api } from '../api'
import { join as pathJoin, downloadDir } from '@tauri-apps/api/path'
import { appWindow } from '@tauri-apps/api/window'

interface Props {
  visible: boolean
  bucket: string
  objectKey: string
  fileName: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'close'): void
}>()

const imageUrl = ref<string>('')
const videoUrl = ref<string>('')
const loading = ref(false)
const isFullscreen = ref(false)
const showNextButton = ref(false)
const currentVideoIndex = ref(-1)
const videoFiles = ref<string[]>([])

// Refs for DOM elements
const mediaViewerRef = ref<HTMLElement | null>(null)

const fileExtension = computed(() => {
  return props.fileName.split('.').pop()?.toLowerCase() || ''
})

const isImage = computed(() => {
  return ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'].includes(fileExtension.value)
})

const isVideo = computed(() => {
  return ['mp4', 'webm', 'ogg', 'mov'].includes(fileExtension.value)
})

// Keyboard event handler for navigation
function handleKeyDown(event: KeyboardEvent) {
  if (showNextButton.value) { // Only enable keyboard navigation if there are multiple files
    if (event.key === 'ArrowRight') {
      playNext();
    } else if (event.key === 'ArrowLeft') {
      playPrevious();
    }
  }
}

// Play previous video/image in the list
async function playPrevious() {
  if (currentVideoIndex.value === -1 || videoFiles.value.length <= 1) return;
  
  // Get the previous item in the list (loop back to end if at beginning)
  const prevIndex = (currentVideoIndex.value - 1 + videoFiles.value.length) % videoFiles.value.length;
  const prevKey = videoFiles.value[prevIndex];
  
  if (prevKey) {
    try {
      // Update the current object key and load the previous item
      currentVideoIndex.value = prevIndex;
      
      if (isVideo.value) {
        // For videos, use stream URL directly
        videoUrl.value = await api.getStreamUrl(props.bucket, prevKey)
        console.log('Playing previous video:', prevKey)
        
        // We need to update the video source and reload
        const videoElement = document.querySelector('video') as HTMLVideoElement
        if (videoElement) {
          videoElement.src = videoUrl.value
          videoElement.load()
          videoElement.play()
        }
      } else if (isImage.value) {
        // Clean up old image blob URL
        if (imageUrl.value && imageUrl.value.startsWith('blob:')) {
          URL.revokeObjectURL(imageUrl.value)
        }
        
        // For images, use getObjectData to avoid certificate issues
        const imageData = await api.getObjectData(props.bucket, prevKey)
        const ext = prevKey.split('.').pop()?.toLowerCase() || 'jpg'
        const mimeType = getMimeType(ext)
        const uint8Data = imageData instanceof Uint8Array ? imageData : new Uint8Array(imageData)
        const blob = new Blob([uint8Data], { type: mimeType })
        imageUrl.value = URL.createObjectURL(blob)
        console.log('Loading previous image:', prevKey, 'MIME type:', mimeType)
        
        // We need to update the image source
        const imgElement = document.querySelector('img') as HTMLImageElement
        if (imgElement) {
          imgElement.src = imageUrl.value
        }
      }
    } catch (error) {
      console.error('Failed to load previous item:', error);
      if (isVideo.value) {
        alert('加载上一个视频失败: ' + (error as Error).message);
      } else if (isImage.value) {
        alert('加载上一张图片失败: ' + (error as Error).message);
      }
    }
  }
}

watch(() => props.visible, async (newVal) => {
  if (newVal) {
    await loadMedia()
  } else {
    cleanup()
  }
})

// Helper function to get correct MIME type
function getMimeType(extension: string): string {
  const mimeTypes: Record<string, string> = {
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'png': 'image/png',
    'gif': 'image/gif',
    'webp': 'image/webp',
    'bmp': 'image/bmp',
    'svg': 'image/svg+xml'
  }
  return mimeTypes[extension] || `image/${extension}`
}

async function loadMedia() {
  loading.value = true
  try {
    console.log('Loading media:', props.fileName, 'Type:', isVideo.value ? 'video' : 'image')
    if (isVideo.value) {
      // 对于视频,直接使用流媒体 URL,避免一次性加载到内存
      // 流媒体服务器支持 Range 请求,可以实现渐进式加载
      videoUrl.value = await api.getStreamUrl(props.bucket, props.objectKey)
      console.log('Video stream URL:', videoUrl.value)
      
      // Load video list for next button functionality
      await loadVideoList()
    } else if (isImage.value) {
      // 图片使用 Blob URL 方式(图片通常较小,可以一次性加载)
      const imageData = await api.getObjectData(props.bucket, props.objectKey)
      console.log('Image data loaded:')
      console.log('  - Type:', Object.prototype.toString.call(imageData))
      console.log('  - Is Array:', Array.isArray(imageData))
      console.log('  - Is Uint8Array:', imageData instanceof Uint8Array)
      console.log('  - Length/byteLength:', imageData.length || imageData.byteLength)
      console.log('  - First 10 bytes:', imageData.slice(0, 10))
      console.log('File extension:', fileExtension.value)
      
      const mimeType = getMimeType(fileExtension.value)
      console.log('Using MIME type:', mimeType)
      
      // Convert to Uint8Array if it's a regular array
      const uint8Data = imageData instanceof Uint8Array ? imageData : new Uint8Array(imageData)
      console.log('Uint8Array length:', uint8Data.length)
      
      const blob = new Blob([uint8Data], { type: mimeType })
      console.log('Blob created, size:', blob.size, 'type:', blob.type)
      
      imageUrl.value = URL.createObjectURL(blob)
      console.log('Image blob URL created:', imageUrl.value)
      
      // Load image list for next button functionality
      await loadImageList()
    }
  } catch (error) {
    console.error('Failed to load media:', error)
    alert('加载媒体失败: ' + (error as Error).message)
  } finally {
    loading.value = false
  }
}

// Load video list for next button functionality
async function loadVideoList() {
  try {
    // Get the current directory (prefix) from the object key
    const currentDir = props.objectKey.substring(0, props.objectKey.lastIndexOf('/') + 1);
    
    // Fetch all objects in the current directory
    const objects = await api.listObjects(props.bucket, currentDir);
    
    // Filter video files
    const videoExtensions = ['mp4', 'webm', 'ogg', 'mov'];
    const currentVideoFiles = objects
      .filter(obj => !obj.is_dir) // Only files, not directories
      .filter(obj => {
        const ext = obj.key.split('.').pop()?.toLowerCase();
        return ext && videoExtensions.includes(ext);
      })
      .map(obj => obj.key) // Get just the keys
      .sort(); // Sort alphabetically
    
    videoFiles.value = currentVideoFiles;
    currentVideoIndex.value = currentVideoFiles.findIndex(file => file === props.objectKey);
    showNextButton.value = currentVideoIndex.value !== -1 && currentVideoFiles.length > 1;
  } catch (error) {
    console.error('Failed to load video list:', error);
    showNextButton.value = false;
  }
}

// Load image list for next button functionality
async function loadImageList() {
  try {
    // Get the current directory (prefix) from the object key
    const currentDir = props.objectKey.substring(0, props.objectKey.lastIndexOf('/') + 1);
    
    // Fetch all objects in the current directory
    const objects = await api.listObjects(props.bucket, currentDir);
    
    // Filter image files
    const imageExtensions = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'];
    const currentImageFiles = objects
      .filter(obj => !obj.is_dir) // Only files, not directories
      .filter(obj => {
        const ext = obj.key.split('.').pop()?.toLowerCase();
        return ext && imageExtensions.includes(ext);
      })
      .map(obj => obj.key) // Get just the keys
      .sort(); // Sort alphabetically
    
    videoFiles.value = currentImageFiles; // Reuse the same variable for images
    currentVideoIndex.value = currentImageFiles.findIndex(file => file === props.objectKey);
    showNextButton.value = currentVideoIndex.value !== -1 && currentImageFiles.length > 1;
  } catch (error) {
    console.error('Failed to load image list:', error);
    showNextButton.value = false;
  }
}



// Toggle fullscreen mode
async function toggleFullscreen() {
  console.log('Toggle fullscreen called, isFullscreen:', isFullscreen.value);
  
  // Wait for next tick to ensure DOM is updated
  await nextTick();
  
  // Try to get the video element directly
  const videoElement = document.querySelector('video') as HTMLVideoElement;
  
  if (videoElement) {
    console.log('Found video element, attempting fullscreen on video:', videoElement);
    
    if (!isFullscreen.value) {
      // Enter fullscreen on video element
      try {
        // Try standard API first
        if (videoElement.requestFullscreen) {
          await videoElement.requestFullscreen();
          console.log('Video fullscreen entered successfully');
          isFullscreen.value = true;
        } else {
          console.error('Video element does not support requestFullscreen API');
          // Try with webkit prefix as fallback
          if ((videoElement as any).webkitRequestFullscreen) {
            (videoElement as any).webkitRequestFullscreen();
            console.log('Video fullscreen entered successfully (webkit)');
            isFullscreen.value = true;
          } else {
            console.error('Video element does not support any fullscreen API');
            
            // Fallback to CSS-based full screen
            toggleCSSFullscreen();
          }
        }
      } catch (err) {
        console.error('Video fullscreen request failed:', err);
        
        // Fallback to CSS-based full screen
        toggleCSSFullscreen();
      }
    } else {
      // Exit fullscreen
      try {
        if (document.exitFullscreen) {
          await document.exitFullscreen();
          console.log('Video fullscreen exited successfully');
          isFullscreen.value = false;
        } else {
          console.log('Could not exit fullscreen, document.exitFullscreen not available');
          
          // Fallback to CSS-based full screen for exit
          toggleCSSFullscreen();
        }
      } catch (err) {
        console.error('Exit fullscreen failed:', err);
        
        // Fallback to CSS-based full screen for exit
        toggleCSSFullscreen();
      }
    }
  } else {
    console.error('Video element not found');
    
    // Fallback to CSS-based full screen
    toggleCSSFullscreen();
  }
}

// CSS-based fullscreen as fallback
function toggleCSSFullscreen() {
  const overlay = document.querySelector('.media-viewer-overlay') as HTMLElement;
  const mediaViewer = document.querySelector('.media-viewer') as HTMLElement;
  
  if (mediaViewer && overlay) {
    if (!isFullscreen.value) {
      // Enter CSS fullscreen
      // Make the overlay cover the entire screen
      overlay.style.position = 'fixed';
      overlay.style.top = '0';
      overlay.style.left = '0';
      overlay.style.width = '100vw';
      overlay.style.height = '100vh';
      overlay.style.zIndex = '9999';
      overlay.style.backgroundColor = 'rgba(0, 0, 0, 0.9)';
      
      // Add fullscreen class to media viewer
      mediaViewer.classList.add('fullscreen');
      
      console.log('CSS fullscreen entered');
      isFullscreen.value = true;
    } else {
      // Exit CSS fullscreen
      // Reset overlay styles
      overlay.style.position = '';
      overlay.style.top = '';
      overlay.style.left = '';
      overlay.style.width = '';
      overlay.style.height = '';
      overlay.style.zIndex = '';
      overlay.style.backgroundColor = '';
      
      // Remove fullscreen class from media viewer
      mediaViewer.classList.remove('fullscreen');
      
      console.log('CSS fullscreen exited');
      isFullscreen.value = false;
    }
  }
}

// Helper function to toggle fullscreen for an element
async function toggleElementFullscreen(element: HTMLElement) {
  console.log('Attempting to enter fullscreen for element:', element);
  
  if (!isFullscreen.value) {
    // Enter fullscreen
    if (element.requestFullscreen) {
      try {
        await element.requestFullscreen();
        console.log('Element fullscreen entered successfully');
        isFullscreen.value = true;
      } catch (err) {
        console.error('Element fullscreen request failed:', err);
      }
    } else if ((element as any).webkitRequestFullscreen) {
      // Safari
      console.log('Using webkitRequestFullscreen on element');
      (element as any).webkitRequestFullscreen();
      isFullscreen.value = true;
    } else if ((element as any).msRequestFullscreen) {
      // IE/Edge
      console.log('Using msRequestFullscreen on element');
      (element as any).msRequestFullscreen();
      isFullscreen.value = true;
    } else {
      console.error('Element does not support fullscreen API');
    }
  } else {
    // Exit fullscreen
    if (document.exitFullscreen) {
      await document.exitFullscreen();
      console.log('Element fullscreen exited successfully');
    } else if ((document as any).webkitExitFullscreen) {
      // Safari
      (document as any).webkitExitFullscreen();
      console.log('Element fullscreen exited successfully (webkit)');
    } else if ((document as any).msExitFullscreen) {
      // IE/Edge
      (document as any).msExitFullscreen();
      console.log('Element fullscreen exited successfully (ms)');
    }
    isFullscreen.value = false;
  }
}

// Play next video/image in the list
async function playNext() {
  if (currentVideoIndex.value === -1 || videoFiles.value.length <= 1) return;
  
  // Get the next item in the list (loop back to beginning if at end)
  const nextIndex = (currentVideoIndex.value + 1) % videoFiles.value.length;
  const nextKey = videoFiles.value[nextIndex];
  
  if (nextKey) {
    try {
      // Update the current object key and load the next item
      // We need to update the props, but since we can't directly modify props,
      // we'll need to emit an event or use a different approach
      
      // For now, we'll update the local state and reload
      currentVideoIndex.value = nextIndex;
      
      if (isVideo.value) {
        // For videos, use stream URL
        videoUrl.value = await api.getStreamUrl(props.bucket, nextKey)
        console.log('Playing next video:', nextKey)
        
        // We need to update the video source and reload
        const videoElement = document.querySelector('video') as HTMLVideoElement
        if (videoElement) {
          videoElement.src = videoUrl.value
          videoElement.load()
          videoElement.play()
        }
      } else if (isImage.value) {
        // Clean up old image blob URL
        if (imageUrl.value && imageUrl.value.startsWith('blob:')) {
          URL.revokeObjectURL(imageUrl.value)
        }
        
        // For images, use getObjectData to avoid certificate issues
        const imageData = await api.getObjectData(props.bucket, nextKey)
        const ext = nextKey.split('.').pop()?.toLowerCase() || 'jpg'
        const mimeType = getMimeType(ext)
        const uint8Data = imageData instanceof Uint8Array ? imageData : new Uint8Array(imageData)
        const blob = new Blob([uint8Data], { type: mimeType })
        imageUrl.value = URL.createObjectURL(blob)
        console.log('Loading next image:', nextKey, 'MIME type:', mimeType)
        
        // We need to update the image source
        const imgElement = document.querySelector('img') as HTMLImageElement
        if (imgElement) {
          imgElement.src = imageUrl.value
        }
      }
    } catch (error) {
      console.error('Failed to load next item:', error);
      if (isVideo.value) {
        alert('加载下一个视频失败: ' + (error as Error).message);
      } else if (isImage.value) {
        alert('加载下一张图片失败: ' + (error as Error).message);
      }
    }
  }
}

// Listen for fullscreen change events and keyboard events
onMounted(() => {
  document.addEventListener('fullscreenchange', handleFullscreenChange);
  document.addEventListener('webkitfullscreenchange', handleFullscreenChange);
  document.addEventListener('mozfullscreenchange', handleFullscreenChange);
  document.addEventListener('MSFullscreenChange', handleFullscreenChange);
  
  // Add keyboard event listener for navigation
  document.addEventListener('keydown', handleKeyDown);
});

onUnmounted(() => {
  document.removeEventListener('fullscreenchange', handleFullscreenChange);
  document.removeEventListener('webkitfullscreenchange', handleFullscreenChange);
  document.removeEventListener('mozfullscreenchange', handleFullscreenChange);
  document.removeEventListener('MSFullscreenChange', handleFullscreenChange);
  
  // Remove keyboard event listener
  document.removeEventListener('keydown', handleKeyDown);
});

function handleFullscreenChange() {
  // Check if native fullscreen is active
  const isNativeFullscreen = !!document.fullscreenElement || 
    (document as any).webkitFullscreenElement || 
    (document as any).mozFullScreenElement ||
    (document as any).msFullscreenElement;
  
  // If native fullscreen is not active, check if we're in CSS fullscreen mode
  if (!isNativeFullscreen) {
    const mediaViewer = document.querySelector('.media-viewer') as HTMLElement;
    isFullscreen.value = mediaViewer?.classList.contains('fullscreen') || false;
  } else {
    isFullscreen.value = isNativeFullscreen;
  }
}

function cleanup() {
  // Revoke blob URLs to free memory
  if (imageUrl.value && imageUrl.value.startsWith('blob:')) {
    URL.revokeObjectURL(imageUrl.value)
  }
  // 视频现在使用流媒体 URL,不是 blob URL,不需要 revoke
  imageUrl.value = ''
  videoUrl.value = ''
}

function close() {
  emit('close')
}

async function download() {
  try {
    // 获取系统下载目录
    const downloadDirPath = await downloadDir();
    const fileName = props.fileName;
    const localPath = await pathJoin(downloadDirPath || '.', fileName);
    
    // Trigger download
    await api.downloadFile(props.bucket, props.objectKey, localPath)
    alert('下载已开始,请查看传输面板')
  } catch (error) {
    console.error('Download failed:', error)
    alert('下载失败: ' + error)
  }
}

function onImageLoad() {
  console.log('Image loaded successfully')
}

function onImageError(event: Event) {
  console.error('Image failed to load:', event)
  // 不显示错误提示，避免在加载过程中出现错误信息
  // alert('图片加载失败')
}

function onVideoError(event: Event) {
  const videoElement = event.target as HTMLVideoElement
  console.error('Video failed to load:', event)
  console.error('Video error details:', {
    error: videoElement.error,
    errorCode: videoElement.error?.code,
    errorMessage: videoElement.error?.message,
    networkState: videoElement.networkState,
    readyState: videoElement.readyState,
    src: videoElement.src,
    currentSrc: videoElement.currentSrc
  })
  
  // 视频加载失败的错误码:
  // 1 = MEDIA_ERR_ABORTED - 用户中止
  // 2 = MEDIA_ERR_NETWORK - 网络错误  
  // 3 = MEDIA_ERR_DECODE - 解码错误
  // 4 = MEDIA_ERR_SRC_NOT_SUPPORTED - 不支持的格式
  
  let errorMsg = '视频加载失败'
  let suggestion = ''
  
  if (videoElement.error) {
    switch (videoElement.error.code) {
      case 1:
        errorMsg = '视频加载被中止'
        break
      case 2:
        errorMsg = '网络错误,请检查连接后重试'
        suggestion = '\n\n可能原因:\n- 网络不稳定\n- MinIO 服务器连接中断'
        break
      case 3:
        errorMsg = '视频解码失败'
        suggestion = '\n\n可能原因:\n- 文件损坏或不完整\n- 加密状态不匹配(上传时加密状态与当前不一致)\n\n建议:\n1. 检查上传任务是否成功完成\n2. 尝试切换加密开关后重试\n3. 重新上传该文件'
        break
      case 4:
        errorMsg = '不支持此视频格式或文件损坏'
        suggestion = '\n\n可能原因:\n- 文件上传不完整(文件大小异常)\n- 加密状态不匹配\n\n建议重新上传该文件'
        break
    }
  }
  
  alert(errorMsg + suggestion)
}

function onVideoCanPlay() {
  console.log('Video can play')
}

function onVideoMetadataLoaded() {
  console.log('Video metadata loaded')
}

function onVideoLoadStart() {
  console.log('Video load started')
  // 大视频可能需要较长时间加载第一个分片
}

function onVideoWaiting() {
  console.log('Video is waiting for data - buffering...')
  // 缓冲中,对于大视频这是正常现象
}

function onVideoPlaying() {
  console.log('Video is playing')
}

function onVideoEnded() {
  console.log('Video playback ended')
}

</script>

<style scoped>
.media-viewer-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.media-viewer {
  background-color: #1e1e1e;
  border-radius: 8px;
  max-width: 90vw;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #333;
}

.header h3 {
  margin: 0;
  color: #fff;
  font-size: 16px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.close-btn {
  width: 32px;
  height: 32px;
  border: none;
  background-color: transparent;
  color: #fff;
  font-size: 24px;
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color 0.2s;
}

.close-btn:hover {
  background-color: #333;
}

.content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  overflow: auto;
  min-height: 400px;
}

.loading {
  color: #fff;
  font-size: 16px;
  text-align: center;
}

.content img {
  max-width: 100%;
  max-height: calc(90vh - 140px);
  object-fit: contain;
}

.content video {
  max-width: 100%;
  max-height: calc(90vh - 140px);
  background-color: #000;
}

/* Fullscreen mode styles */
.media-viewer.fullscreen {
  width: 100% !important;
  height: 100% !important;
  max-width: 100% !important;
  max-height: 100% !important;
  border-radius: 0 !important;
}

.media-viewer.fullscreen .content {
  flex: 1;
  min-height: auto;
}

.media-viewer.fullscreen .content video {
  max-width: 100% !important;
  max-height: 100% !important;
  width: 100% !important;
  height: 100% !important;
  object-fit: contain;
}

.media-viewer.fullscreen .content img {
  max-width: 100% !important;
  max-height: 100% !important;
  width: 100% !important;
  height: 100% !important;
  object-fit: contain;
}

.unsupported {
  color: #999;
  font-size: 16px;
  text-align: center;
}

.actions {
  padding: 16px 20px;
  border-top: 1px solid #333;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.actions button {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  background-color: #667eea;
  color: white;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.actions button:hover {
  background-color: #5568d3;
}
</style>
