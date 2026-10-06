import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  Check,
  CircleCheck,
  Loader2,
  QrCode,
  RefreshCw,
  TriangleAlert,
  Users,
} from 'lucide-react'
import { toast } from 'sonner'
import EmptyState from '@/components/EmptyState'
import SectionCard from '@/components/SectionCard'
import StatusBar from '@/components/StatusBar'
import ViewShell from '@/components/ViewShell'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { api, errorMessage, type UserInfo } from '@/lib/ipc'
import { cn } from '@/lib/utils'

const POLL_INTERVAL = 2000
const QR_TIMEOUT = 180_000

type StatusKind = 'pending' | 'warning' | 'success' | 'error'

const STATUS_STYLE: Record<
  StatusKind,
  { className: string; Icon: typeof Check }
> = {
  pending: { className: 'bg-info-tint text-info', Icon: Loader2 },
  warning: { className: 'bg-warning-tint text-warning', Icon: Check },
  success: { className: 'bg-success-tint text-success', Icon: CircleCheck },
  error: { className: 'bg-destructive/10 text-destructive', Icon: TriangleAlert },
}

export default function LoginView() {
  const navigate = useNavigate()

  const [qrDataUrl, setQrDataUrl] = useState('')
  const [status, setStatus] = useState<{ kind: StatusKind; text: string } | null>(null)
  const [loading, setLoading] = useState(false)
  const [polling, setPolling] = useState(false)
  const [loggedIn, setLoggedIn] = useState(false)
  const [userInfo, setUserInfo] = useState<UserInfo | null>(null)
  const [error, setError] = useState('')

  const startedAtRef = useRef(0)

  const poll = useCallback(async () => {
    try {
      const result = await api.checkLoginStatus()
      if (result.status === 'success') {
        setPolling(false)
        setLoggedIn(true)
        setUserInfo(result.userInfo)
        setStatus({ kind: 'success', text: '登录成功！' })
        toast.success('登录成功，账号已保存到本地')
      } else if (result.status === 'expired') {
        setPolling(false)
        setQrDataUrl('')
        setStatus({ kind: 'error', text: '二维码已过期，请重新获取' })
      } else if (result.status === 'scanned') {
        setStatus({ kind: 'warning', text: '已扫码，请在手机上确认' })
      } else if (Date.now() - startedAtRef.current > QR_TIMEOUT) {
        // B站二维码有效期约 3 分钟，超时后主动停轮询，避免无意义请求。
        setPolling(false)
        setQrDataUrl('')
        setStatus({ kind: 'error', text: '二维码已超时，请重新获取' })
      }
    } catch (e) {
      setPolling(false)
      setError(errorMessage(e))
    }
  }, [])

  useEffect(() => {
    if (!polling) return
    const timer = window.setInterval(() => void poll(), POLL_INTERVAL)
    return () => window.clearInterval(timer)
  }, [polling, poll])

  const getQrCode = async () => {
    setLoading(true)
    setError('')
    setLoggedIn(false)
    setUserInfo(null)
    setPolling(false)
    try {
      const result = await api.getQrCode()
      // 后端可能直接返回 PNG base64，也可能返回待编码的登录链接。
      const png = result.qrcode.startsWith('http')
        ? await api.generateQrCode(result.qrcode)
        : result.qrcode
      setQrDataUrl(`data:image/png;base64,${png}`)
      setStatus({ kind: 'pending', text: '等待扫码...' })
      startedAtRef.current = Date.now()
      setPolling(true)
    } catch (e) {
      setError(errorMessage(e))
      setStatus(null)
    } finally {
      setLoading(false)
    }
  }

  const statusStyle = status ? STATUS_STYLE[status.kind] : null

  return (
    <ViewShell
      title="扫码登录"
      subtitle="使用 B站 App 扫码即可添加账号，无需输入密码"
      maxWidth="max-w-2xl"
    >
      {error ? <StatusBar tone="error">{error}</StatusBar> : null}

      {loggedIn ? (
        <SectionCard title="登录成功" description="账号已保存到本地">
          <div className="flex flex-col items-center gap-5 text-center">
            <span className="flex size-14 items-center justify-center rounded-full bg-success-tint text-success">
              <CircleCheck className="size-7" strokeWidth={2.5} />
            </span>
            {userInfo ? (
              <div className="flex items-center gap-3 rounded-lg border bg-muted/50 px-4 py-3 text-left">
                <span className="flex size-10 items-center justify-center rounded-full bg-brand text-sm font-semibold text-brand-foreground">
                  {userInfo.name.charAt(0).toUpperCase()}
                </span>
                <div className="min-w-0">
                  <div className="truncate font-medium">{userInfo.name}</div>
                  <div className="text-xs text-muted-foreground">UID: {userInfo.uid}</div>
                </div>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">账号已保存到本地。</p>
            )}
            <div className="flex flex-wrap items-center justify-center gap-3">
              <Button variant="outline" onClick={() => void getQrCode()}>
                <QrCode />
                继续添加账号
              </Button>
              <Button variant="brand" onClick={() => navigate('/accounts')}>
                <Users />
                查看账号
              </Button>
            </div>
          </div>
        </SectionCard>
      ) : qrDataUrl ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-4 text-center">
            <div className="rounded-xl border bg-card p-4">
              <img className="size-56" src={qrDataUrl} alt="B站登录二维码" />
            </div>
            {statusStyle && status ? (
              <Badge className={statusStyle.className}>
                <statusStyle.Icon
                  className={status.kind === 'pending' ? 'animate-spin' : undefined}
                />
                {status.text}
              </Badge>
            ) : null}
            <p className="text-sm text-muted-foreground">请使用 B站 App 扫描二维码登录</p>
            <Button variant="outline" onClick={() => void getQrCode()} disabled={loading}>
              {loading ? <Loader2 className="animate-spin" /> : <RefreshCw />}
              刷新二维码
            </Button>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="space-y-4">
            {/* 过期与超时会清空二维码，状态提示必须留在这里，否则用户看不到失败原因 */}
            {statusStyle && status ? (
              <Badge className={cn('mx-auto', statusStyle.className)}>
                <statusStyle.Icon />
                {status.text}
              </Badge>
            ) : null}
            <EmptyState
              icon={QrCode}
              title="扫码登录 B站账号"
              description="安全、快速、无需输入密码"
              className="border-0 py-6"
              action={
                <Button variant="brand" onClick={() => void getQrCode()} disabled={loading}>
                  {loading ? <Loader2 className="animate-spin" /> : <QrCode />}
                  {loading ? '生成中...' : '获取二维码'}
                </Button>
              }
            />
          </CardContent>
        </Card>
      )}
    </ViewShell>
  )
}
