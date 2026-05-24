import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { supabase } from '../lib/supabase'

function requireSupabase() {
  if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
  return supabase
}

const LOCAL_LICENSE_KEY = 'biliassist_license_activated'

export const useAuthStore = defineStore('auth', () => {
  const user = ref(null)
  const session = ref(null)
  const loading = ref(true)
  const userTier = ref('basic')
  const tierChecked = ref(false)

  const isAuthenticated = computed(() => !!session.value && !!user.value)
  const isPlus = computed(() => userTier.value === 'plus')

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
      user.value = currentSession?.user ?? null
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

  const signOut = async () => {
    const { error } = await requireSupabase().auth.signOut()
    if (error) throw error
    session.value = null
    user.value = null
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
    isLocallyActivated,
    saveLocalActivation,
    getSession: agetSession,
    checkTier,
    signInWithOtp,
    verifyOtp,
    signUpWithPassword,
    signInWithPassword,
    signOut,
    setPassword
  }
})