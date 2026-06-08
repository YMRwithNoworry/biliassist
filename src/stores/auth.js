import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { supabase } from '../lib/supabase'

function requireSupabase() {
  if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
  return supabase
}

const LOCAL_LICENSE_KEY = 'biliassist_license_activated'
const OAUTH_REDIRECT_URL = 'biliassist://auth/callback'

export const useAuthStore = defineStore('auth', () => {
  const user = ref(null)
  const session = ref(null)
  const loading = ref(true)
  const userTier = ref('basic')
  const tierChecked = ref(false)
  const githubLoading = ref(false)

  const isAuthenticated = computed(() => !!session.value && !!user.value)
  const isPlus = computed(() => userTier.value === 'plus')
  const identities = computed(() => user.value?.identities || [])
  const hasGitHubIdentity = computed(() => identities.value.some(identity => identity.provider === 'github'))
  const hasEmailIdentity = computed(() => identities.value.some(identity => identity.provider === 'email'))

  /** 检查本地是否已激活 */
  function isLocallyActivated() {
    try {
      return localStorage.getItem(LOCAL_LICENSE_KEY) === 'true'
    } catch {
      return false
    }
  }

  /** 将激活状态保存到本地 */
  function saveLocalActivation() {
    try {
      localStorage.setItem(LOCAL_LICENSE_KEY, 'true')
    } catch (e) {
      console.warn('保存本地激活状态失败:', e)
    }
  }

  const agetSession = async () => {
    try {
      const { data: { session: currentSession } } = await requireSupabase().auth.getSession()
      session.value = currentSession
      if (currentSession) {
        const { data: { user: currentUser } } = await requireSupabase().auth.getUser()
        user.value = currentUser ?? currentSession.user
      } else {
        user.value = null
      }
      if (user.value) {
        await checkTier()
      } else if (isLocallyActivated()) {
        userTier.value = 'plus'
        tierChecked.value = true
      }
    } catch (error) {
      console.error('获取会话失败:', error)
      if (isLocallyActivated()) {
        userTier.value = 'plus'
        tierChecked.value = true
      }
    } finally {
      loading.value = false
    }
  }

  async function checkTier() {
    // 本地已激活则直接设为 plus
    if (isLocallyActivated()) {
      userTier.value = 'plus'
      tierChecked.value = true
      return
    }
    if (!supabase || !user.value?.id) {
      userTier.value = 'basic'
      tierChecked.value = true
      return
    }
    try {
      const { data, error } = await supabase
        .from('user_tiers')
        .select('tier')
        .eq('user_id', user.value.id)
        .maybeSingle()
      if (error) {
        console.warn('检查用户等级失败:', error)
      } else if (data?.tier === 'plus') {
        userTier.value = 'plus'
      } else {
        userTier.value = 'basic'
      }
    } catch (e) {
      console.warn('检查用户等级异常:', e)
    } finally {
      tierChecked.value = true
    }
  }

  const signInWithOtp = async (email) => {
    const { error } = await requireSupabase().auth.signInWithOtp({
      email,
      options: { shouldCreateUser: true }
    })
    if (error) throw error
  }

  const verifyOtp = async (email, token) => {
    const { data, error } = await requireSupabase().auth.verifyOtp({
      email,
      token,
      type: 'email'
    })
    if (error) throw error
    session.value = data.session
    user.value = data.user
    return data
  }

  const signUpWithPassword = async (email, password) => {
    const { data, error } = await requireSupabase().auth.signUp({
      email,
      password
    })
    if (error) throw error
    if (data.session) {
      session.value = data.session
      user.value = data.user
    }
    return data
  }

  const signInWithPassword = async (email, password) => {
    const { data, error } = await requireSupabase().auth.signInWithPassword({
      email,
      password
    })
    if (error) throw error
    session.value = data.session
    user.value = data.user
    return data
  }

  async function openOAuthUrl(data) {
    const url = data?.url
    if (!url) throw new Error('未获取到 GitHub 授权链接')
    await invoke('open_external_url', { url })
  }

  const signInWithGitHub = async () => {
    githubLoading.value = true
    try {
      const { data, error } = await requireSupabase().auth.signInWithOAuth({
        provider: 'github',
        options: {
          redirectTo: OAUTH_REDIRECT_URL,
          skipBrowserRedirect: true,
          scopes: 'read:user user:email'
        }
      })
      if (error) throw error
      await openOAuthUrl(data)
      githubLoading.value = false
      // OAuth URL is opened in the system browser; callback arrives via deep-link.
    } catch (e) {
      githubLoading.value = false
      throw e
    }
  }

  const linkGitHubIdentity = async () => {
    if (!isAuthenticated.value) {
      throw new Error('请先使用邮箱登录后再绑定 GitHub')
    }
    githubLoading.value = true
    try {
      const { data, error } = await requireSupabase().auth.linkIdentity({
        provider: 'github',
        options: {
          redirectTo: OAUTH_REDIRECT_URL,
          skipBrowserRedirect: true,
          scopes: 'read:user user:email'
        }
      })
      if (error) throw error
      await openOAuthUrl(data)
      githubLoading.value = false
      // Binding continues in the system browser; callback arrives via deep-link.
    } catch (e) {
      githubLoading.value = false
      throw e
    }
  }

  /** 处理 OAuth 回调 URL（由 App.vue deep-link 监听器调用） */
  const handleOAuthCallback = async (callbackUrl) => {
    try {
      // Supabase JS v2: PKCE code exchange
      const { data, error } = await requireSupabase().auth.exchangeCodeForSession(
        new URL(callbackUrl).searchParams.get('code')
      )
      if (error) throw error
      session.value = data.session
      const { data: { user: currentUser } } = await requireSupabase().auth.getUser()
      user.value = currentUser ?? data.user
      await checkTier()
      return true
    } catch (e) {
      console.error('OAuth 回调处理失败:', e)
      // Fallback: try getSession in case Supabase already stored the session
      try {
        const { data: { session: currentSession } } = await requireSupabase().auth.getSession()
        if (currentSession) {
          session.value = currentSession
          const { data: { user: currentUser } } = await requireSupabase().auth.getUser()
          user.value = currentUser ?? currentSession.user
          await checkTier()
          return true
        }
      } catch {}
      return false
    } finally {
      githubLoading.value = false
    }
  }

  const signOut = async () => {
    const { error } = await requireSupabase().auth.signOut()
    if (error) throw error
    session.value = null
    user.value = null
  }

  const setEmailPassword = async (email, password) => {
    const update = { password }
    if (email && email !== user.value?.email) {
      update.email = email
    }
    const { data, error } = await requireSupabase().auth.updateUser(update)
    if (error) throw error
    if (data.user) {
      user.value = data.user
    }
    return data
  }

  const setPassword = async (newPassword) => {
    const { error } = await requireSupabase().auth.updateUser({
      password: newPassword
    })
    if (error) throw error
  }

  return {
    user,
    session,
    loading,
    userTier,
    tierChecked,
    isAuthenticated,
    isPlus,
    identities,
    hasGitHubIdentity,
    hasEmailIdentity,
    isLocallyActivated,
    saveLocalActivation,
    getSession: agetSession,
    checkTier,
    signInWithOtp,
    verifyOtp,
    signUpWithPassword,
    signInWithPassword,
    signInWithGitHub,
    linkGitHubIdentity,
    handleOAuthCallback,
    githubLoading,
    signOut,
    setEmailPassword,
    setPassword
  }
})