import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { api, type AuthSession } from '../lib/ipc'

export interface AuthContextValue {
  /** 后端返回的 Supabase 会话，未登录为 null。 */
  session: AuthSession | null
  isAuthenticated: boolean
  /** 后端 tier 为 plus 或本机已激活授权码。 */
  isPlus: boolean
  loading: boolean
  email: string
  signIn: (email: string, password: string) => Promise<void>
  sendOtp: (email: string) => Promise<void>
  verifyOtp: (email: string, token: string) => Promise<void>
  logout: () => Promise<void>
  refresh: () => Promise<void>
  activateLicense: (key: string) => Promise<void>
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<AuthSession | null>(null)
  const [licensed, setLicensed] = useState(false)
  const [loading, setLoading] = useState(true)

  const refresh = useCallback(async () => {
    setLoading(true)
    try {
      const restored = await api.restoreSession()
      setSession(restored)
      // 授权查询失败不应中断启动流程，按未激活处理即可。
      try {
        setLicensed(await api.isLicensed())
      } catch {
        setLicensed(false)
      }
    } finally {
      setLoading(false)
    }
  }, [])

  const signIn = useCallback(async (email: string, password: string) => {
    setSession(await api.signIn(email, password))
  }, [])

  const sendOtp = useCallback(async (email: string) => {
    await api.sendOtp(email)
  }, [])

  const verifyOtp = useCallback(async (email: string, token: string) => {
    setSession(await api.verifyOtp(email, token))
  }, [])

  const logout = useCallback(async () => {
    await api.logout()
    setSession(null)
  }, [])

  const activateLicense = useCallback(async (key: string) => {
    await api.activateLicense(key)
    setLicensed(await api.isLicensed())
  }, [])

  const value = useMemo<AuthContextValue>(
    () => ({
      session,
      isAuthenticated: session !== null,
      isPlus: licensed || session?.tier === 'plus',
      loading,
      email: session?.email ?? '',
      signIn,
      sendOtp,
      verifyOtp,
      logout,
      refresh,
      activateLicense,
    }),
    [session, licensed, loading, signIn, sendOtp, verifyOtp, logout, refresh, activateLicense],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth 必须在 AuthProvider 内使用')
  }
  return context
}
