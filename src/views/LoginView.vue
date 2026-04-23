<template>
  <div class="login-page">
    <!-- Header -->
    <header class="page-header">
      <button class="btn-back" @click="goBack">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M7.78 12.53a.75.75 0 0 1-1.06 0L2.47 8.28a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 1.06L4.81 7h7.44a.75.75 0 0 1 0 1.5H4.81l2.97 2.97a.75.75 0 0 1 0 1.06Z"/>
        </svg>
        返回
      </button>
      <h1 class="page-title">扫码登录</h1>
      <div class="header-spacer"></div>
    </header>

    <!-- Main Content -->
    <main class="login-main">
      <!-- QR Code Section -->
      <div v-if="!isLoggedIn" class="login-section">
        <div v-if="qrcodeUrl" class="qr-container">
          <div class="qr-card">
            <img :src="qrcodeUrl" alt="扫码登录" class="qr-image" />
          </div>
          <div class="status-badge" :class="getStatusClass()">
            <svg v-if="loginStatus === '等待扫码...'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <path d="M12 6v6l4 2"/>
            </svg>
            <svg v-else-if="loginStatus === '已扫码，请在手机上确认'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
              <polyline points="22 4 12 14.01 9 11.01"/>
            </svg>
            <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="8" x2="12" y2="12"/>
              <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            {{ loginStatus }}
          </div>
          <p class="qr-hint">请使用 B站 App 扫描二维码登录</p>
          <button class="btn-refresh" @click="getQrCode">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="23 4 23 10 17 10"/>
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
            </svg>
            刷新二维码
          </button>
        </div>

        <div v-else class="placeholder-section">
          <div class="placeholder-icon">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            </svg>
          </div>
          <h2 class="placeholder-title">扫码登录 B站账号</h2>
          <p class="placeholder-desc">安全、快速、无需输入密码</p>
          <button class="btn-primary" @click="getQrCode" :disabled="loading">
            <svg v-if="loading" class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <path d="M12 6v6l4 2"/>
            </svg>
            <span v-else>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                <line x1="8" y1="12" x2="16" y2="12"/>
                <line x1="12" y1="8" x2="12" y2="16"/>
              </svg>
            </span>
            {{ loading ? '生成中...' : '获取二维码' }}
          </button>
        </div>
      </div>

      <!-- Success Section -->
      <div v-else class="success-section">
        <div class="success-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        </div>
        <h2 class="success-title">登录成功</h2>
        <div class="user-card">
          <div class="user-avatar">
            {{ userInfo.name.charAt(0).toUpperCase() }}
          </div>
          <div class="user-info">
            <p class="user-name">{{ userInfo.name }}</p>
            <p class="user-uid">UID: {{ userInfo.uid }}</p>
          </div>
        </div>
        <button class="btn-primary" @click="goToAccounts">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
            <circle cx="9" cy="7" r="4"/>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
          </svg>
          查看账号
        </button>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const router = useRouter()
const qrcodeUrl = ref('')
const loginStatus = ref('')
const isLoggedIn = ref(false)
const loading = ref(false)
const userInfo = ref({ name: '', uid: '' })
let pollInterval = null

const getQrCode = async () => {
  try {
    loading.value = true
    const result = await invoke('get_qr_code')
    qrcodeUrl.value = `data:image/png;base64,${result.qrcode}`
    loginStatus.value = '等待扫码...'
    pollInterval = setInterval(pollLoginStatus, 2000)
  } catch (error) {
    console.error('获取二维码失败:', error)
    alert('获取二维码失败，请重试')
  } finally {
    loading.value = false
  }
}

const pollLoginStatus = async () => {
  try {
    const status = await invoke('check_login_status')
    if (status.status === 'success') {
      clearInterval(pollInterval)
      isLoggedIn.value = true
      userInfo.value = status.userInfo
      loginStatus.value = '登录成功！'
    } else if (status.status === 'expired') {
      clearInterval(pollInterval)
      loginStatus.value = '二维码已过期，请重新获取'
      qrcodeUrl.value = ''
    } else if (status.status === 'scanned') {
      loginStatus.value = '已扫码，请在手机上确认'
    }
  } catch (error) {
    console.error('检查登录状态失败:', error)
  }
}

const getStatusClass = () => {
  if (loginStatus.value === '登录成功！') return 'status-success'
  if (loginStatus.value === '二维码已过期，请重新获取') return 'status-error'
  if (loginStatus.value === '已扫码，请在手机上确认') return 'status-warning'
  return 'status-pending'
}

const goBack = () => router.push('/')
const goToAccounts = () => router.push('/accounts')

onUnmounted(() => {
  if (pollInterval) clearInterval(pollInterval)
})
</script>

<style scoped>
/* Octo Code Design System - Dark Theme */

.login-page {
  min-height: 100vh;
  background-color: #0D1117;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
}

/* Header */
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  background-color: #161B22;
  border-bottom: 1px solid #30363D;
}

.btn-back {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: transparent;
  border: 1px solid #30363D;
  border-radius: 6px;
  color: #C9D1D9;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-back:hover {
  background-color: #21262D;
  border-color: #484F58;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0;
}

.header-spacer {
  width: 80px;
}

/* Main */
.login-main {
  max-width: 480px;
  margin: 0 auto;
  padding: 48px 24px;
}

/* QR Section */
.login-section {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.qr-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
}

.qr-card {
  background-color: #FFFFFF;
  padding: 24px;
  border-radius: 16px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.qr-image {
  width: 200px;
  height: 200px;
  display: block;
}

.status-badge {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  border-radius: 9999px;
  font-size: 14px;
  font-weight: 500;
}

.status-pending {
  background-color: rgba(47, 129, 247, 0.15);
  color: #58A6FF;
}

.status-warning {
  background-color: rgba(210, 153, 34, 0.15);
  color: #D29922;
}

.status-success {
  background-color: rgba(46, 160, 67, 0.15);
  color: #3FB950;
}

.status-error {
  background-color: rgba(248, 81, 73, 0.15);
  color: #F85149;
}

.qr-hint {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
}

.btn-refresh {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background-color: transparent;
  border: 1px solid #30363D;
  border-radius: 6px;
  color: #C9D1D9;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-refresh:hover {
  background-color: #21262D;
  border-color: #484F58;
}

/* Placeholder Section */
.placeholder-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 48px 24px;
}

.placeholder-icon {
  color: #8B949E;
  margin-bottom: 24px;
}

.placeholder-title {
  font-size: 24px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 8px 0;
}

.placeholder-desc {
  font-size: 14px;
  color: #8B949E;
  margin: 0 0 32px 0;
}

.btn-primary {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  max-width: 280px;
  padding: 12px 24px;
  background-color: #238636;
  border: 1px solid rgba(46, 160, 67, 0.4);
  border-radius: 6px;
  color: #FFFFFF;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-primary:hover:not(:disabled) {
  background-color: #2EA043;
}

.btn-primary:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Success Section */
.success-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 48px 24px;
}

.success-icon {
  width: 64px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #238636;
  border-radius: 50%;
  color: #FFFFFF;
  margin-bottom: 24px;
}

.success-title {
  font-size: 24px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 24px 0;
}

.user-card {
  display: flex;
  align-items: center;
  gap: 16px;
  width: 100%;
  max-width: 320px;
  padding: 20px 24px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  margin-bottom: 32px;
}

.user-avatar {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #2F81F7, #388BFD);
  border-radius: 50%;
  font-size: 20px;
  font-weight: 600;
  color: #FFFFFF;
}

.user-info {
  text-align: left;
}

.user-name {
  font-size: 16px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 4px 0;
}

.user-uid {
  font-size: 13px;
  color: #8B949E;
  margin: 0;
}

/* Responsive */
@media (max-width: 640px) {
  .page-header {
    padding: 12px 16px;
  }
  
  .login-main {
    padding: 32px 16px;
  }
  
  .qr-image {
    width: 180px;
    height: 180px;
  }
  
  .placeholder-section {
    padding: 32px 16px;
  }
}
</style>