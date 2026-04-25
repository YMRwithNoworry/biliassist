<template>
  <div class="app">
    <div v-if="initialLoading" class="app-loading">
      <div class="loading-spinner">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#2F81F7" stroke-width="2">
          <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
          <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
        </svg>
      </div>
      <p class="loading-text">加载中...</p>
    </div>
    <div v-else-if="initError" class="app-error">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#F85149" stroke-width="1.5">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      <h2 class="error-title">启动失败</h2>
      <p class="error-desc">{{ initError }}</p>
      <button class="error-btn" @click="retry">重试</button>
    </div>
    <RouterView v-else />
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { RouterView } from 'vue-router'
import { useAuthStore } from './stores/auth'

const initialLoading = ref(true)
const initError = ref('')
let timeoutTimer = null

onMounted(async () => {
  timeoutTimer = setTimeout(() => {
    if (initialLoading.value) {
      initialLoading.value = false
      initError.value = '应用启动超时，请检查网络连接后重试'
    }
  }, 8000)

  try {
    const auth = useAuthStore()
    await auth.getSession()
  } catch (e) {
    initError.value = e?.message || '应用初始化失败'
  } finally {
    clearTimeout(timeoutTimer)
    initialLoading.value = false
  }
})

onUnmounted(() => {
  if (timeoutTimer) clearTimeout(timeoutTimer)
})

const retry = () => {
  initError.value = ''
  initialLoading.value = true
  timeoutTimer = setTimeout(() => {
    if (initialLoading.value) {
      initialLoading.value = false
      initError.value = '应用启动超时，请检查网络连接后重试'
    }
  }, 8000)

  const auth = useAuthStore()
  auth.getSession().catch(e => {
    initError.value = e?.message || '应用初始化失败'
  }).finally(() => {
    clearTimeout(timeoutTimer)
    initialLoading.value = false
  })
}
</script>

<style scoped>
.app {
  min-height: 100vh;
}

.app-loading {
  min-height: 100vh;
  background-color: #0D1117;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}

.loading-spinner svg {
  animation: pulse 1.5s ease-in-out infinite;
}

.loading-text {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
}

@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}

.app-error {
  min-height: 100vh;
  background-color: #0D1117;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 24px;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
  text-align: center;
}

.error-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: #F85149;
}

.error-desc {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
  max-width: 400px;
  line-height: 1.5;
}

.error-btn {
  margin-top: 8px;
  padding: 10px 24px;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 6px;
  color: #E6EDF3;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.error-btn:hover {
  background-color: #30363D;
}
</style>
