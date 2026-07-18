<template>
  <div class="auto-reply-page">
    <header class="page-header">
      <div class="header-content">
        <button class="icon-button" type="button" aria-label="返回首页" title="返回首页" @click="goBack">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
        </button>
        <h1>自动回复</h1>
        <span class="save-state" :class="saveState">
          {{ saveStateLabel }}
        </span>
      </div>
    </header>

    <main class="page-main">
      <section v-if="!auth.isPlus" class="access-panel">
        <svg width="38" height="38" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="3" y="11" width="18" height="11" rx="2" />
          <path d="M7 11V7a5 5 0 0 1 10 0v4" />
        </svg>
        <h2>需要 Plus 等级</h2>
        <p>自动回复功能仅限 Plus 用户使用。</p>
        <button class="button primary" type="button" @click="goBack">返回首页输入密钥</button>
      </section>

      <section v-else-if="loading" class="loading-panel" aria-live="polite">
        <span class="spinner" aria-hidden="true"></span>
        <span>正在加载自动回复设置</span>
      </section>

      <section v-else-if="loadError" class="error-panel" role="alert">
        <strong>设置加载失败</strong>
        <span>{{ loadError }}</span>
        <button class="button secondary" type="button" @click="load">重试</button>
      </section>

      <template v-else>
        <section class="settings-surface">
          <div class="surface-section overview-section">
            <div class="section-heading">
              <div>
                <h2>运行设置</h2>
                <p>统一控制自动回复服务的运行状态和检查频率。</p>
              </div>
            </div>

            <div class="setting-list">
              <div class="setting-row">
                <div class="setting-copy">
                  <strong>自动回复总开关</strong>
                  <span>关闭后暂停三个渠道的回复；评论点赞仍按评论区设置执行。</span>
                </div>
                <label class="toggle">
                  <input v-model="settings.enabled" type="checkbox" @change="save" />
                  <span class="toggle-track"></span>
                </label>
              </div>

              <div class="setting-row">
                <div class="setting-copy">
                  <strong>开机自启</strong>
                  <span>系统启动后在后台运行。</span>
                </div>
                <label class="toggle">
                  <input v-model="autostartEnabled" type="checkbox" @change="toggleAutostart" />
                  <span class="toggle-track"></span>
                </label>
              </div>
            </div>

            <div class="compact-field">
              <label for="poll-interval">检查间隔</label>
              <div class="number-control">
                <input
                  id="poll-interval"
                  v-model.number="settings.interval"
                  type="number"
                  min="1"
                  max="3600"
                  inputmode="numeric"
                  @change="saveInterval"
                />
                <span>秒</span>
              </div>
            </div>
          </div>

          <div class="surface-section provider-section">
            <div class="section-heading">
              <div>
                <h2>AI 服务</h2>
                <p>接口、模型和密钥由三个回复渠道共用。</p>
              </div>
            </div>

            <div class="preset-group" aria-label="AI 服务预设">
              <button type="button" @click="applyPreset('deepseek')">DeepSeek</button>
              <button type="button" @click="applyPreset('openai')">OpenAI</button>
              <button type="button" @click="applyPreset('anthropic')">Anthropic</button>
              <button type="button" @click="applyPreset('ollama')">Ollama</button>
            </div>

            <div class="provider-grid">
              <div class="form-field">
                <label for="ai-api-format">接口格式</label>
                <select
                  id="ai-api-format"
                  v-model="settings.aiProvider.apiFormat"
                  @change="save"
                >
                  <option value="openAiChatCompletions">OpenAI Chat Completions</option>
                  <option value="openAiCompletions">OpenAI Completions</option>
                  <option value="openAiResponses">OpenAI Responses</option>
                  <option value="anthropicMessages">Anthropic Messages</option>
                </select>
              </div>

              <div class="form-field wide-field">
                <label for="ai-base-url">API Base URL</label>
                <input
                  id="ai-base-url"
                  v-model.trim="settings.aiProvider.baseUrl"
                  type="text"
                  placeholder="https://api.openai.com/v1"
                  @blur="save"
                />
              </div>

              <div class="form-field">
                <label for="ai-model">模型名称</label>
                <input
                  id="ai-model"
                  v-model.trim="settings.aiProvider.model"
                  type="text"
                  placeholder="gpt-4o-mini"
                  @blur="save"
                />
              </div>

              <div class="form-field wide-field">
                <label for="ai-api-key">API Key</label>
                <div class="input-with-action">
                  <input
                    id="ai-api-key"
                    v-model="settings.aiProvider.apiKey"
                    :type="showApiKey ? 'text' : 'password'"
                    placeholder="sk-..."
                    autocomplete="off"
                    @blur="save"
                  />
                  <button
                    class="field-icon-button"
                    type="button"
                    :aria-label="showApiKey ? '隐藏 API Key' : '显示 API Key'"
                    :title="showApiKey ? '隐藏 API Key' : '显示 API Key'"
                    @click="showApiKey = !showApiKey"
                  >
                    <svg v-if="!showApiKey" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z" />
                      <circle cx="12" cy="12" r="3" />
                    </svg>
                    <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="m3 3 18 18M10.6 10.7a2 2 0 0 0 2.7 2.7M9.9 5.1A10.8 10.8 0 0 1 12 5c6.5 0 10 7 10 7a17.3 17.3 0 0 1-2.1 3.2M6.2 6.2C3.5 8.1 2 12 2 12s3.5 7 10 7a10.6 10.6 0 0 0 4.1-.8" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>

            <div class="inline-action-row">
              <button class="button secondary" type="button" :disabled="aiTesting" @click="testAiReply">
                <span v-if="aiTesting" class="button-spinner" aria-hidden="true"></span>
                <svg v-else width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="m13 2-2 8h7l-7 12 2-8H6l7-12Z" />
                </svg>
                {{ aiTesting ? '测试中' : '测试 AI 服务' }}
              </button>
              <span v-if="aiTestResult" class="inline-result" :class="{ error: aiTestError }">
                {{ aiTestResult }}
              </span>
            </div>
          </div>

          <div class="surface-section channel-section">
            <div class="section-heading channel-heading">
              <div>
                <h2>分渠道配置</h2>
                <p>回复内容、AI 提示词和回复策略互不影响。</p>
              </div>
            </div>

            <div class="channel-tabs" role="tablist" aria-label="自动回复渠道">
              <button
                v-for="tab in channelTabs"
                :id="`channel-tab-${tab.key}`"
                :key="tab.key"
                class="channel-tab"
                :class="{ active: activeChannel === tab.key }"
                type="button"
                role="tab"
                :aria-selected="activeChannel === tab.key"
                :aria-controls="`channel-panel-${tab.key}`"
                @click="activeChannel = tab.key"
              >
                <span>{{ tab.label }}</span>
                <small :class="{ enabled: settings.channels[tab.key].enabled }">
                  {{ settings.channels[tab.key].enabled ? '已开启' : '已关闭' }}
                </small>
              </button>
            </div>

            <div
              :id="`channel-panel-${activeChannel}`"
              class="channel-panel"
              role="tabpanel"
              :aria-labelledby="`channel-tab-${activeChannel}`"
            >
              <div class="channel-title-row">
                <div>
                  <h3>{{ activeChannelMeta.label }}</h3>
                  <p>{{ activeChannelMeta.description }}</p>
                </div>
                <label class="toggle">
                  <input v-model="currentChannel.enabled" type="checkbox" @change="save" />
                  <span class="toggle-track"></span>
                </label>
              </div>

              <div v-if="activeChannel === 'comment'" class="setting-row channel-option-row">
                <div class="setting-copy">
                  <strong>自动点赞评论</strong>
                  <span>该开关可独立于评论自动回复运行。</span>
                </div>
                <label class="toggle">
                  <input v-model="settings.channels.comment.likeComments" type="checkbox" @change="save" />
                  <span class="toggle-track"></span>
                </label>
              </div>

              <div class="channel-form-grid">
                <div class="form-field full-width">
                  <label>回复策略</label>
                  <div class="segmented-control" role="group" :aria-label="`${activeChannelMeta.label}回复策略`">
                    <button
                      type="button"
                      :class="{ active: currentChannel.replyPolicy === 'perMessage' }"
                      @click="setReplyPolicy('perMessage')"
                    >
                      每条消息
                    </button>
                    <button
                      type="button"
                      :class="{ active: currentChannel.replyPolicy === 'oncePerUser' }"
                      @click="setReplyPolicy('oncePerUser')"
                    >
                      每个用户一次
                    </button>
                  </div>
                </div>

                <div class="form-field full-width">
                  <label :for="`${activeChannel}-message`">固定回复内容</label>
                  <textarea
                    :id="`${activeChannel}-message`"
                    v-model="currentChannel.message"
                    rows="4"
                    placeholder="输入自动回复内容"
                    @blur="save"
                  ></textarea>
                  <span class="field-hint">支持 {用户名}、{时间}</span>
                </div>
              </div>

              <div class="channel-ai-section">
                <div class="setting-row">
                  <div class="setting-copy">
                    <strong>使用 AI 生成回复</strong>
                    <span>关闭时使用当前渠道的固定回复内容。</span>
                  </div>
                  <label class="toggle">
                    <input v-model="currentChannel.ai.enabled" type="checkbox" @change="save" />
                    <span class="toggle-track"></span>
                  </label>
                </div>

                <div v-if="currentChannel.ai.enabled" class="ai-prompt-grid">
                  <div class="form-field full-width">
                    <label :for="`${activeChannel}-system-prompt`">系统提示词</label>
                    <textarea
                      :id="`${activeChannel}-system-prompt`"
                      v-model="currentChannel.ai.systemPrompt"
                      rows="3"
                      placeholder="设定当前渠道的角色、语气和回复风格"
                      @blur="save"
                    ></textarea>
                  </div>

                  <div class="form-field full-width">
                    <label :for="`${activeChannel}-prompt-template`">回复提示词模板</label>
                    <textarea
                      :id="`${activeChannel}-prompt-template`"
                      v-model="currentChannel.ai.promptTemplate"
                      rows="4"
                      placeholder="用户「{用户名}」通过{来源}发来：「{消息内容}」"
                      @blur="save"
                    ></textarea>
                    <div class="variable-row">
                      <span>插入变量</span>
                      <button type="button" @click="insertVar('用户名')">{用户名}</button>
                      <button type="button" @click="insertVar('消息内容')">{消息内容}</button>
                      <button type="button" @click="insertVar('来源')">{来源}</button>
                    </div>
                  </div>
                </div>
              </div>

              <div class="channel-actions">
                <button class="button secondary" type="button" :disabled="previewRunning" @click="testReply">
                  <span v-if="previewRunning" class="button-spinner" aria-hidden="true"></span>
                  <svg v-else width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15a4 4 0 0 1-4 4H8l-5 3V7a4 4 0 0 1 4-4h10a4 4 0 0 1 4 4v8Z" />
                  </svg>
                  预览全部模板
                </button>
                <button
                  v-if="activeChannel === 'comment'"
                  class="button primary"
                  type="button"
                  :disabled="manualRunning"
                  @click="manualReply"
                >
                  <span v-if="manualRunning" class="button-spinner" aria-hidden="true"></span>
                  <svg v-else width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="m13 2-2 8h7l-7 12 2-8H6l7-12Z" />
                  </svg>
                  立即处理视频评论
                </button>
              </div>

              <div v-if="actionResult" class="action-result" :class="{ error: actionError }" role="status">
                {{ actionResult }}
              </div>
            </div>
          </div>
        </section>

        <section class="history-surface">
          <div class="section-heading history-heading">
            <div>
              <h2>{{ activeChannelMeta.label }}回复记录</h2>
              <p>最近保存的当前渠道回复。</p>
            </div>
            <span class="count-badge">{{ channelHistory.length }}</span>
          </div>

          <div v-if="channelHistory.length === 0" class="empty-history">
            <svg width="42" height="42" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M21 15a4 4 0 0 1-4 4H8l-5 3V7a4 4 0 0 1 4-4h10a4 4 0 0 1 4 4v8Z" />
            </svg>
            <span>暂无回复记录</span>
          </div>

          <div v-else class="history-list">
            <article v-for="(item, index) in channelHistory" :key="`${item.time}-${index}`" class="history-item">
              <div class="history-meta">
                <strong>{{ item.user }}</strong>
                <time>{{ item.time }}</time>
              </div>
              <p>{{ item.message }}</p>
            </article>
          </div>
        </section>
      </template>
    </main>
  </div>
</template>

<script setup>
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useAuthStore } from '../stores/auth'

const DEFAULT_MESSAGE = '感谢您的留言！我会尽快回复。'
const DEFAULT_AI_BASE_URL = 'https://api.openai.com/v1'
const DEFAULT_AI_MODEL = 'gpt-4o-mini'

const router = useRouter()
const auth = useAuthStore()

const createChannel = (replyPolicy) => ({
  enabled: true,
  message: DEFAULT_MESSAGE,
  replyPolicy,
  ai: {
    enabled: false,
    systemPrompt: '',
    promptTemplate: '',
  },
})

const createSettings = () => ({
  enabled: true,
  interval: 60,
  aiProvider: {
    apiFormat: 'openAiChatCompletions',
    baseUrl: DEFAULT_AI_BASE_URL,
    model: DEFAULT_AI_MODEL,
    apiKey: '',
  },
  channels: {
    comment: {
      ...createChannel('perMessage'),
      likeComments: true,
    },
    directMessage: createChannel('oncePerUser'),
    follow: createChannel('oncePerUser'),
  },
  history: [],
})

const settings = reactive(createSettings())
const activeChannel = ref('comment')
const autostartEnabled = ref(false)
const loading = ref(true)
const loadError = ref('')
const saveState = ref('idle')
const showApiKey = ref(false)
const aiTesting = ref(false)
const aiTestResult = ref('')
const aiTestError = ref(false)
const previewRunning = ref(false)
const manualRunning = ref(false)
const actionResult = ref('')
const actionError = ref(false)

let saveQueue = Promise.resolve()
let saveVersion = 0
let loaded = false

const channelTabs = [
  { key: 'comment', label: '评论区', description: '处理视频下的新评论。' },
  { key: 'directMessage', label: '私信', description: '处理未读的一对一私信。' },
  { key: 'follow', label: '关注', description: '向新关注用户发送欢迎私信。' },
]

const activeChannelMeta = computed(
  () => channelTabs.find((tab) => tab.key === activeChannel.value) || channelTabs[0],
)
const currentChannel = computed(() => settings.channels[activeChannel.value])
const channelHistory = computed(() =>
  settings.history.filter((item) => item.source === activeChannel.value),
)
const saveStateLabel = computed(() => {
  const labels = {
    idle: '',
    saving: '保存中',
    saved: '已保存',
    error: '保存失败',
  }
  return labels[saveState.value]
})

const errorMessage = (error, fallback) => {
  if (typeof error === 'string') return error
  return error?.message || fallback
}

const normalizeChannel = (channel, fallbackPolicy, legacy) => ({
  enabled: channel?.enabled ?? legacy.enabled,
  message: channel?.message ?? legacy.message,
  replyPolicy: channel?.replyPolicy ?? fallbackPolicy,
  ai: {
    enabled: channel?.ai?.enabled ?? legacy.ai.enabled,
    systemPrompt: channel?.ai?.systemPrompt ?? legacy.ai.systemPrompt,
    promptTemplate: channel?.ai?.promptTemplate ?? legacy.ai.promptTemplate,
  },
})

const normalizeSettings = (raw = {}) => {
  const legacySources = raw.sources || ['comment', 'directMessage', 'follow']
  const legacyAi = raw.ai || {}
  const legacy = {
    enabled: true,
    message: raw.message || DEFAULT_MESSAGE,
    ai: {
      enabled: legacyAi.enabled ?? false,
      systemPrompt: legacyAi.systemPrompt || '',
      promptTemplate: legacyAi.promptTemplate || '',
    },
  }
  const oncePerUser = raw.replyOnlyOnce ?? true

  const comment = normalizeChannel(raw.channels?.comment, 'perMessage', {
    ...legacy,
    enabled: legacySources.includes('comment'),
  })
  const directMessage = normalizeChannel(
    raw.channels?.directMessage,
    oncePerUser ? 'oncePerUser' : 'perMessage',
    { ...legacy, enabled: legacySources.includes('directMessage') },
  )
  const follow = normalizeChannel(
    raw.channels?.follow,
    oncePerUser ? 'oncePerUser' : 'perMessage',
    { ...legacy, enabled: legacySources.includes('follow') },
  )

  return {
    enabled: raw.enabled ?? true,
    interval: raw.interval ?? 60,
    aiProvider: {
      apiFormat: raw.aiProvider?.apiFormat || legacyAi.apiFormat || 'openAiChatCompletions',
      baseUrl: raw.aiProvider?.baseUrl || legacyAi.baseUrl || DEFAULT_AI_BASE_URL,
      model: raw.aiProvider?.model || legacyAi.model || DEFAULT_AI_MODEL,
      apiKey: raw.aiProvider?.apiKey || legacyAi.apiKey || '',
    },
    channels: {
      comment: {
        ...comment,
        likeComments: raw.channels?.comment?.likeComments ?? raw.likeComments ?? true,
      },
      directMessage,
      follow,
    },
    history: Array.isArray(raw.history) ? raw.history : [],
  }
}

const load = async () => {
  loading.value = true
  loadError.value = ''
  try {
    const stored = await invoke('get_auto_reply_settings')
    Object.assign(settings, normalizeSettings(stored))
    loaded = true
  } catch (error) {
    loaded = false
    loadError.value = errorMessage(error, '无法读取本地设置')
  } finally {
    loading.value = false
  }

  try {
    autostartEnabled.value = await invoke('get_autostart_status')
  } catch (error) {
    console.error('加载开机自启状态失败:', error)
  }
}

const save = async () => {
  if (!loaded) return false

  const version = ++saveVersion
  const snapshot = JSON.parse(JSON.stringify(settings))
  saveState.value = 'saving'
  saveQueue = saveQueue
    .catch(() => undefined)
    .then(() => invoke('save_auto_reply_settings', { settings: snapshot }))

  try {
    await saveQueue
    if (version === saveVersion) saveState.value = 'saved'
    return true
  } catch (error) {
    if (version === saveVersion) saveState.value = 'error'
    console.error('保存设置失败:', error)
    return false
  }
}

const ensureSaved = async () => {
  if (!(await save())) {
    throw new Error('保存设置失败，请重试')
  }
}

const saveInterval = () => {
  const value = Number(settings.interval) || 60
  settings.interval = Math.min(3600, Math.max(1, Math.round(value)))
  save()
}

const applyPreset = (provider) => {
  const presets = {
    deepseek: {
      apiFormat: 'openAiChatCompletions',
      baseUrl: 'https://api.deepseek.com',
      model: 'deepseek-v4-flash',
    },
    openai: {
      apiFormat: 'openAiChatCompletions',
      baseUrl: DEFAULT_AI_BASE_URL,
      model: DEFAULT_AI_MODEL,
    },
    anthropic: {
      apiFormat: 'anthropicMessages',
      baseUrl: 'https://api.anthropic.com/v1',
      model: 'claude-sonnet-4-20250514',
    },
    ollama: {
      apiFormat: 'openAiChatCompletions',
      baseUrl: 'http://localhost:11434/v1',
      model: 'qwen2.5:7b',
    },
  }
  const preset = presets[provider]
  if (!preset) return
  Object.assign(settings.aiProvider, preset)
  save()
}

const setReplyPolicy = (policy) => {
  currentChannel.value.replyPolicy = policy
  save()
}

const insertVar = (variable) => {
  currentChannel.value.ai.promptTemplate += `{${variable}}`
  save()
}

const testAiReply = async () => {
  aiTesting.value = true
  aiTestResult.value = ''
  aiTestError.value = false
  try {
    await ensureSaved()
    aiTestResult.value = await invoke('test_ai_reply')
  } catch (error) {
    aiTestResult.value = errorMessage(error, '测试失败，请检查 AI 服务配置')
    aiTestError.value = true
  } finally {
    aiTesting.value = false
  }
}

const testReply = async () => {
  previewRunning.value = true
  actionResult.value = ''
  actionError.value = false
  try {
    await ensureSaved()
    actionResult.value = await invoke('test_auto_reply')
  } catch (error) {
    actionResult.value = errorMessage(error, '回复模板预览失败')
    actionError.value = true
  } finally {
    previewRunning.value = false
  }
}

const manualReply = async () => {
  manualRunning.value = true
  actionResult.value = ''
  actionError.value = false
  try {
    await ensureSaved()
    actionResult.value = await invoke('manual_reply_video_comments')
  } catch (error) {
    actionResult.value = errorMessage(error, '处理视频评论失败')
    actionError.value = true
  } finally {
    manualRunning.value = false
  }
}

const toggleAutostart = async () => {
  try {
    await invoke('set_autostart', { enabled: autostartEnabled.value })
  } catch (error) {
    autostartEnabled.value = !autostartEnabled.value
    console.error('设置开机自启失败:', error)
  }
}

const goBack = () => router.push('/')

onMounted(load)
</script>

<style scoped>
.auto-reply-page {
  min-height: 100vh;
  background: #0D1117;
  color: #E6EDF3;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
}

.page-header {
  position: sticky;
  top: 0;
  z-index: 20;
  border-bottom: 1px solid #30363D;
  background: rgba(22, 27, 34, 0.96);
  backdrop-filter: blur(12px);
}

.header-content {
  display: grid;
  grid-template-columns: 40px 1fr minmax(64px, auto);
  align-items: center;
  width: min(960px, calc(100% - 32px));
  min-height: 64px;
  margin: 0 auto;
  gap: 12px;
}

.header-content h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 650;
  letter-spacing: 0;
}

.icon-button,
.field-icon-button {
  display: inline-grid;
  place-items: center;
  width: 36px;
  height: 36px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: #C9D1D9;
  cursor: pointer;
}

.icon-button:hover,
.field-icon-button:hover {
  border-color: #484F58;
  background: #21262D;
}

.save-state {
  min-width: 64px;
  color: #8B949E;
  font-size: 13px;
  text-align: right;
}

.save-state.saved {
  color: #3FB950;
}

.save-state.error {
  color: #F85149;
}

.page-main {
  width: min(960px, calc(100% - 32px));
  margin: 0 auto;
  padding: 28px 0 48px;
}

.settings-surface,
.history-surface,
.access-panel,
.loading-panel,
.error-panel {
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #161B22;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.24);
}

.surface-section {
  padding: 24px;
}

.surface-section + .surface-section {
  border-top: 1px solid #30363D;
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  margin-bottom: 20px;
}

.section-heading h2,
.channel-title-row h3,
.access-panel h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 650;
  letter-spacing: 0;
}

.section-heading p,
.channel-title-row p,
.access-panel p {
  margin: 5px 0 0;
  color: #8B949E;
  font-size: 13px;
  line-height: 1.5;
}

.setting-list {
  display: grid;
  gap: 1px;
  overflow: hidden;
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #30363D;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 66px;
  gap: 24px;
  padding: 14px 16px;
  background: #161B22;
}

.setting-copy {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.setting-copy strong {
  font-size: 14px;
  font-weight: 600;
}

.setting-copy span {
  color: #8B949E;
  font-size: 12px;
  line-height: 1.45;
}

.toggle {
  position: relative;
  flex: 0 0 auto;
  width: 42px;
  height: 24px;
}

.toggle input {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
}

.toggle-track {
  position: absolute;
  inset: 0;
  border-radius: 999px;
  background: #484F58;
  cursor: pointer;
  transition: background-color 0.18s ease;
}

.toggle-track::after {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #E6EDF3;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
  content: '';
  transition: transform 0.18s ease;
}

.toggle input:checked + .toggle-track {
  background: #2F81F7;
}

.toggle input:checked + .toggle-track::after {
  transform: translateX(18px);
}

.toggle input:focus-visible + .toggle-track {
  outline: 3px solid rgba(47, 129, 247, 0.3);
  outline-offset: 2px;
}

.compact-field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  margin-top: 18px;
}

.compact-field label,
.form-field label {
  color: #C9D1D9;
  font-size: 13px;
  font-weight: 600;
}

.number-control {
  display: flex;
  align-items: center;
  overflow: hidden;
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #0D1117;
}

.number-control input {
  width: 92px;
  height: 38px;
  padding: 0 10px;
  border: 0;
  outline: 0;
  font: inherit;
  text-align: right;
}

.number-control span {
  display: grid;
  place-items: center;
  align-self: stretch;
  min-width: 42px;
  border-left: 1px solid #30363D;
  background: #21262D;
  color: #8B949E;
  font-size: 12px;
}

.preset-group,
.segmented-control {
  display: inline-grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(88px, 1fr);
  overflow: hidden;
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #0D1117;
}

.preset-group {
  margin-bottom: 18px;
}

.preset-group button,
.segmented-control button {
  min-height: 36px;
  padding: 7px 14px;
  border: 0;
  border-left: 1px solid #30363D;
  background: #0D1117;
  color: #C9D1D9;
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}

.preset-group button:first-child,
.segmented-control button:first-child {
  border-left: 0;
}

.preset-group button:hover,
.segmented-control button:hover {
  background: #21262D;
}

.segmented-control button.active {
  background: #1F6FEB;
  color: #FFFFFF;
  font-weight: 600;
}

.provider-grid,
.channel-form-grid,
.ai-prompt-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.55fr);
  gap: 16px;
}

.form-field {
  display: grid;
  align-content: start;
  gap: 8px;
  min-width: 0;
}

.wide-field,
.full-width {
  grid-column: 1 / -1;
}

.form-field input,
.form-field textarea,
.form-field select {
  box-sizing: border-box;
  width: 100%;
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #0D1117;
  color: #E6EDF3;
  font: inherit;
  font-size: 14px;
  letter-spacing: 0;
  outline: 0;
}

.form-field input {
  height: 40px;
  padding: 0 12px;
}

.form-field select {
  height: 40px;
  padding: 0 12px;
}

.form-field textarea {
  min-height: 92px;
  padding: 10px 12px;
  line-height: 1.55;
  resize: vertical;
}

.form-field input:focus,
.form-field textarea:focus,
.form-field select:focus,
.number-control:focus-within {
  border-color: #2F81F7;
  box-shadow: 0 0 0 3px rgba(47, 129, 247, 0.18);
}

.input-with-action {
  position: relative;
}

.input-with-action input {
  padding-right: 48px;
}

.field-icon-button {
  position: absolute;
  top: 2px;
  right: 2px;
}

.field-hint {
  color: #8B949E;
  font-size: 12px;
}

.inline-action-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 18px;
}

.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 40px;
  gap: 8px;
  padding: 9px 15px;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.button.primary {
  border-color: #1F6FEB;
  background: #2F81F7;
  color: #ffffff;
}

.button.secondary {
  border-color: #30363D;
  background: #21262D;
  color: #C9D1D9;
}

.button:hover:not(:disabled) {
  filter: brightness(0.97);
}

.button:disabled {
  cursor: not-allowed;
  opacity: 0.58;
}

.button-spinner,
.spinner {
  width: 15px;
  height: 15px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

.inline-result,
.action-result {
  color: #3FB950;
  font-size: 13px;
  line-height: 1.55;
  white-space: pre-wrap;
}

.inline-result.error,
.action-result.error {
  color: #F85149;
}

.channel-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  overflow: hidden;
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #0D1117;
}

.channel-tab {
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 58px;
  gap: 3px;
  padding: 9px 8px;
  border: 0;
  border-left: 1px solid #30363D;
  background: transparent;
  color: #C9D1D9;
  font: inherit;
  cursor: pointer;
}

.channel-tab:first-child {
  border-left: 0;
}

.channel-tab span {
  font-size: 14px;
  font-weight: 600;
}

.channel-tab small {
  color: #8B949E;
  font-size: 11px;
}

.channel-tab small.enabled {
  color: #3FB950;
}

.channel-tab.active {
  background: #161B22;
  box-shadow: inset 0 -3px #2F81F7;
  color: #58A6FF;
}

.channel-panel {
  padding-top: 22px;
}

.channel-title-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  min-height: 44px;
  gap: 20px;
}

.channel-option-row {
  margin-top: 18px;
  border: 1px solid #30363D;
  border-radius: 8px;
  background: #21262D;
}

.channel-form-grid {
  margin-top: 20px;
}

.channel-ai-section {
  margin-top: 22px;
  padding-top: 20px;
  border-top: 1px solid #30363D;
}

.channel-ai-section > .setting-row {
  min-height: auto;
  padding: 0;
}

.ai-prompt-grid {
  margin-top: 18px;
}

.variable-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 7px;
}

.variable-row span {
  margin-right: 2px;
  color: #8B949E;
  font-size: 12px;
}

.variable-row button {
  min-height: 28px;
  padding: 4px 8px;
  border: 1px solid #30363D;
  border-radius: 6px;
  background: #21262D;
  color: #58A6FF;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.channel-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 22px;
  padding-top: 20px;
  border-top: 1px solid #30363D;
}

.action-result {
  margin-top: 14px;
  padding: 12px 14px;
  border-left: 3px solid currentColor;
  background: #21262D;
}

.history-surface {
  margin-top: 18px;
  padding: 24px;
}

.history-heading {
  margin-bottom: 14px;
}

.count-badge {
  display: inline-grid;
  place-items: center;
  min-width: 28px;
  height: 28px;
  padding: 0 8px;
  border-radius: 999px;
  background: #21262D;
  color: #C9D1D9;
  font-size: 12px;
  font-weight: 600;
}

.empty-history {
  display: grid;
  place-items: center;
  min-height: 150px;
  gap: 8px;
  color: #8B949E;
  font-size: 13px;
}

.history-list {
  display: grid;
}

.history-item {
  padding: 14px 0;
  border-top: 1px solid #30363D;
}

.history-item:first-child {
  border-top: 0;
}

.history-meta {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
}

.history-meta strong {
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: 13px;
}

.history-meta time {
  flex: 0 0 auto;
  color: #8B949E;
  font-size: 11px;
}

.history-item p {
  margin: 7px 0 0;
  color: #C9D1D9;
  font-size: 13px;
  line-height: 1.55;
  overflow-wrap: anywhere;
}

.access-panel,
.loading-panel,
.error-panel {
  display: grid;
  justify-items: center;
  min-height: 260px;
  align-content: center;
  gap: 12px;
  padding: 32px;
  text-align: center;
}

.access-panel svg {
  color: #D29922;
}

.loading-panel,
.error-panel {
  color: #8B949E;
  font-size: 14px;
}

.error-panel strong {
  color: #F85149;
}

.spinner {
  width: 24px;
  height: 24px;
  color: #2F81F7;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 700px) {
  .header-content,
  .page-main {
    width: min(100% - 24px, 960px);
  }

  .page-main {
    padding-top: 16px;
  }

  .surface-section,
  .history-surface {
    padding: 18px;
  }

  .provider-grid,
  .channel-form-grid,
  .ai-prompt-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .wide-field,
  .full-width {
    grid-column: auto;
  }

  .setting-row {
    gap: 16px;
  }

  .preset-group {
    display: grid;
    width: 100%;
  }

  .channel-actions .button {
    flex: 1 1 220px;
  }
}

@media (max-width: 430px) {
  .header-content {
    grid-template-columns: 36px 1fr 58px;
  }

  .header-content h1 {
    font-size: 18px;
  }

  .surface-section,
  .history-surface {
    padding: 16px;
  }

  .section-heading {
    margin-bottom: 16px;
  }

  .setting-row {
    align-items: flex-start;
  }

  .compact-field {
    align-items: flex-start;
    flex-direction: column;
  }

  .number-control {
    width: 100%;
  }

  .number-control input {
    flex: 1;
    width: auto;
  }

  .channel-tab {
    min-height: 62px;
    padding-inline: 4px;
  }

  .channel-tab span {
    font-size: 13px;
  }

  .segmented-control {
    display: grid;
    grid-auto-columns: minmax(0, 1fr);
    width: 100%;
  }

  .history-meta {
    align-items: flex-start;
    flex-direction: column;
    gap: 4px;
  }
}
</style>
