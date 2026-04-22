<template>
  <div class="page">
    <div class="topbar">
      <button class="topbar-back" @click="goBack">←</button>
      <span class="topbar-title">扫码登录</span>
    </div>

    <div class="page-body slide-up">
      <div v-if="!isLoggedIn" class="login-center">
        <div v-if="qrcodeUrl" class="qr-wrap">
          <img :src="qrcodeUrl" alt="扫码登录" class="qr-image" />
          <p class="qr-status">{{ loginStatus }}</p>
          <p class="qr-hint">请使用B站App扫描二维码</p>
        </div>

        <div v-else class="qr-placeholder">
          <button class="btn btn-primary btn-block" @click="getQrCode">
            {{ loading ? '生成中...' : '获取二维码' }}
          </button>
        </div>
      </div>

      <div v-else class="success-center">
        <div class="success-icon">✓</div>
        <h2 class="section-title" style="text-align:center">登录成功</h2>
        <div class="success-info">
          <p><span class="label">用户名</span>{{ userInfo.name }}</p>
          <p><span class="label">UID</span>{{ userInfo.uid }}</p>
        </div>
        <button class="btn btn-primary btn-block" @click="goToAccounts" style="margin-top:24px">
          查看账号
        </button>
      </div>
    </div>
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

const goBack = () => router.push('/')
const goToAccounts = () => router.push('/account')

onUnmounted(() => {
  if (pollInterval) clearInterval(pollInterval)
})
</script>

<style scoped>
.login-center,
.success-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: var(--sp-xl);
}

.qr-wrap {
  text-align: center;
}

.qr-image {
  width: 220px;
  height: 220px;
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  margin-bottom: var(--sp-lg);
}

.qr-status {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.qr-hint {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: var(--sp-sm);
}

.qr-placeholder {
  width: 100%;
  max-width: 320px;
  padding-top: var(--sp-2xl);
}

.success-icon {
  width: 64px;
  height: 64px;
  background: var(--rausch-red);
  color: white;
  border-radius: var(--radius-circle);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 32px;
  margin-bottom: var(--sp-md);
}

.success-info p {
  font-size: 16px;
  color: var(--text-primary);
  margin: var(--sp-sm) 0;
}

.success-info .label {
  display: inline-block;
  width: 72px;
  font-weight: 600;
  color: var(--text-secondary);
}
</style>
