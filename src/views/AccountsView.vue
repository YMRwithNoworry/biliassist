<template>
  <div class="accounts-page">
    <!-- Header -->
    <header class="page-header">
      <button class="btn-back" @click="goBack">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M7.78 12.53a.75.75 0 0 1-1.06 0L2.47 8.28a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 1.06L4.81 7h7.44a.75.75 0 0 1 0 1.5H4.81l2.97 2.97a.75.75 0 0 1 0 1.06Z"/>
        </svg>
        返回
      </button>
      <h1 class="page-title">账号管理</h1>
      <button class="btn-add" @click="goToLogin">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"/>
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
        添加
      </button>
    </header>

    <!-- Main Content -->
    <main class="accounts-main">
      <!-- Loading State -->
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>加载中...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="accounts.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
            <circle cx="9" cy="7" r="4"/>
            <line x1="23" y1="11" x2="17" y2="11"/>
          </svg>
        </div>
        <h2 class="empty-title">暂无账号</h2>
        <p class="empty-desc">请先扫码登录添加账号</p>
        <button class="btn-primary" @click="goToLogin">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          扫码登录
        </button>
      </div>

      <!-- Account List -->
      <div v-else class="accounts-list">
        <div
          v-for="account in accounts"
          :key="account.uid"
          class="account-card"
          :class="{ active: account.active }"
        >
          <div class="account-avatar">
            <img v-if="account.avatar" :src="account.avatar" class="avatar-img" />
            <span v-else class="avatar-text">{{ account.name.charAt(0).toUpperCase() }}</span>
            <div v-if="account.active" class="active-indicator"></div>
          </div>
          <div class="account-info">
            <h3 class="account-name">{{ account.name }}</h3>
            <p class="account-uid">UID: {{ account.uid }}</p>
          </div>
          <div class="account-actions">
            <button
              v-if="!account.active"
              class="btn-action btn-switch"
              @click="activateAccount(account.uid)"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
              切换
            </button>
            <div v-else class="current-badge">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
              当前
            </div>
            <button class="btn-action btn-delete" @click="deleteAccount(account.uid)">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6"/>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const router = useRouter()
const accounts = ref([])
const loading = ref(true)

const loadAccounts = async () => {
  try {
    loading.value = true
    const result = await invoke('get_accounts')
    accounts.value = Array.isArray(result) ? result : []
  } catch (error) {
    console.error('加载账号失败:', error)
    accounts.value = []
  } finally {
    loading.value = false
  }
}

const activateAccount = async (uid) => {
  try {
    await invoke('activate_account', { uid })
    await loadAccounts()
  } catch (error) {
    console.error('激活账号失败:', error)
  }
}

const deleteAccount = async (uid) => {
  if (!confirm('确定要删除此账号吗？')) return
  try {
    await invoke('delete_account', { uid })
    await loadAccounts()
  } catch (error) {
    console.error('删除账号失败:', error)
  }
}

const goBack = () => router.push('/')
const goToLogin = () => router.push('/login')

onMounted(() => loadAccounts())
</script>

<style scoped>
/* Octo Code Design System - Dark Theme */

.accounts-page {
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

.btn-add {
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

.btn-add:hover {
  background-color: #2EA043;
}

/* Main */
.accounts-main {
  max-width: 768px;
  margin: 0 auto;
  padding: 32px 24px;
}

/* Loading State */
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px;
  color: #8B949E;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid #30363D;
  border-top-color: #2F81F7;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 80px 24px;
}

.empty-icon {
  color: #30363D;
  margin-bottom: 24px;
}

.empty-title {
  font-size: 20px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 8px 0;
}

.empty-desc {
  font-size: 14px;
  color: #8B949E;
  margin: 0 0 32px 0;
}

.btn-primary {
  display: flex;
  align-items: center;
  gap: 8px;
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

.btn-primary:hover {
  background-color: #2EA043;
}

/* Account List */
.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.account-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 20px;
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  transition: all 0.15s ease;
}

.account-card:hover {
  border-color: #484F58;
  background-color: #1C2128;
}

.account-card.active {
  border-left: 3px solid #238636;
  background-color: rgba(46, 160, 67, 0.1);
}

.account-avatar {
  position: relative;
  width: 48px;
  height: 48px;
  flex-shrink: 0;
}

.avatar-img {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
}

.avatar-text {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #2F81F7, #388BFD);
  border-radius: 50%;
  font-size: 18px;
  font-weight: 600;
  color: #FFFFFF;
}

.active-indicator {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 14px;
  height: 14px;
  background-color: #238636;
  border: 2px solid #161B22;
  border-radius: 50%;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-size: 16px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 4px 0;
}

.account-uid {
  font-size: 13px;
  color: #8B949E;
  margin: 0;
}

.account-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-action {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border: 1px solid #30363D;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-switch {
  background-color: transparent;
  color: #C9D1D9;
}

.btn-switch:hover {
  background-color: #21262D;
  border-color: #484F58;
}

.current-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  background-color: rgba(46, 160, 67, 0.15);
  border: 1px solid rgba(46, 160, 67, 0.4);
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  color: #3FB950;
}

.btn-delete {
  padding: 8px;
  background-color: transparent;
  color: #F85149;
  border-color: #30363D;
}

.btn-delete:hover {
  background-color: rgba(248, 81, 73, 0.1);
  border-color: #F85149;
}

/* Responsive */
@media (max-width: 640px) {
  .page-header {
    padding: 12px 16px;
  }
  
  .accounts-main {
    padding: 24px 16px;
  }
  
  .account-card {
    padding: 14px 16px;
  }
  
  .account-avatar {
    width: 40px;
    height: 40px;
  }
  
  .avatar-text {
    font-size: 16px;
  }
}
</style>