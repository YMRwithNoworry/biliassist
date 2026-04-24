import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { supabase } from '../lib/supabase'

function noop() {
  return { data: null, error: new Error('Supabase 未配置') }
}

function noopError() {
  throw new Error('Supabase 未配置')
}

const safeSupabase = supabase || {
  auth: {
    getSession: noop,
    signInWithOtp: noop,
    verifyOtp: noop,
    signUp: noop,
    signInWithPassword: noop,
    signOut: () => Promise.resolve({ error: null }),
    updateUser: noop,
    onAuthStateChange: () => ({ data: { subscription: { unsubscribe: () => {} } } })
  }
}

export const useAuthStore = defineStore('auth', () => {
  const user = ref(null)
  const session = ref(null)
  const loading = ref(true)

  const isAuthenticated = computed(() => !!session.value && !!user.value)

  const getSession = async () => {
    try {
      if (!supabase) {
        loading.value = false
        return
      }
      const { data: { session: currentSession } } = await safeSupabase.auth.getSession()
      session.value = currentSession
      user.value = currentSession?.user ?? null
    } catch (error) {
      console.error('获取会话失败:', error)
    } finally {
      loading.value = false
    }
  }

  const signInWithOtp = async (email) => {
    if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
    const { error } = await safeSupabase.auth.signInWithOtp({
      email,
      options: { shouldCreateUser: true }
    })
    if (error) throw error
  }

  const verifyOtp = async (email, token) => {
    if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
    const { data, error } = await safeSupabase.auth.verifyOtp({
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
    if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
    const { data, error } = await safeSupabase.auth.signUp({
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
    if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
    const { data, error } = await safeSupabase.auth.signInWithPassword({
      email,
      password
    })
    if (error) throw error
    session.value = data.session
    user.value = data.user
    return data
  }

  const signOut = async () => {
    if (supabase) {
      const { error } = await safeSupabase.auth.signOut()
      if (error) throw error
    }
    session.value = null
    user.value = null
  }

  const setPassword = async (newPassword) => {
    if (!supabase) throw new Error('Supabase 未配置，请检查 .env 文件')
    const { error } = await safeSupabase.auth.updateUser({
      password: newPassword
    })
    if (error) throw error
  }

  return {
    user,
    session,
    loading,
    isAuthenticated,
    getSession,
    signInWithOtp,
    verifyOtp,
    signUpWithPassword,
    signInWithPassword,
    signOut,
    setPassword
  }
})
