<template>
  <div class="page">
    <div class="topbar">
      <button class="topbar-back" @click="goBack">←</button>
      <span class="topbar-title">自动回复</span>
    </div>

    <div class="page-body slide-up">
      <!-- 开关 -->
      <div class="toggle-row">
        <div>
          <div class="toggle-label">启用自动回复</div>
          <div class="toggle-desc">开启后将自动回复粉丝消息</div>
        </div>
        <label class="toggle">
          <input type="checkbox" v-model="enabled" @change="save" />
          <span class="toggle-track"></span>
        </label>
      </div>

      <!-- 消息来源 -->
      <div class="section" style="margin-top:24px">
        <div class="section-title">回复来源</div>
        <div class="chips">
          <span
            class="chip"
            :class="{ active: sources.includes('comment') }"
            @click="toggleSource('comment')"
          >视频评论</span>
          <span
            class="chip"
            :class="{ active: sources.includes('directMessage') }"
            @click="toggleSource('directMessage')"
          >私信</span>
          <span
            class="chip"
            :class="{ active: sources.includes('follow') }"
            @click="toggleSource('follow')"
          >粉丝关注</span>
        </div>
      </div>

      <!-- 回复内容 -->
      <div class="section" style="margin-top:24px">
        <div class="section-title">回复内容</div>
        <div class="field">
          <textarea
            v-model="message"
            class="textarea"
            placeholder="输入自动回复内容..."
            @blur="save"
          ></textarea>
          <div class="hint">支持变量：{用户名}、{时间}</div>
        </div>
      </div>

      <!-- 间隔 & 只回复一次 -->
      <div class="section" style="margin-top:24px">
        <div class="toggle-row">
          <div>
            <div class="toggle-label">每个用户只回复一次</div>
            <div class="toggle-desc">避免重复发送消息</div>
          </div>
          <label class="toggle">
            <input type="checkbox" v-model="replyOnlyOnce" @change="save" />
            <span class="toggle-track"></span>
          </label>
        </div>

        <div class="field" style="margin-top:16px">
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

      <!-- 测试 -->
      <div style="margin-top:24px">
        <button class="btn btn-ghost btn-block" @click="testReply">测试回复</button>
        <button class="btn btn-primary btn-block" style="margin-top:12px" @click="manualReply">立即回复视频评论</button>
      </div>

      <!-- 历史记录 -->
      <div class="section" style="margin-top:32px">
        <div class="section-title">回复记录</div>
        <div v-if="history.length === 0" class="empty-hint">暂无记录</div>
        <div v-else class="history-list">
          <div v-for="(r, i) in history" :key="i" class="history-item">
            <div class="history-head">
              <span class="history-user">{{ r.user }}</span>
              <span class="history-source chip active" style="padding:2px 8px;font-size:11px;border-radius:8px">
                {{ sourceLabel(r.source) }}
              </span>
            </div>
            <p class="history-msg">{{ r.message }}</p>
            <p class="history-time">{{ r.time }}</p>
          </div>
        </div>
      </div>
    </div>
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

const goBack = () => router.push('/')

onMounted(() => load())
</script>

<style scoped>
.section {
  /* just a grouping wrapper */
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-sm);
}

.hint {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: var(--sp-xs);
}

.empty-hint {
  font-size: 14px;
  color: var(--text-secondary);
  padding: var(--sp-lg) 0;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.history-item {
  padding: var(--sp-md);
  background: var(--surface-light);
  border-radius: var(--radius-md);
}

.history-head {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  margin-bottom: var(--sp-xs);
}

.history-user {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.history-msg {
  font-size: 14px;
  color: var(--text-primary);
  line-height: 1.4;
}

.history-time {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
}
</style>
