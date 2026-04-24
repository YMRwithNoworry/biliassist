import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { supabase } from '../lib/supabase'

export const useAuthStore = defineStore('auth', () => {
  const user = ref(null)
  const session = ref(null)
  const loading = ref(true)

  const isAuthenticated = computed(() => !!session.value && !!user.value)

  const agetSession = async () => {
    try {
      const { data: { session: currentSession } } = await supabase.auth.getSession()
      session.value = currentSession
      user.value = currentSession?.user ?? null
    } catch (error) {
      console.error('获取会话失败:', error)
    } finally {
      loading.value = false
    }
  }

  const signInWithOtp = async (email) => {
    const { error } = await supabase.auth.signInWithOtp({
      email,
      options: { shouldCreateUser: true }
    })
    if (error) throw error
  }

  const verifyOtp = async (email, token) => {
    const { data, error } = await supabase.auth.verifyOtp({
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
    const { data, error } = await supabase.auth.signUp({
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
    const { data, error } = await supabase.auth.signInWithPassword({
      email,
      password
    })
    if (error) throw error
    session.value = data.session
    user.value = data.user
    return data
  }

  const signOut = async () => {
    const { error } = await supabase.auth.signOut()
    if (error) throw error
    session.value = null
    user.value = null
  }

  const setPassword = async (newPassword) => {
    const { error } = await supabase.auth.updateUser({
      password: newPassword
    })
    if (error) throw error
  }

  return {
    user,
    session,
    loading,
    isAuthenticated,
    getSession: agetSession,
    signInWithOtp,
    verifyOtp,
    signUpWithPassword,
    signInWithPassword,
    signOut,
    setPassword
  }
})
