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
    <RouterView v-else />
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { RouterView } from 'vue-router'
import { useAuthStore } from './stores/auth'

const initialLoading = ref(true)

onMounted(async () => {
  const auth = useAuthStore()
  await auth.getSession()
  initialLoading.value = false
})
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
</style>
