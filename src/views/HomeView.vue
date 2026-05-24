<template>
  <div class="home-page">
    <!-- Header -->
    <header class="home-header">
      <div class="header-content">
        <div class="logo">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#00AEEC" stroke-width="2">
            <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>
          </svg>
          <span>BilibiliAccountManager</span>
        </div>
        <div class="header-actions">
          <span class="header-user">{{ auth.user?.email }}</span>
          <button v-if="auth.isPlus" class="tier-badge tier-plus" disabled>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            Plus
          </button>
          <button v-else class="tier-badge tier-basic" @click="showKeyDialog = true">
            Basic
          </button>
          <button class="btn-sponsor" @click="goToSponsor">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
            </svg>
            赞助
          </button>
          <button class="btn-logout" @click="logout" title="退出登录">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>
              <polyline points="16 17 21 12 16 7"/>
              <line x1="21" y1="12" x2="9" y2="12"/>
            </svg>
          </button>
        </div>
      </div>
    </header>

    <!-- Key Input Dialog -->
    <div v-if="showKeyDialog" class="dialog-overlay" @click.self="showKeyDialog = false">
      <div class="dialog-card">
        <div class="dialog-header">
          <h2 class="dialog-title">输入激活密钥</h2>
          <button class="dialog-close" @click="showKeyDialog = false">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
        <div class="dialog-body">
          <input
            v-model="licenseKey"
            type="text"
            class="dialog-input"
            placeholder="请输入激活密钥"
            @keyup.enter="submitKey"
            :disabled="keySubmitting"
          />
          <div v-if="keyError" class="dialog-error">{{ keyError }}</div>
          <div v-if="keySuccess" class="dialog-success">{{ keySuccess }}</div>
          <button
            class="btn btn-primary btn-block"
            :disabled="!licenseKey || keySubmitting"
            @click="submitKey"
          >
            {{ keySubmitting ? '验证中...' : '激活' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Purchase Reminder Dialog -->
    <div v-if="showPurchaseDialog" class="dialog-overlay" @click.self="closePurchaseDialog">
      <div class="purchase-dialog">
        <div class="purchase-header">
          <div class="purchase-icon-wrap" :class="auth.isPlus ? 'icon-plus' : 'icon-basic'">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
            </svg>
          </div>
          <div>
            <h2 class="purchase-title">{{ auth.isPlus ? '获取激活码' : '升级到 Plus' }}</h2>
            <p class="purchase-subtitle">{{ auth.isPlus ? '为其他账号或朋友购买' : '解锁自动回复、自动点赞等全部功能' }}</p>
          </div>
          <button class="dialog-close purchase-close" @click="closePurchaseDialog">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <div class="purchase-body">
          <a href="#" @click.prevent="openPurchaseLink" class="purchase-link-card">
            <div class="purchase-link-header">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
                <polyline points="15 3 21 3 21 9"/>
                <line x1="10" y1="14" x2="21" y2="3"/>
              </svg>
              <span class="purchase-link-label">爱发电购买页面</span>
            </div>
            <span class="purchase-link-url">ifdian.net/a/Alkut</span>
          </a>

          <div class="purchase-plan">
            <div class="purchase-plan-name">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#3FB950" stroke-width="2">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
              B站账号自动化工具 plus 账户兑换码独立方案
            </div>
            <p class="purchase-plan-desc">付款后自动收到激活码，在下方输入即可升级</p>
          </div>

          <div class="purchase-steps">
            <div class="purchase-step">
              <span class="step-num">1</span>
              <span>打开上方链接，选择方案并付款</span>
            </div>
            <div class="purchase-step">
              <span class="step-num">2</span>
              <span>收到激活码后，点击下方按钮输入</span>
            </div>
            <div class="purchase-step">
              <span class="step-num">3</span>
              <span>激活成功，享受 Plus 全部功能</span>
            </div>
          </div>
        </div>

        <div class="purchase-footer">
          <button class="btn btn-outline" @click="closePurchaseDialog">
            {{ auth.isPlus ? '稍后再说' : '暂不升级' }}
          </button>
          <button class="btn btn-primary" @click="closePurchaseDialog(); showKeyDialog = true">
            输入激活码
          </button>
        </div>
      </div>
    </div>

    <!-- Main Content -->
    <main class="home-main">
      <!-- Hero -->
      <section class="hero">
        <h1 class="hero-title">B站账号管理工具</h1>
        <p class="hero-subtitle">扫码登录 · 自动回复 · 多账号管理</p>
      </section>

      <!-- Upgrade Reminder for Basic Users -->
      <div v-if="!auth.isPlus && showReminder" class="reminder-banner">
        <div class="reminder-content">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#F0883E" stroke-width="2">
            <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
            <line x1="12" y1="9" x2="12" y2="13"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <div class="reminder-text">
            <span class="reminder-title">升级到 Plus</span>
            <span class="reminder-desc">使用激活密钥解锁自动回复、自动点赞等全部功能</span>
          </div>
          <button class="reminder-btn" @click="showKeyDialog = true">输入密钥</button>
          <button class="reminder-close" @click="dismissReminder" title="关闭">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </div>

      <!-- Navigation Cards -->
      <nav class="nav-grid">
        <div class="nav-card" @click="goToLogin">
          <div class="nav-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            </svg>
          </div>
          <div class="nav-content">
            <h2 class="nav-title">扫码登录</h2>
            <p class="nav-desc">使用B站App扫码快速登录</p>
          </div>
          <svg class="nav-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 18l6-6-6-6"/>
          </svg>
        </div>

        <div class="nav-card" @click="goToAccounts">
          <div class="nav-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
              <circle cx="9" cy="7" r="4"/>
              <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
              <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
            </svg>
          </div>
          <div class="nav-content">
            <h2 class="nav-title">账号管理</h2>
            <p class="nav-desc">多账号切换与管理</p>
          </div>
          <svg class="nav-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 18l6-6-6-6"/>
          </svg>
        </div>

        <div class="nav-card" :class="{ disabled: !auth.isPlus }" @click="goToAutoReply">
          <div class="nav-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
            </svg>
          </div>
          <div class="nav-content">
            <h2 class="nav-title">自动回复</h2>
            <p class="nav-desc">{{ auth.isPlus ? '配置智能自动回复规则' : '需升级到 Plus 后使用' }}</p>
          </div>
          <svg class="nav-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 18l6-6-6-6"/>
          </svg>
        </div>
      </nav>

      <!-- Footer -->
      <footer class="home-footer">
        <p>BilibiliAccountManager v0.2.4</p>
      </footer>
    </main>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { invoke } from '@tauri-apps/api/core'
import { supabase } from '../lib/supabase'

const router = useRouter()
const auth = useAuthStore()

const showKeyDialog = ref(false)
const licenseKey = ref('')
const keySubmitting = ref(false)
const keyError = ref('')
const keySuccess = ref('')
const showReminder = ref(true)
let reminderTimer = null

const REMINDER_INTERVAL = 30 * 60 * 1000

const PURCHASE_DISMISSED_KEY = 'biliassist_purchase_dialog_dismissed'
const showPurchaseDialog = ref(false)

function openPurchaseLink() {
  window.open('https://www.ifdian.net/a/Alkut?tab=home', '_blank')
}

function closePurchaseDialog() {
  showPurchaseDialog.value = false
  if (auth.isPlus) {
    try { localStorage.setItem(PURCHASE_DISMISSED_KEY, 'true') } catch {}
  }
}

async function submitKey() {
  if (!licenseKey.value || keySubmitting.value) return
  keySubmitting.value = true
  keyError.value = ''
  keySuccess.value = ''
  try {
    await invoke('verify_license', { licenseKey: licenseKey.value })

    const { error } = await supabase
      .from('user_tiers')
      .upsert(
        { user_id: auth.user.id, tier: 'plus' },
        { onConflict: 'user_id' }
      )

    if (error) throw error

    keySuccess.value = '激活成功！'
    auth.userTier = 'plus'
    auth.saveLocalActivation()
    setTimeout(() => { showKeyDialog.value = false }, 1000)
  } catch (e) {
    keyError.value = typeof e === 'string' ? e : (e?.message || '激活失败，请检查密钥')
  } finally {
    keySubmitting.value = false
  }
}

function dismissReminder() {
  showReminder.value = false
  clearTimeout(reminderTimer)
  reminderTimer = setTimeout(() => {
    showReminder.value = true
    startReminderTimer()
  }, REMINDER_INTERVAL)
}

function startReminderTimer() {
  clearTimeout(reminderTimer)
  reminderTimer = setTimeout(() => {
    showReminder.value = true
    startReminderTimer()
  }, REMINDER_INTERVAL)
}

const goToLogin = () => {
  if (!auth.isPlus) {
    showKeyDialog.value = true
    return
  }
  router.push('/login')
}
const goToAccounts = () => router.push('/accounts')
const goToAutoReply = () => {
  if (!auth.isPlus) {
    showKeyDialog.value = true
    return
  }
  router.push('/auto-reply')
}
const goToSponsor = () => router.push('/sponsor')

const logout = async () => {
  await auth.signOut()
  router.push('/auth')
}

onMounted(() => {
  startReminderTimer()
  if (auth.isPlus) {
    const dismissed = localStorage.getItem(PURCHASE_DISMISSED_KEY)
    showPurchaseDialog.value = dismissed !== 'true'
  } else {
    showPurchaseDialog.value = true
  }
})

onUnmounted(() => {
  clearTimeout(reminderTimer)
})
</script>

<style scoped>
/* Octo Code Design System - Dark Theme */

.home-page {
  min-height: 100vh;
  background-color: #0D1117;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
  display: flex;
  flex-direction: column;
}

/* Header */
.home-header {
  background-color: #161B22;
  border-bottom: 1px solid #30363D;
  padding: 0 24px;
}

.header-content {
  max-width: 1280px;
  margin: 0 auto;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.logo {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 18px;
  font-weight: 600;
  color: #E6EDF3;
}

.btn-sponsor {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background-color: #238636;
  border: 1px solid rgba(46, 160, 67, 0.4);
  border-radius: 6px;
  color: #FFFFFF;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-sponsor:hover {
  background-color: #2EA043;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-user {
  font-size: 13px;
  color: #8B949E;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Tier Badge */
.tier-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid;
  font-family: inherit;
  white-space: nowrap;
}

.tier-badge:disabled {
  cursor: default;
  opacity: 1;
}

.tier-plus {
  background-color: rgba(63, 185, 80, 0.1);
  border-color: rgba(63, 185, 80, 0.3);
  color: #3FB950;
}

.tier-basic {
  background-color: rgba(248, 129, 62, 0.1);
  border-color: rgba(248, 129, 62, 0.3);
  color: #F0883E;
}

.tier-basic:hover {
  background-color: rgba(248, 129, 62, 0.2);
  border-color: #F0883E;
}

.btn-logout {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: transparent;
  border: 1px solid #30363D;
  border-radius: 6px;
  color: #8B949E;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-logout:hover {
  background-color: rgba(248, 81, 73, 0.1);
  border-color: #F85149;
  color: #F85149;
}

/* Reminder Banner */
.reminder-banner {
  margin-bottom: 24px;
  background-color: rgba(248, 129, 62, 0.08);
  border: 1px solid rgba(248, 129, 62, 0.3);
  border-radius: 10px;
  overflow: hidden;
}

.reminder-content {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
}

.reminder-text {
  flex: 1;
  min-width: 0;
}

.reminder-title {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: #F0883E;
  margin-bottom: 2px;
}

.reminder-desc {
  display: block;
  font-size: 12px;
  color: #8B949E;
}

.reminder-btn {
  padding: 7px 16px;
  background-color: #F0883E;
  border: none;
  border-radius: 6px;
  color: #FFFFFF;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
  flex-shrink: 0;
  font-family: inherit;
}

.reminder-btn:hover {
  background-color: #F29D5C;
}

.reminder-close {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: #6E7681;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s ease;
}

.reminder-close:hover {
  background-color: rgba(248, 129, 62, 0.2);
  color: #F0883E;
}

/* Main */
.home-main {
  flex: 1;
  max-width: 768px;
  width: 100%;
  margin: 0 auto;
  padding: 48px 24px;
  display: flex;
  flex-direction: column;
}

/* Hero */
.hero {
  text-align: center;
  margin-bottom: 48px;
}

.hero-title {
  font-size: 32px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 12px 0;
  letter-spacing: -0.02em;
}

.hero-subtitle {
  font-size: 16px;
  color: #8B949E;
  margin: 0;
}

/* Nav Grid */
.nav-grid {
  display: flex;
  flex-direction: column;
  gap: 16px;
  flex: 1;
}

.nav-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px 24px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.nav-card:hover {
  border-color: #484F58;
  background-color: #1C2128;
}

.nav-card.disabled {
  opacity: 0.6;
  cursor: default;
}

.nav-card.disabled:hover {
  border-color: #30363D;
  background-color: #161B22;
}

.nav-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 12px;
  color: #2F81F7;
  flex-shrink: 0;
}

.nav-content {
  flex: 1;
  min-width: 0;
}

.nav-title {
  font-size: 16px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 4px 0;
}

.nav-desc {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
}

.nav-arrow {
  color: #8B949E;
  flex-shrink: 0;
  transition: transform 0.15s ease, color 0.15s ease;
}

.nav-card:hover .nav-arrow {
  transform: translateX(4px);
  color: #E6EDF3;
}

.nav-card.disabled .nav-arrow {
  display: none;
}

/* Footer */
.home-footer {
  text-align: center;
  padding-top: 48px;
  margin-top: auto;
}

.home-footer p {
  font-size: 12px;
  color: #8B949E;
  margin: 0;
}

/* Dialog */
.dialog-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 24px;
}

.dialog-card {
  width: 100%;
  max-width: 400px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 0;
}

.dialog-title {
  font-size: 16px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0;
}

.dialog-close {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: #8B949E;
  cursor: pointer;
  transition: all 0.15s ease;
}

.dialog-close:hover {
  background-color: #21262D;
  color: #E6EDF3;
}

.dialog-body {
  padding: 20px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.dialog-input {
  width: 100%;
  padding: 12px 14px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 6px;
  font-size: 14px;
  color: #E6EDF3;
  transition: all 0.15s ease;
  box-sizing: border-box;
  font-family: inherit;
}

.dialog-input:focus {
  outline: none;
  border-color: #2F81F7;
  box-shadow: 0 0 0 3px rgba(47, 129, 247, 0.15);
}

.dialog-input:disabled {
  opacity: 0.6;
}

.dialog-error {
  padding: 10px 14px;
  background-color: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.3);
  border-radius: 6px;
  font-size: 13px;
  color: #F85149;
}

.dialog-success {
  padding: 10px 14px;
  background-color: rgba(63, 185, 80, 0.1);
  border: 1px solid rgba(63, 185, 80, 0.3);
  border-radius: 6px;
  font-size: 13px;
  color: #3FB950;
}

.btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 12px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid;
  font-family: inherit;
}

.btn-primary {
  background-color: #238636;
  border-color: rgba(46, 160, 67, 0.4);
  color: #FFFFFF;
}

.btn-primary:hover:not(:disabled) {
  background-color: #2EA043;
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-block {
  width: 100%;
}

/* Responsive */
@media (max-width: 640px) {
  .home-header {
    padding: 0 16px;
  }

  .logo span {
    display: none;
  }

  .header-user {
    display: none;
  }

  .hero-title {
    font-size: 24px;
  }

  .home-main {
    padding: 32px 16px;
  }

  .nav-card {
    padding: 16px 20px;
  }

  .nav-icon {
    width: 40px;
    height: 40px;
  }
}
/* Purchase Dialog */
.purchase-dialog {
  width: 100%;
  max-width: 480px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 16px;
  overflow: hidden;
  animation: purchase-slide-in 0.3s ease;
}

@keyframes purchase-slide-in {
  from { opacity: 0; transform: scale(0.95) translateY(10px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.purchase-header {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 24px 24px 0;
  position: relative;
}

.purchase-icon-wrap {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.purchase-icon-wrap.icon-basic {
  background: linear-gradient(135deg, #F0883E 0%, #DB6D28 100%);
  color: #FFFFFF;
}

.purchase-icon-wrap.icon-plus {
  background: linear-gradient(135deg, #2F81F7 0%, #1F6FEB 100%);
  color: #FFFFFF;
}

.purchase-title {
  font-size: 18px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 4px 0;
}

.purchase-subtitle {
  font-size: 13px;
  color: #8B949E;
  margin: 0;
}

.purchase-close {
  position: absolute;
  top: 20px;
  right: 20px;
}

.purchase-body {
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.purchase-link-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 10px;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.purchase-link-card:hover {
  border-color: #2F81F7;
  background-color: #161B22;
}

.purchase-link-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #2F81F7;
}

.purchase-link-label {
  font-size: 14px;
  font-weight: 500;
  color: #2F81F7;
}

.purchase-link-url {
  font-size: 12px;
  color: #6E7681;
  padding-left: 28px;
}

.purchase-plan {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  background-color: rgba(63, 185, 80, 0.06);
  border: 1px solid rgba(63, 185, 80, 0.2);
  border-radius: 10px;
}

.purchase-plan-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #E6EDF3;
}

.purchase-plan-desc {
  font-size: 12px;
  color: #8B949E;
  margin: 0;
  padding-left: 24px;
}

.purchase-steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.purchase-step {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: #C9D1D9;
}

.step-num {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 50%;
  font-size: 12px;
  font-weight: 600;
  color: #8B949E;
  flex-shrink: 0;
}

.purchase-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 24px 24px;
}

.btn-outline {
  background: transparent;
  border: 1px solid #30363D;
  color: #C9D1D9;
}

.btn-outline:hover {
  background-color: #21262D;
  border-color: #484F58;
}
/* Purchase Dialog */
.purchase-dialog {
  width: 100%;
  max-width: 480px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 16px;
  overflow: hidden;
  animation: purchase-slide-in 0.3s ease;
}

@keyframes purchase-slide-in {
  from { opacity: 0; transform: scale(0.95) translateY(10px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.purchase-header {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 24px 24px 0;
  position: relative;
}

.purchase-icon-wrap {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.purchase-icon-wrap.icon-basic {
  background: linear-gradient(135deg, #F0883E 0%, #DB6D28 100%);
  color: #FFFFFF;
}

.purchase-icon-wrap.icon-plus {
  background: linear-gradient(135deg, #2F81F7 0%, #1F6FEB 100%);
  color: #FFFFFF;
}

.purchase-title {
  font-size: 18px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 4px 0;
}

.purchase-subtitle {
  font-size: 13px;
  color: #8B949E;
  margin: 0;
}

.purchase-close {
  position: absolute;
  top: 20px;
  right: 20px;
}

.purchase-body {
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.purchase-link-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 10px;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.purchase-link-card:hover {
  border-color: #2F81F7;
  background-color: #161B22;
}

.purchase-link-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #2F81F7;
}

.purchase-link-label {
  font-size: 14px;
  font-weight: 500;
  color: #2F81F7;
}

.purchase-link-url {
  font-size: 12px;
  color: #6E7681;
  padding-left: 28px;
}

.purchase-plan {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  background-color: rgba(63, 185, 80, 0.06);
  border: 1px solid rgba(63, 185, 80, 0.2);
  border-radius: 10px;
}

.purchase-plan-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #E6EDF3;
}

.purchase-plan-desc {
  font-size: 12px;
  color: #8B949E;
  margin: 0;
  padding-left: 24px;
}

.purchase-steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.purchase-step {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: #C9D1D9;
}

.step-num {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 50%;
  font-size: 12px;
  font-weight: 600;
  color: #8B949E;
  flex-shrink: 0;
}

.purchase-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 24px 24px;
}

.btn-outline {
  background: transparent;
  border: 1px solid #30363D;
  color: #C9D1D9;
}

.btn-outline:hover {
  background-color: #21262D;
  border-color: #484F58;
}
/* Purchase Dialog */
.purchase-dialog {
  width: 100%;
  max-width: 480px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 16px;
  overflow: hidden;
  animation: purchase-slide-in 0.3s ease;
}
@keyframes purchase-slide-in {
  from { opacity: 0; transform: scale(0.95) translateY(10px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
.purchase-header {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 24px 24px 0;
  position: relative;
}
.purchase-icon-wrap {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.purchase-icon-wrap.icon-basic {
  background: linear-gradient(135deg, #F0883E 0%, #DB6D28 100%);
  color: #FFFFFF;
}
.purchase-icon-wrap.icon-plus {
  background: linear-gradient(135deg, #2F81F7 0%, #1F6FEB 100%);
  color: #FFFFFF;
}
.purchase-title {
  font-size: 18px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 4px 0;
}
.purchase-subtitle {
  font-size: 13px;
  color: #8B949E;
  margin: 0;
}
.purchase-close {
  position: absolute;
  top: 20px;
  right: 20px;
}
.purchase-body {
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.purchase-link-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 10px;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s ease;
}
.purchase-link-card:hover {
  border-color: #2F81F7;
  background-color: #161B22;
}
.purchase-link-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #2F81F7;
}
.purchase-link-label {
  font-size: 14px;
  font-weight: 500;
  color: #2F81F7;
}
.purchase-link-url {
  font-size: 12px;
  color: #6E7681;
  padding-left: 28px;
}
.purchase-plan {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
  background-color: rgba(63, 185, 80, 0.06);
  border: 1px solid rgba(63, 185, 80, 0.2);
  border-radius: 10px;
}
.purchase-plan-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #E6EDF3;
}
.purchase-plan-desc {
  font-size: 12px;
  color: #8B949E;
  margin: 0;
  padding-left: 24px;
}
.purchase-steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.purchase-step {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: #C9D1D9;
}
.step-num {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 50%;
  font-size: 12px;
  font-weight: 600;
  color: #8B949E;
  flex-shrink: 0;
}
.purchase-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 24px 24px;
}
.btn-outline {
  background: transparent;
  border: 1px solid #30363D;
  color: #C9D1D9;
}
.btn-outline:hover {
  background-color: #21262D;
  border-color: #484F58;
}
</style>



