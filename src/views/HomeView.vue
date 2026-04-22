<template>
  <div class="page">
    <div class="page-body slide-up">
      <div class="hero">
        <h1 class="hero-title">B站账号管理</h1>
        <p class="hero-subtitle">扫码登录 · 自动回复 · 多账号管理</p>
      </div>

      <div class="nav-grid">
        <div class="nav-card card" @click="goToLogin">
          <div class="nav-icon">📱</div>
          <div class="nav-content">
            <h2 class="nav-title">扫码登录</h2>
            <p class="nav-desc">使用B站App扫码快速登录</p>
          </div>
          <span class="nav-arrow">→</span>
        </div>

        <div class="nav-card card" @click="goToAccounts">
          <div class="nav-icon">👥</div>
          <div class="nav-content">
            <h2 class="nav-title">账号管理</h2>
            <p class="nav-desc">多账号切换与管理</p>
          </div>
          <span class="nav-arrow">→</span>
        </div>

        <div class="nav-card card" @click="goToAutoReply">
          <div class="nav-icon">💬</div>
          <div class="nav-content">
            <h2 class="nav-title">自动回复</h2>
            <p class="nav-desc">配置智能自动回复规则</p>
          </div>
          <span class="nav-arrow">→</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { useRouter } from 'vue-router'
const router = useRouter()
const goToLogin = () => router.push('/login')
const goToAccounts = () => router.push('/accounts')
const goToAutoReply = () => router.push('/auto-reply')
</script>

<style scoped>
.hero {
  text-align: center;
  padding: var(--sp-2xl) 0 var(--sp-xl);
}

.hero-title {
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.44px;
  color: var(--text-primary);
}

.hero-subtitle {
  font-size: 16px;
  color: var(--text-secondary);
  margin-top: var(--sp-sm);
}

.nav-grid {
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
}

.nav-card {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  cursor: pointer;
  padding: var(--sp-lg);
}

.nav-icon {
  font-size: 28px;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--surface-light);
  border-radius: var(--radius-circle);
  flex-shrink: 0;
}

.nav-content {
  flex: 1;
  min-width: 0;
}

.nav-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.nav-desc {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.nav-arrow {
  font-size: 20px;
  color: var(--text-secondary);
  flex-shrink: 0;
  transition: transform 0.15s;
}

.nav-card:hover .nav-arrow {
  transform: translateX(4px);
  color: var(--rausch-red);
}
</style>
