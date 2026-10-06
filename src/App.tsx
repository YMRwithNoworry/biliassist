import { Loader2, TriangleAlert } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { HashRouter, Navigate, Outlet, Route, Routes } from 'react-router-dom'
import Sidebar from '@/components/Sidebar'
import { Button } from '@/components/ui/button'
import { Toaster } from '@/components/ui/sonner'
import { errorMessage } from '@/lib/ipc'
import { AuthProvider, useAuth } from '@/state/auth'
import AccountsView from '@/views/AccountsView'
import AuthPage from '@/views/AuthPage'
import AutoReplyView from '@/views/AutoReplyView'
import DashboardView from '@/views/DashboardView'
import LoginView from '@/views/LoginView'
import SponsorView from '@/views/SponsorView'

const MAX_ATTEMPTS = 3
const RESTORE_TIMEOUT = 10_000

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/** 会话恢复依赖网络，超时兜底避免启动界面永久挂起。 */
function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('会话恢复超时')), ms)
    promise.then(
      (value) => {
        clearTimeout(timer)
        resolve(value)
      },
      (error: unknown) => {
        clearTimeout(timer)
        reject(error instanceof Error ? error : new Error(String(error)))
      },
    )
  })
}

function BootLoading() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3">
      <Loader2 className="size-8 animate-spin text-muted-foreground" />
      <p className="text-sm text-muted-foreground">加载中...</p>
    </div>
  )
}

function BootError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 px-6 text-center">
      <span className="flex size-14 items-center justify-center rounded-full bg-destructive/10 text-destructive">
        <TriangleAlert className="size-7" />
      </span>
      <div className="space-y-1">
        <h1 className="text-lg font-semibold">启动失败</h1>
        <p className="max-w-md text-sm text-muted-foreground">{message}</p>
      </div>
      <Button onClick={onRetry}>重试</Button>
    </div>
  )
}

/** 已登录用户的框架：侧边栏导航 + 路由出口。 */
function AuthenticatedShell() {
  const { isAuthenticated, loading } = useAuth()

  if (loading) return <BootLoading />
  if (!isAuthenticated) return <Navigate to="/auth" replace />

  return (
    <div className="flex h-full">
      <Sidebar />
      <main className="flex min-w-0 flex-1 flex-col bg-muted">
        <Outlet />
      </main>
    </div>
  )
}

function AuthRoute() {
  const { isAuthenticated, loading } = useAuth()

  if (loading) return <BootLoading />
  if (isAuthenticated) return <Navigate to="/" replace />
  return <AuthPage />
}

function AppRoutes() {
  const { refresh } = useAuth()
  const [booting, setBooting] = useState(true)
  const [bootError, setBootError] = useState('')

  const boot = useCallback(async () => {
    setBooting(true)
    setBootError('')
    for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
      try {
        await withTimeout(refresh(), RESTORE_TIMEOUT)
        setBooting(false)
        return
      } catch (error) {
        if (attempt < MAX_ATTEMPTS) {
          await delay(1000 * attempt)
          continue
        }
        setBootError(`应用初始化失败：${errorMessage(error)}，请检查网络连接后重试`)
      }
    }
    setBooting(false)
  }, [refresh])

  useEffect(() => {
    void boot()
  }, [boot])

  if (booting) return <BootLoading />
  if (bootError) return <BootError message={bootError} onRetry={() => void boot()} />

  return (
    <Routes>
      <Route path="/auth" element={<AuthRoute />} />
      <Route element={<AuthenticatedShell />}>
        <Route path="/" element={<DashboardView />} />
        <Route path="/login" element={<LoginView />} />
        <Route path="/accounts" element={<AccountsView />} />
        <Route path="/auto-reply" element={<AutoReplyView />} />
        <Route path="/sponsor" element={<SponsorView />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}

export default function App() {
  return (
    <AuthProvider>
      <HashRouter>
        <AppRoutes />
        <Toaster position="top-center" />
      </HashRouter>
    </AuthProvider>
  )
}
