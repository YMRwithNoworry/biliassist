<template>
  <div class="sponsor-page">
    <!-- Header -->
    <header class="page-header">
      <button class="btn-back" @click="goBack">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M7.78 12.53a.75.75 0 0 1-1.06 0L2.47 8.28a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 1.06L4.81 7h7.44a.75.75 0 0 1 0 1.5H4.81l2.97 2.97a.75.75 0 0 1 0 1.06Z"/>
        </svg>
        返回
      </button>
      <h1 class="page-title">赞助支持</h1>
      <div class="header-spacer"></div>
    </header>

    <!-- Main Content -->
    <main class="sponsor-content">
      <!-- Intro Card -->
      <div class="card intro-card">
        <div class="intro-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
          </svg>
        </div>
        <h2 class="intro-title">支持 BilibiliAccountManager</h2>
        <p class="intro-text">
          感谢您的使用！如果您觉得这个工具对您有帮助，欢迎通过以下方式赞助支持开发。
          您的支持将帮助我们持续改进功能、修复问题，并推出更多实用特性。
        </p>
      </div>

      <!-- QR Code Card -->
      <div class="card qr-card">
        <div class="qr-header">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="7" height="7" rx="1"/>
            <rect x="14" y="3" width="7" height="7" rx="1"/>
            <rect x="3" y="14" width="7" height="7" rx="1"/>
            <path d="M14 14h7v7h-7z"/>
            <path d="M17 17h1v1h-1z"/>
          </svg>
          <span>扫码赞助</span>
        </div>
        <div class="qr-container" :class="{ zoomed: isZoomed }" @click="toggleZoom">
          <img :src="qrImageUrl" alt="赞助二维码" class="qr-image" :class="{ zoomed: isZoomed }"/>
        </div>
        <p class="qr-hint">使用微信或支付宝扫描上方二维码</p>
        <p class="qr-zoom-hint">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/>
            <path d="M21 21l-4.35-4.35"/>
            <line x1="11" y1="8" x2="11" y2="14"/>
            <line x1="8" y1="11" x2="14" y2="11"/>
          </svg>
          {{ isZoomed ? '单击缩小' : '单击放大' }}
        </p>
      </div>

      <!-- Features Card -->
      <div class="card features-card">
        <h3 class="features-title">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
          </svg>
          您的支持将用于
        </h3>
        <ul class="features-list">
          <li class="feature-item">
            <span class="feature-dot"></span>
            <span>持续维护与 Bug 修复</span>
          </li>
          <li class="feature-item">
            <span class="feature-dot"></span>
            <span>新功能开发与优化</span>
          </li>
          <li class="feature-item">
            <span class="feature-dot"></span>
            <span>服务器与基础设施</span>
          </li>
          <li class="feature-item">
            <span class="feature-dot"></span>
            <span>文档与教程制作</span>
          </li>
        </ul>
      </div>

      <!-- Thanks Card -->
      <div class="card thanks-card">
        <p class="thanks-text">❤️ 感谢您的每一份支持！</p>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const isZoomed = ref(false)

// 使用 Vite 的 URL 导入方式处理静态资源
const qrImageUrl = new URL('/sponsor-qr.png', import.meta.url).href

const goBack = () => router.push('/')

const toggleZoom = () => {
  isZoomed.value = !isZoomed.value
}
</script>

<style scoped>
/* Octo Code Design System - Dark Theme */

.sponsor-page {
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
  position: sticky;
  top: 0;
  z-index: 100;
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

/* Main Content */
.sponsor-content {
  max-width: 768px;
  margin: 0 auto;
  padding: 32px 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

/* Cards */
.card {
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  padding: 24px;
  transition: border-color 0.15s ease;
}

.card:hover {
  border-color: #484F58;
}

/* Intro Card */
.intro-card {
  text-align: center;
}

.intro-icon {
  color: #D29922;
  margin-bottom: 16px;
}

.intro-title {
  font-size: 24px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 12px 0;
}

.intro-text {
  font-size: 14px;
  color: #8B949E;
  line-height: 1.6;
  margin: 0;
}

/* QR Card */
.qr-card {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.qr-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #3FB950;
  font-size: 16px;
  font-weight: 500;
  margin-bottom: 20px;
}

.qr-container {
  background-color: #FFFFFF;
  padding: 16px;
  border-radius: 12px;
  margin-bottom: 16px;
  cursor: zoom-in;
  transition: all 0.3s ease;
}

.qr-container.zoomed {
  cursor: zoom-out;
  padding: 24px;
}

.qr-image {
  max-width: 200px;
  max-height: 200px;
  width: auto;
  height: auto;
  display: block;
  transition: all 0.3s ease;
}

.qr-image.zoomed {
  max-width: 360px;
  max-height: 360px;
}

.qr-hint {
  font-size: 13px;
  color: #8B949E;
  margin: 0 0 8px 0;
}

.qr-zoom-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 12px;
  color: #6E7681;
  margin: 0;
}

/* Features Card */
.features-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 16px 0;
}

.features-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.feature-item {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
  color: #C9D1D9;
}

.feature-dot {
  width: 8px;
  height: 8px;
  background-color: #2F81F7;
  border-radius: 50%;
  flex-shrink: 0;
}

/* Thanks Card */
.thanks-card {
  text-align: center;
  background-color: #1C2128;
}

.thanks-text {
  font-size: 16px;
  color: #E6EDF3;
  margin: 0;
}

/* Responsive */
@media (max-width: 640px) {
  .page-header {
    padding: 12px 16px;
  }
  
  .page-title {
    font-size: 18px;
  }
  
  .sponsor-content {
    padding: 24px 16px;
  }
  
  .card {
    padding: 20px;
  }
  
  .qr-image {
    max-width: 180px;
    max-height: 180px;
  }
  
  .qr-image.zoomed {
    max-width: 280px;
    max-height: 280px;
  }
}
</style>