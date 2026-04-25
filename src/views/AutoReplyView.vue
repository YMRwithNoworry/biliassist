<template>
  <div class="auto-reply-page">
    <!-- Header -->
    <header class="page-header">
      <div class="header-content">
        <button class="btn-back" @click="goBack">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7"/>
          </svg>
        </button>
        <h1 class="header-title">自动回复</h1>
        <div class="header-spacer"></div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="page-main">
      <!-- Enable Toggle -->
      <div class="card">
        <div class="setting-row">
          <div class="setting-info">
            <div class="setting-title">启用自动回复</div>
            <div class="setting-desc">开启后将自动回复粉丝消息</div>
          </div>
          <label class="toggle">
            <input type="checkbox" v-model="enabled" @change="save" />
            <span class="toggle-track"></span>
          </label>
        </div>
      </div>

      <!-- Autostart -->
      <div class="card">
        <div class="setting-row">
          <div class="setting-info">
            <div class="setting-title">开机自启</div>
            <div class="setting-desc">系统启动后自动在后台运行</div>
          </div>
          <label class="toggle">
            <input type="checkbox" v-model="autostartEnabled" @change="toggleAutostart" />
            <span class="toggle-track"></span>
          </label>
        </div>
      </div>

      <!-- Sources -->
      <div class="card">
        <div class="card-header">
          <h2 class="card-title">回复来源</h2>
        </div>
        <div class="chips">
          <button
            class="chip"
            :class="{ active: sources.includes('comment') }"
            @click="toggleSource('comment')"
          >
            <svg v-if="sources.includes('comment')" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            视频评论
          </button>
          <button
            class="chip"
            :class="{ active: sources.includes('directMessage') }"
            @click="toggleSource('directMessage')"
          >
            <svg v-if="sources.includes('directMessage')" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            私信
          </button>
          <button
            class="chip"
            :class="{ active: sources.includes('follow') }"
            @click="toggleSource('follow')"
          >
            <svg v-if="sources.includes('follow')" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            粉丝关注
          </button>
        </div>
      </div>

      <!-- Message Content -->
      <div class="card">
        <div class="card-header">
          <h2 class="card-title">回复内容</h2>
        </div>
        <div class="form-field">
          <textarea
            v-model="message"
            class="textarea"
            placeholder="输入自动回复内容..."
            rows="4"
            @blur="save"
          ></textarea>
          <div class="field-hint">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="16" x2="12" y2="12"/>
              <line x1="12" y1="8" x2="12.01" y2="8"/>
            </svg>
            支持变量：{用户名}、{时间}
          </div>
        </div>
      </div>

      <!-- Settings -->
      <div class="card">
        <div class="card-header">
          <h2 class="card-title">回复设置</h2>
        </div>
        
        <div class="setting-row">
          <div class="setting-info">
            <div class="setting-title">每个用户只回复一次</div>
            <div class="setting-desc">避免重复发送消息给同一用户</div>
          </div>
          <label class="toggle">
            <input type="checkbox" v-model="replyOnlyOnce" @change="save" />
            <span class="toggle-track"></span>
          </label>
        </div>

        <div class="form-field" style="margin-top: 20px;">
          <label class="field-label">回复间隔（秒）</label>
          <input
            type="number"
            v-model.number="interval"
            class="input"
            min="1"
            max="3600"
            @blur="save"
          />
        </div>
      </div>

      <!-- Actions -->
      <div class="actions">
        <button class="btn btn-secondary btn-block" @click="testReply">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
          </svg>
          测试回复
        </button>
        <button class="btn btn-primary btn-block" @click="manualReply">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
          </svg>
          立即回复视频评论
        </button>
      </div>

      <!-- History -->
      <div class="card">
        <div class="card-header">
          <h2 class="card-title">回复记录</h2>
          <span class="badge" v-if="history.length > 0">{{ history.length }}</span>
        </div>
        
        <div v-if="history.length === 0" class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
          </svg>
          <p>暂无回复记录</p>
        </div>
        
        <div v-else class="history-list">
          <div v-for="(r, i) in history" :key="i" class="history-item">
            <div class="history-meta">
              <div class="history-user">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
                  <circle cx="12" cy="7" r="4"/>
                </svg>
                {{ r.user }}
              </div>
              <span class="history-source" :class="r.source">
                {{ sourceLabel(r.source) }}
              </span>
            </div>
            <p class="history-msg">{{ r.message }}</p>
            <p class="history-time">{{ r.time }}</p>
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

const enabled = ref(true)
const message = ref('感谢您的留言！我会尽快回复。')
const interval = ref(60)
const replyOnlyOnce = ref(true)
const sources = ref(['comment', 'directMessage', 'follow'])
const history = ref([])
const autostartEnabled = ref(false)

const sourceLabel = (s) => {
  const map = { comment: '评论', directMessage: '私信', follow: '关注' }
  return map[s] || s
}

const toggleSource = (src) => {
  const idx = sources.value.indexOf(src)
  if (idx >= 0) {
    sources.value.splice(idx, 1)
  } else {
    sources.value.push(src)
  }
  save()
}

const load = async () => {
  try {
    const s = await invoke('get_auto_reply_settings')
    enabled.value = s.enabled
    message.value = s.message
    interval.value = s.interval
    replyOnlyOnce.value = s.replyOnlyOnce
    sources.value = s.sources || ['comment', 'directMessage', 'follow']
    history.value = s.history || []
  } catch (e) {
    console.error('加载设置失败:', e)
  }
  try {
    autostartEnabled.value = await invoke('get_autostart_status')
  } catch (e) {
    console.error('加载开机自启状态失败:', e)
  }
}

const save = async () => {
  try {
    await invoke('save_auto_reply_settings', {
      settings: {
        enabled: enabled.value,
        message: message.value,
        interval: interval.value,
        replyOnlyOnce: replyOnlyOnce.value,
        sources: sources.value,
        history: history.value || [],
      }
    })
  } catch (e) {
    console.error('保存设置失败:', e)
  }
}

const testReply = async () => {
  try {
    const result = await invoke('test_auto_reply')
    alert(result)
  } catch (e) {
    console.error('测试失败:', e)
  }
}

const manualReply = async () => {
  try {
    const result = await invoke('manual_reply_video_comments')
    alert(result)
  } catch (e) {
    alert('执行失败: ' + e)
    console.error('手动回复失败:', e)
  }
}

const toggleAutostart = async () => {
  try {
    await invoke('set_autostart', { enabled: autostartEnabled.value })
  } catch (e) {
    console.error('设置开机自启失败:', e)
    autostartEnabled.value = !autostartEnabled.value
  }
}

const goBack = () => router.push('/')

onMounted(() => load())
</script>

<style scoped>
/* Octo Code Design System - Dark Theme */

.auto-reply-page {
  min-height: 100vh;
  background-color: #0D1117;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
  display: flex;
  flex-direction: column;
}

/* Header */
.page-header {
  background-color: #161B22;
  border-bottom: 1px solid #30363D;
  padding: 0 24px;
  flex-shrink: 0;
}

.header-content {
  max-width: 768px;
  margin: 0 auto;
  height: 64px;
  display: flex;
  align-items: center;
  gap: 16px;
}

.btn-back {
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

.btn-back:hover {
  background-color: #21262D;
  border-color: #484F58;
  color: #E6EDF3;
}

.header-title {
  font-size: 20px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0;
}

.header-spacer {
  width: 36px;
}

/* Main Content */
.page-main {
  flex: 1;
  max-width: 768px;
  width: 100%;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* Cards */
.card {
  background-color: #161B22;
  border: 1px solid #30363D;
  border-radius: 12px;
  padding: 20px;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.card-title {
  font-size: 14px;
  font-weight: 600;
  color: #E6EDF3;
  margin: 0;
}

.badge {
  padding: 2px 8px;
  background-color: #388BFD1A;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  color: #2F81F7;
}

/* Setting Row */
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-info {
  flex: 1;
  min-width: 0;
}

.setting-title {
  font-size: 14px;
  font-weight: 500;
  color: #E6EDF3;
  margin-bottom: 4px;
}

.setting-desc {
  font-size: 12px;
  color: #8B949E;
}

/* Toggle */
.toggle {
  position: relative;
  display: inline-block;
  width: 48px;
  height: 24px;
  cursor: pointer;
  flex-shrink: 0;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-track {
  position: absolute;
  inset: 0;
  background-color: #30363D;
  border-radius: 24px;
  transition: background-color 0.2s ease;
}

.toggle-track::before {
  content: '';
  position: absolute;
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background-color: #E6EDF3;
  border-radius: 50%;
  transition: transform 0.2s ease;
}

.toggle input:checked + .toggle-track {
  background-color: #238636;
}

.toggle input:checked + .toggle-track::before {
  transform: translateX(24px);
}

/* Chips */
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  background-color: #21262D;
  border: 1px solid #30363D;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 500;
  color: #8B949E;
  cursor: pointer;
  transition: all 0.15s ease;
}

.chip:hover {
  border-color: #484F58;
  color: #E6EDF3;
}

.chip.active {
  background-color: #388BFD1A;
  border-color: #2F81F7;
  color: #2F81F7;
}

/* Form Fields */
.form-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field-label {
  font-size: 13px;
  font-weight: 500;
  color: #E6EDF3;
}

.field-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #8B949E;
}

/* Input */
.input {
  width: 100%;
  padding: 10px 14px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 6px;
  font-size: 14px;
  color: #E6EDF3;
  transition: all 0.15s ease;
  box-sizing: border-box;
}

.input:focus {
  outline: none;
  border-color: #2F81F7;
  box-shadow: 0 0 0 3px rgba(47, 129, 247, 0.15);
}

.input::placeholder {
  color: #6E7681;
}

/* Textarea */
.textarea {
  width: 100%;
  padding: 12px 14px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 6px;
  font-size: 14px;
  line-height: 1.5;
  color: #E6EDF3;
  resize: vertical;
  transition: all 0.15s ease;
  box-sizing: border-box;
  font-family: inherit;
}

.textarea:focus {
  outline: none;
  border-color: #2F81F7;
  box-shadow: 0 0 0 3px rgba(47, 129, 247, 0.15);
}

.textarea::placeholder {
  color: #6E7681;
}

/* Actions */
.actions {
  display: flex;
  flex-direction: column;
  gap: 12px;
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
}

.btn-primary {
  background-color: #238636;
  border-color: rgba(46, 160, 67, 0.4);
  color: #FFFFFF;
}

.btn-primary:hover {
  background-color: #2EA043;
}

.btn-secondary {
  background-color: #21262D;
  border-color: #30363D;
  color: #E6EDF3;
}

.btn-secondary:hover {
  background-color: #30363D;
}

.btn-block {
  width: 100%;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  color: #8B949E;
}

.empty-state svg {
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-state p {
  margin: 0;
  font-size: 14px;
}

/* History */
.history-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.history-item {
  padding: 16px;
  background-color: #0D1117;
  border: 1px solid #30363D;
  border-radius: 8px;
}

.history-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.history-user {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: #E6EDF3;
}

.history-user svg {
  color: #8B949E;
}

.history-source {
  padding: 3px 10px;
  border-radius: 12px;
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
}

.history-source.comment {
  background-color: #388BFD1A;
  color: #2F81F7;
}

.history-source.directMessage {
  background-color: #3FB9501A;
  color: #3FB950;
}

.history-source.follow {
  background-color: #A371F71A;
  color: #A371F7;
}

.history-msg {
  font-size: 14px;
  color: #C9D1D9;
  line-height: 1.5;
  margin: 0 0 8px 0;
}

.history-time {
  font-size: 12px;
  color: #8B949E;
  margin: 0;
}

/* Responsive */
@media (max-width: 640px) {
  .page-header {
    padding: 0 16px;
  }
  
  .page-main {
    padding: 16px;
  }
  
  .card {
    padding: 16px;
  }
}
</style>
