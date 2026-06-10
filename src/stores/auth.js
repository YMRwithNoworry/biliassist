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

function firstString(...values) {
  for (const value of values) {
    if (typeof value === 'string' && value.trim()) {
      return value.trim()
    }
  }
  return ''
}

function getOAuthUrl(data) {
  return firstString(
    data?.verification_uri_complete,
    data?.verificationUriComplete,
    data?.verification_url_complete,
    data?.verificationUrlComplete,
    data?.url,
    data?.verification_uri,
    data?.verificationUri,
    data?.verification_url,
    data?.verificationUrl
  )
}

function getUrlUserCode(url) {
  try {
    const params = new URL(url).searchParams
    return firstString(
      params.get('user_code'),
      params.get('userCode'),
      params.get('device_user_code'),
      params.get('deviceUserCode')
    )
  } catch {
    return ''
  }
}

function getOAuthUserCode(data, url) {
  return firstString(
    data?.user_code,
    data?.userCode,
    data?.device_user_code,
    data?.deviceUserCode,
    data?.device?.user_code,
    data?.device?.userCode,
    getUrlUserCode(url)
  )
}

function getCallbackParam(callbackUrl, key) {
  const url = new URL(callbackUrl)
  const searchValue = url.searchParams.get(key)
  if (searchValue) return searchValue

  const hash = url.hash.startsWith('#') ? url.hash.slice(1) : url.hash
  return new URLSearchParams(hash).get(key)
}

function normalizeLinkIdentityError(error) {
  const message = error?.message || String(error || '')
  const lowerMessage = message.toLowerCase()

  if (lowerMessage.includes('manual') && lowerMessage.includes('link')) {
    return new Error('GitHub 绑定失败：请在 Supabase Auth 设置中开启 Manual Linking（手动身份绑定）')
  }
  if (lowerMessage.includes('already') && lowerMessage.includes('linked')) {
    return new Error('这个 GitHub 账号已绑定到其他账号，请更换 GitHub 账号或先解绑')
  }
  return error instanceof Error ? error : new Error(message || 'GitHub 绑定失败')
}

function toProviderList(value) {
  if (Array.isArray(value)) return value
  if (typeof value === 'string' && value) return [value]
  return []
}

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
  const appMetadataProviders = computed(() => toProviderList(user.value?.app_metadata?.providers))
  const providers = computed(() => Array.from(new Set([
    ...identities.value.map(identity => identity.provider).filter(Boolean),
    ...appMetadataProviders.value.filter(Boolean)
  ])))
  const hasGitHubIdentity = computed(() => providers.value.includes('github'))
  const hasEmailIdentity = computed(() => providers.value.includes('email') || !!user.value?.email)

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
    const url = getOAuthUrl(data)
    if (!url) throw new Error('未获取到 GitHub 授权链接')
    const userCode = getOAuthUserCode(data, url)
    let codeCopied = false

    if (userCode) {
      try {
        await invoke('copy_text_to_clipboard', { text: userCode })
        codeCopied = true
      } catch (tauriError) {
        try {
          await navigator.clipboard.writeText(userCode)
          codeCopied = true
        } catch (browserError) {
          console.warn('复制 GitHub 验证码失败:', tauriError, browserError)
        }
      }
    }

    await invoke('open_external_url', { url })
    return { userCode, codeCopied }
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
      if (error) throw normalizeLinkIdentityError(error)
      const result = await openOAuthUrl(data)
      githubLoading.value = false
      return result
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
      const result = await openOAuthUrl(data)
      githubLoading.value = false
      return result
      // Binding continues in the system browser; callback arrives via deep-link.
    } catch (e) {
      githubLoading.value = false
      throw e
    }
  }

  /** 处理 OAuth 回调 URL（由 App.vue deep-link 监听器调用） */
  const handleOAuthCallback = async (callbackUrl) => {
    try {
      const callbackError = getCallbackParam(callbackUrl, 'error_description') || getCallbackParam(callbackUrl, 'error')
      if (callbackError) throw new Error(callbackError)

      const code = getCallbackParam(callbackUrl, 'code')
      const accessToken = getCallbackParam(callbackUrl, 'access_token')
      const refreshToken = getCallbackParam(callbackUrl, 'refresh_token')
      if (!code && (!accessToken || !refreshToken)) {
        throw new Error('GitHub 回调缺少授权码')
      }

      const { data, error } = code
        ? await requireSupabase().auth.exchangeCodeForSession(code)
        : await requireSupabase().auth.setSession({
            access_token: accessToken,
            refresh_token: refreshToken
          })
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
    providers,
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
