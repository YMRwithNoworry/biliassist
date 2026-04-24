<template>
  <div class="auth-page">
    <div class="auth-container">
      <!-- Logo -->
      <div class="auth-header">
        <div class="auth-logo">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#2F81F7" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
        </div>
        <h1 class="auth-title">BilibiliAccountManager</h1>
        <p class="auth-subtitle">登录以继续使用</p>
      </div>

      <!-- Auth Card -->
      <div class="auth-card">
        <!-- Tabs -->
        <div class="auth-tabs">
          <button
            class="auth-tab"
            :class="{ active: mode === 'otp' }"
            @click="mode = 'otp'; error = ''"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="4" width="20" height="16" rx="2"/>
              <path d="M12 11v4M12 7h.01"/>
            </svg>
            验证码登录
          </button>
          <button
            class="auth-tab"
            :class="{ active: mode === 'password' }"
            @click="mode = 'password'; error = ''"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            </svg>
            密码登录
          </button>
        </div>

        <!-- Error Message -->
        <div v-if="error" class="auth-error">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          {{ error }}
        </div>

        <!-- Success Message -->
        <div v-if="success" class="auth-success">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          {{ success }}
        </div>

        <!-- OTP Mode -->
        <template v-if="mode === 'otp'">
          <div class="form-group">
            <label class="form-label">邮箱地址</label>
            <input
              v-model="email"
              type="email"
              class="form-input"
              placeholder="请输入邮箱"
              :disabled="otpSent"
              @keyup.enter="sendOtp"
            />
          </div>

          <div v-if="!otpSent" class="form-action">
            <button
              class="btn btn-primary btn-block"
              :disabled="!isValidEmail || sending"
              @click="sendOtp"
            >
              <svg v-if="sending" class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 6v6l4 2"/>
              </svg>
              {{ sending ? '发送中...' : '发送验证码' }}
            </button>
          </div>

          <template v-if="otpSent">
            <div class="form-group">
              <label class="form-label">验证码</label>
              <input
                v-model="otpCode"
                type="text"
                class="form-input"
                placeholder="请输入6位验证码"
                maxlength="6"
                inputmode="numeric"
                @keyup.enter="verifyOtpCode"
              />
            </div>

            <div class="form-action">
              <button
                class="btn btn-primary btn-block"
                :disabled="otpCode.length !== 6 || verifying"
                @click="verifyOtpCode"
              >
                <svg v-if="verifying" class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <path d="M12 6v6l4 2"/>
                </svg>
                {{ verifying ? '验证中...' : '登录' }}
              </button>
            </div>

            <div class="form-footer">
              <button
                class="btn-text"
                :disabled="countdown > 0 || sending"
                @click="resendOtp"
              >
                {{ countdown > 0 ? `${countdown}s 后重新发送` : '重新发送验证码' }}
              </button>
              <button class="btn-text" @click="resetOtp">
                更换邮箱
              </button>
            </div>

            <p class="form-note">
              验证码已发送至 <strong>{{ email }}</strong>
            </p>
          </template>
        </template>

        <!-- Password Mode -->
        <template v-if="mode === 'password'">
          <div class="form-group">
            <label class="form-label">邮箱地址</label>
            <input
              v-model="email"
              type="email"
              class="form-input"
              placeholder="请输入邮箱"
              @keyup.enter="handlePasswordAction"
            />
          </div>

          <div class="form-group">
            <label class="form-label">{{ isRegister ? '设置密码' : '密码' }}</label>
            <div class="password-field">
              <input
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                class="form-input"
                :placeholder="isRegister ? '至少6位密码' : '请输入密码'"
                @keyup.enter="handlePasswordAction"
              />
              <button class="password-toggle" @click="showPassword = !showPassword" type="button">
                <svg v-if="showPassword" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                  <circle cx="12" cy="12" r="3"/>
                </svg>
                <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/>
                  <line x1="1" y1="1" x2="23" y2="23"/>
                </svg>
              </button>
            </div>
          </div>

          <div v-if="isRegister" class="password-strength">
            <div class="strength-bar">
              <div class="strength-fill" :class="strengthClass" :style="{ width: strengthPercent + '%' }"></div>
            </div>
            <span class="strength-text" :class="strengthClass">{{ strengthLabel }}</span>
          </div>

          <div class="form-action">
            <button
              class="btn btn-primary btn-block"
              :disabled="!isValidEmail || !password || password.length < (isRegister ? 6 : 1) || loggingIn"
              @click="handlePasswordAction"
            >
              <svg v-if="loggingIn" class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 6v6l4 2"/>
              </svg>
              {{ loggingIn ? '处理中...' : (isRegister ? '注册并登录' : '登录') }}
            </button>
          </div>

          <div class="form-footer">
            <button class="btn-text" @click="isRegister = !isRegister">
              {{ isRegister ? '已有账号？去登录' : '没有账号？去注册' }}
            </button>
          </div>
        </template>
      </div>

      <!-- Footer -->
      <p class="auth-footer">
        继续即表示同意我们的服务条款
      </p>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const auth = useAuthStore()

const mode = ref('otp')
const email = ref('')
const password = ref('')
const otpCode = ref('')
const showPassword = ref(false)
const isRegister = ref(false)

const error = ref('')
const success = ref('')

const sending = ref(false)
const verifying = ref(false)
const loggingIn = ref(false)
const otpSent = ref(false)
const countdown = ref(0)
let countdownTimer = null

const isValidEmail = computed(() => {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value)
})

const isValidPassword = computed(() => {
  return password.value.length >= 6
})

const strengthPercent = computed(() => {
  const pwd = password.value
  if (!pwd) return 0
  let score = 0
  if (pwd.length >= 8) score += 25
  if (pwd.length >= 12) score += 15
  if (/[a-z]/.test(pwd)) score += 20
  if (/[A-Z]/.test(pwd)) score += 20
  if (/\d/.test(pwd)) score += 10
  if (/[^a-zA-Z0-9]/.test(pwd)) score += 10
  return Math.min(100, score)
})

const strengthClass = computed(() => {
  if (strengthPercent.value >= 80) return 'strong'
  if (strengthPercent.value >= 40) return 'medium'
  if (strengthPercent.value > 0) return 'weak'
  return ''
})

const strengthLabel = computed(() => {
  if (strengthPercent.value >= 80) return '强'
  if (strengthPercent.value >= 40) return '中'
  if (strengthPercent.value > 0) return '弱'
  return ''
})

const startCountdown = () => {
  countdown.value = 60
  countdownTimer = setInterval(() => {
    countdown.value--
    if (countdown.value <= 0) {
      clearInterval(countdownTimer)
      countdownTimer = null
    }
  }, 1000)
}

const sendOtp = async () => {
  if (!isValidEmail.value || sending.value) return
  sending.value = true
  error.value = ''
  success.value = ''
  try {
    await auth.signInWithOtp(email.value)
    otpSent.value = true
    startCountdown()
    success.value = '验证码已发送到您的邮箱'
  } catch (e) {
    error.value = e.message || '发送验证码失败'
  } finally {
    sending.value = false
  }
}

const resendOtp = async () => {
  if (countdown.value > 0 || sending.value) return
  await sendOtp()
}

const resetOtp = () => {
  otpSent.value = false
  otpCode.value = ''
  error.value = ''
  success.value = ''
  if (countdownTimer) {
    clearInterval(countdownTimer)
    countdownTimer = null
  }
  countdown.value = 0
}

const verifyOtpCode = async () => {
  if (otpCode.value.length !== 6 || verifying.value) return
  verifying.value = true
  error.value = ''
  try {
    await auth.verifyOtp(email.value, otpCode.value)
    router.push('/')
  } catch (e) {
    error.value = e.message || '验证码错误或已过期'
  } finally {
    verifying.value = false
  }
}

const handlePasswordAction = async () => {
  if (!isValidEmail.value || !password.value || loggingIn.value) return

  if (isRegister.value && password.value.length < 6) {
    error.value = '密码至少需要6位'
    return
  }

  loggingIn.value = true
  error.value = ''
  success.value = ''

  try {
    if (isRegister.value) {
      const result = await auth.signUpWithPassword(email.value, password.value)
      if (result.session) {
        router.push('/')
      } else {
        success.value = '注册成功！请检查邮箱确认（如已确认可直接登录）'
        isRegister.value = false
      }
    } else {
      await auth.signInWithPassword(email.value, password.value)
      router.push('/')
    }
  } catch (e) {
    error.value = e.message || '操作失败'
  } finally {
    loggingIn.value = false
  }
}
</script>

<style scoped>
.auth-page {
  min-height: 100vh;
  background-color: #0D1117;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.auth-container {
  width: 100%;
  max-width: 420px;
}

/* Header */
.auth-header {
  text-align: center;
  margin-bottom: 32px;
}

.auth-logo {
  margin-bottom: 16px;
}

.auth-title {
  font-size: 24px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0 0 8px 0;
  letter-spacing: -0.02em;
}

.auth-subtitle {
  font-size: 14px;
  color: #8B949E;
  margin: 0;
}

/* Card */
.auth-card {
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  padding: 24px;
}

/* Tabs */
.auth-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 24px;
  background-color: #21262D;
  border-radius: 8px;
  padding: 4px;
}

.auth-tab {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 10px 12px;
  background: transparent;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  color: #8B949E;
  cursor: pointer;
  transition: all 0.15s ease;
}

.auth-tab:hover {
  color: #E6EDF3;
}

.auth-tab.active {
  background-color: #161B22;
  color: #E6EDF3;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

/* Messages */
.auth-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background-color: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.3);
  border-radius: 6px;
  font-size: 13px;
  color: #F85149;
  margin-bottom: 16px;
}

.auth-success {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background-color: rgba(63, 185, 80, 0.1);
  border: 1px solid rgba(63, 185, 80, 0.3);
  border-radius: 6px;
  font-size: 13px;
  color: #3FB950;
  margin-bottom: 16px;
}

/* Form */
.form-group {
  margin-bottom: 20px;
}

.form-label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: #E6EDF3;
  margin-bottom: 8px;
}

.form-input {
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

.form-input:focus {
  outline: none;
  border-color: #2F81F7;
  box-shadow: 0 0 0 3px rgba(47, 129, 247, 0.15);
}

.form-input::placeholder {
  color: #6E7681;
}

.form-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.form-action {
  margin-top: 8px;
}

.form-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  margin-top: 16px;
}

.form-note {
  font-size: 12px;
  color: #8B949E;
  text-align: center;
  margin: 16px 0 0 0;
}

.form-note strong {
  color: #E6EDF3;
}

/* Password Field */
.password-field {
  position: relative;
}

.password-field .form-input {
  padding-right: 44px;
}

.password-toggle {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: #8B949E;
  cursor: pointer;
  transition: color 0.15s ease;
}

.password-toggle:hover {
  color: #E6EDF3;
}

/* Password Strength */
.password-strength {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
}

.strength-bar {
  flex: 1;
  height: 4px;
  background-color: #21262D;
  border-radius: 2px;
  overflow: hidden;
}

.strength-fill {
  height: 100%;
  border-radius: 2px;
  transition: all 0.3s ease;
}

.strength-fill.weak {
  background-color: #F85149;
}

.strength-fill.medium {
  background-color: #D29922;
}

.strength-fill.strong {
  background-color: #3FB950;
}

.strength-text {
  font-size: 12px;
  font-weight: 500;
  min-width: 20px;
  text-align: center;
}

.strength-text.weak {
  color: #F85149;
}

.strength-text.medium {
  color: #D29922;
}

.strength-text.strong {
  color: #3FB950;
}

/* Buttons */
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

.btn-text {
  background: transparent;
  border: none;
  color: #2F81F7;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  padding: 4px 8px;
  transition: color 0.15s ease;
  font-family: inherit;
}

.btn-text:hover:not(:disabled) {
  color: #58A6FF;
  text-decoration: underline;
}

.btn-text:disabled {
  color: #6E7681;
  cursor: not-allowed;
}

/* Spinner */
.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Footer */
.auth-footer {
  text-align: center;
  font-size: 12px;
  color: #6E7681;
  margin: 24px 0 0 0;
}

/* Responsive */
@media (max-width: 640px) {
  .auth-page {
    padding: 16px;
  }

  .auth-card {
    padding: 20px;
  }

  .auth-title {
    font-size: 20px;
  }
}
</style>
