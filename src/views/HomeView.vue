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

    <!-- Main Content -->
    <main class="home-main">
      <!-- Hero -->
      <section class="hero">
        <h1 class="hero-title">B站账号管理工具</h1>
        <p class="hero-subtitle">扫码登录 · 自动回复 · 多账号管理</p>
      </section>

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

        <div class="nav-card" @click="goToAutoReply">
          <div class="nav-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
            </svg>
          </div>
          <div class="nav-content">
            <h2 class="nav-title">自动回复</h2>
            <p class="nav-desc">配置智能自动回复规则</p>
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
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const auth = useAuthStore()

const goToLogin = () => router.push('/login')
const goToAccounts = () => router.push('/accounts')
const goToAutoReply = () => router.push('/auto-reply')
const goToSponsor = () => router.push('/sponsor')

const logout = async () => {
  await auth.signOut()
  router.push('/auth')
}
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
</style>