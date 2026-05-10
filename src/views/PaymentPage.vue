<template>
  <div class="payment-page">
    <div class="payment-container">
      <div class="payment-header">
        <div class="payment-logo">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#2F81F7" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
        </div>
        <h1 class="payment-title">升级到 Plus</h1>
        <p class="payment-subtitle">支付 6 元解锁全部功能</p>
      </div>

      <div class="payment-card">
        <div v-if="pageStatus === 'loading'" class="status-box">
          <div class="spinner"></div>
          <p>正在获取信息...</p>
        </div>

        <div v-else-if="pageStatus === 'verified'" class="verified-box">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#3FB950" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
            <polyline points="22 4 12 14.01 9 11.01"/>
          </svg>
          <h2 class="verified-title">已是 Plus 会员</h2>
          <p class="verified-desc">感谢您的支持！即将进入应用...</p>
        </div>

        <div v-else-if="pageStatus === 'error'" class="error-box">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#F85149" stroke-width="1.5">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="8"/>
            <line x1="12" y1="12" x2="12" y2="16"/>
          </svg>
          <p class="error-text">{{ errorMsg }}</p>
          <button class="btn btn-secondary" @click="init">重试</button>
        </div>

        <div v-else class="upgrade-box">
          <div class="features-card">
            <div class="feature-row">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#3FB950" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
              <span>管理多个 B站账号</span>
            </div>
            <div class="feature-row">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#3FB950" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
              <span>自动回复评论、私信</span>
            </div>
            <div class="feature-row">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#3FB950" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
              <span>评论自动点赞</span>
            </div>
            <div class="feature-row">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#3FB950" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
              <span>云端同步账号数据</span>
            </div>
          </div>

          <div class="price-badge">
            <span class="price-label">限时优惠</span>
            <span class="price-amount">¥6.00</span>
            <span class="price-note">永久有效</span>
          </div>

          <div class="qr-section">
            <div class="qr-wrapper" v-if="qrBase64">
              <img :src="'data:image/png;base64,' + qrBase64" alt="微信支付二维码" class="qr-image" />
            </div>
            <div class="qr-wrapper-loading" v-else>
              <div class="spinner"></div>
              <p>生成二维码中...</p>
            </div>
            <p class="qr-hint">使用微信扫码支付 6 元</p>
          </div>

          <div class="status-indicator" :class="orderStatus">
            <div class="pulse-dot"></div>
            <span>{{ orderStatus === 'order_created' ? '等待扫码支付...' : '订单已创建，等待支付' }}</span>
          </div>

          <div class="payment-guide">
            <div class="guide-step">
              <span class="step-num">1</span>
              <span class="step-text">打开微信扫一扫</span>
            </div>
            <div class="guide-step">
              <span class="step-num">2</span>
              <span class="step-text">扫描上方二维码</span>
            </div>
            <div class="guide-step">
              <span class="step-num">3</span>
              <span class="step-text">支付成功后自动升级到 Plus</span>
            </div>
          </div>

          <p class="auto-note">支付后系统自动升级，无需手动确认</p>
        </div>
      </div>

      <button class="btn-logout" @click="logout">退出登录</button>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { invoke } from '@tauri-apps/api/core'
import { PAYMENT_WORKER_URL } from '../lib/config'

const router = useRouter()
const auth = useAuthStore()

const pageStatus = ref('loading')
const orderStatus = ref('')
const errorMsg = ref('')
const qrBase64 = ref('')

let pollTimer = null
let redirectTimer = null

async function createOrder() {
  const resp = await fetch(`${PAYMENT_WORKER_URL}/create-order`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ user_id: auth.user.id })
  })
  if (!resp.ok) {
    const err = await resp.json()
    throw new Error(err.error || '创建订单失败')
  }
  return await resp.json()
}

async function generateQrImage(codeUrl) {
  const base64 = await invoke('generate_qr_code', { data: codeUrl })
  qrBase64.value = base64
}

async function checkOrderTier() {
  try {
    const resp = await fetch(`${PAYMENT_WORKER_URL}/check-order?user_id=${auth.user.id}`)
    if (!resp.ok) return null
    return await resp.json()
  } catch (e) {
    console.error('检查订单状态失败:', e)
    return null
  }
}

async function init() {
  pageStatus.value = 'loading'
  errorMsg.value = ''
  qrBase64.value = ''

  if (!PAYMENT_WORKER_URL || PAYMENT_WORKER_URL === 'http://localhost:8787') {
    pageStatus.value = 'error'
    errorMsg.value = '支付服务未配置，请在 src/lib/config.js 中设置 PAYMENT_WORKER_URL'
    return
  }

  // 1. 检查当前等级
  const result = await checkOrderTier()
  if (result?.tier === 'plus') {
    pageStatus.value = 'verified'
    auth.userTier = 'plus'
    scheduleRedirect()
    return
  }

  if (result?.status === 'pending') {
    pageStatus.value = 'pending'
    orderStatus.value = 'pending'
    startPolling()
    return
  }

  // 2. 创建新订单
  try {
    const order = await createOrder()
    pageStatus.value = 'order'
    orderStatus.value = 'order_created'
    await generateQrImage(order.code_url)
    startPolling()
  } catch (e) {
    pageStatus.value = 'error'
    errorMsg.value = e.message || '创建支付订单失败，请检查网络连接'
  }
}

function startPolling() {
  pollTimer = setInterval(async () => {
    if (pageStatus.value === 'verified') return
    const result = await checkOrderTier()
    if (result?.tier === 'plus') {
      pageStatus.value = 'verified'
      auth.userTier = 'plus'
      clearInterval(pollTimer)
      pollTimer = null
      scheduleRedirect()
    }
  }, 3000)
}

function scheduleRedirect() {
  redirectTimer = setTimeout(() => router.push('/'), 2000)
}

async function logout() {
  cleanup()
  await auth.signOut()
  router.push('/auth')
}

function cleanup() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
  if (redirectTimer) { clearTimeout(redirectTimer); redirectTimer = null }
}

onMounted(() => init())
onUnmounted(() => cleanup())
</script>

<style scoped>
.payment-page {
  min-height: 100vh;
  background-color: #0D1117;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.payment-container { width: 100%; max-width: 420px; }

.payment-header { text-align: center; margin-bottom: 32px; }

.payment-logo { margin-bottom: 16px; }

.payment-title {
  font-size: 24px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 8px 0;
  letter-spacing: -0.02em;
}

.payment-subtitle {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
}

.payment-card {
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  padding: 24px;
}

/* Features */
.features-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 8px;
  margin-bottom: 20px;
}

.feature-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  color: #C9D1D9;
}

.feature-row svg { flex-shrink: 0; }

/* Price */
.price-badge {
  text-align: center;
  padding: 20px;
  margin-bottom: 20px;
  background: linear-gradient(135deg, #0D1117 0%, rgba(47, 129, 247, 0.08) 100%);
  border: 1px solid rgba(47, 129, 247, 0.3);
  border-radius: 12px;
}

.price-label {
  display: block;
  font-size: 13px;
  color: #8B949E;
  margin-bottom: 4px;
}

.price-amount {
  font-size: 36px;
  font-weight: 700;
  color: #F0883E;
  letter-spacing: -0.02em;
}

.price-note {
  display: block;
  font-size: 12px;
  color: #6E7681;
  margin-top: 4px;
}

/* QR Code */
.qr-section { text-align: center; margin-bottom: 16px; }

.qr-wrapper {
  display: inline-block;
  padding: 16px;
  background-color: #FFFFFF;
  border-radius: 12px;
  margin-bottom: 12px;
}

.qr-image {
  display: block;
  width: 200px;
  height: 200px;
  image-rendering: pixelated;
}

.qr-wrapper-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  width: 200px;
  height: 200px;
  margin: 0 auto 12px auto;
  background-color: #21262D;
  border-radius: 12px;
  justify-content: center;
  color: #8B949E;
  font-size: 13px;
}

.qr-hint { font-size: 13px; color: #8B949E; margin: 0; }

/* Status Indicator */
.status-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 16px;
  margin-bottom: 20px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 8px;
  font-size: 13px;
  color: #8B949E;
}

.status-indicator.order_created { border-color: #2F81F7; color: #58A6FF; }

.pulse-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  background-color: #2F81F7;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }

/* Guide */
.payment-guide {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 8px;
  margin-bottom: 16px;
}

.guide-step {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: #E6EDF3;
}

.step-num {
  width: 24px; height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 50%;
  font-size: 12px;
  font-weight: 600;
  color: #2F81F7;
  flex-shrink: 0;
}

.step-text { color: #C9D1D9; }

.auto-note {
  font-size: 12px;
  color: #6E7681;
  text-align: center;
  margin: 0;
}

/* States */
.status-box, .verified-box, .error-box { text-align: center; padding: 48px 24px; }

.spinner {
  width: 32px; height: 32px;
  border: 3px solid #21262D;
  border-top-color: #2F81F7;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin: 0 auto 16px auto;
}

@keyframes spin { to { transform: rotate(360deg); } }

.verified-title {
  font-size: 20px;
  font-weight: 600;
  color: #3FB950;
  margin: 16px 0 8px 0;
}

.verified-desc {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
}

.error-text {
  font-size: 14px;
  color: #F85149;
  margin: 16px 0 24px 0;
  line-height: 1.5;
}

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 24px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid;
  font-family: inherit;
  transition: all 0.15s ease;
}

.btn-secondary { background-color: #21262D; border-color: #30363D; color: #E6EDF3; }
.btn-secondary:hover { background-color: #30363D; }

.btn-logout {
  display: block;
  width: 100%;
  padding: 12px;
  margin-top: 16px;
  background: transparent;
  border: none;
  color: #8B949E;
  font-size: 13px;
  cursor: pointer;
  text-align: center;
  transition: color 0.15s ease;
  font-family: inherit;
}

.btn-logout:hover { color: #F85149; }
</style>
