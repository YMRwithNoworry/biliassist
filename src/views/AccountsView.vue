<template>
  <div class="page">
    <div class="topbar">
      <button class="topbar-back" @click="goBack">←</button>
      <span class="topbar-title">账号管理</span>
    </div>

    <div class="page-body slide-up">
      <div v-if="loading" style="text-align:center;padding:48px;color:var(--text-secondary)">
        加载中...
      </div>

      <div v-else-if="accounts.length === 0" class="empty-state">
        <p>暂无账号，请先扫码登录</p>
        <button class="btn btn-primary" @click="goToLogin">扫码登录</button>
      </div>

      <div v-else class="account-list">
        <div
          v-for="account in accounts"
          :key="account.uid"
          class="account-row card"
          :class="{ active: account.active }"
        >
          <div class="account-avatar">
            <img v-if="account.avatar" :src="account.avatar" class="avatar-img" />
            <span v-else>{{ account.name.charAt(0) }}</span>
          </div>
          <div class="account-info">
            <h3 class="account-name">{{ account.name }}</h3>
            <p class="account-uid">UID: {{ account.uid }}</p>
          </div>
          <div v-if="account.active" class="account-badge">当前</div>
          <div class="account-actions">
            <button
              v-if="!account.active"
              class="btn btn-ghost btn-sm"
              @click="activateAccount(account.uid)"
            >
              切换
            </button>
            <button class="btn btn-danger btn-sm" @click="deleteAccount(account.uid)">
              删除
            </button>
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
.account-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
}

.account-row {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-md) var(--sp-lg);
}

.account-row.active {
  border-left: 3px solid var(--rausch-red);
}

.account-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--surface-light);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  flex-shrink: 0;
  overflow: hidden;
}

.avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 50%;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.account-uid {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.account-badge {
  font-size: 12px;
  font-weight: 600;
  color: var(--rausch-red);
  background: #fff0f3;
  padding: 2px 10px;
  border-radius: var(--radius-xl);
  flex-shrink: 0;
}

.account-actions {
  display: flex;
  gap: var(--sp-sm);
  flex-shrink: 0;
}
</style>
