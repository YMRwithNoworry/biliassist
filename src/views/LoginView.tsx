import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import PageHeader from '../components/PageHeader'
import StatusBar from '../components/StatusBar'
import {
  IconAlert,
  IconCheck,
  IconCheckCircle,
  IconClock,
  IconQrCode,
  IconRefresh,
  IconUsers,
} from '../components/icons'
import { api, errorMessage, type UserInfo } from '../lib/ipc'

const POLL_INTERVAL = 2000
const QR_TIMEOUT = 180_000

type StatusKind = 'pending' | 'warning' | 'success' | 'error'

const STATUS_STYLE: Record<StatusKind, { badge: string; Icon: typeof IconClock }> = {
  pending: { badge: 'badge badge-info', Icon: IconClock },
  warning: { badge: 'badge badge-basic', Icon: IconCheck },
  success: { badge: 'badge badge-success', Icon: IconCheckCircle },
  error: { badge: 'badge badge-danger', Icon: IconAlert },
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
    <>
      <PageHeader title="扫码登录" subtitle="使用 B站 App 扫码即可添加账号，无需输入密码" />

      <div className="view-body">
        {error ? <StatusBar tone="error">{error}</StatusBar> : null}

        {loggedIn ? (
          <div className="card user-result">
            <span className="success-icon">
              <IconCheck size={30} strokeWidth={3} />
            </span>
            <h2 className="section-title" style={{ marginBottom: 0 }}>
              登录成功
            </h2>
            {userInfo ? (
              <div className="user-card">
                <span className="avatar-fallback">{userInfo.name.charAt(0).toUpperCase()}</span>
                <div className="user-meta">
                  <div className="user-name">{userInfo.name}</div>
                  <div className="user-uid">UID: {userInfo.uid}</div>
                </div>
              </div>
            ) : (
              <p className="qr-hint">账号已保存到本地。</p>
            )}
            <div style={{ display: 'flex', gap: 12 }}>
              <button className="btn btn-ghost" onClick={() => void getQrCode()}>
                继续添加账号
              </button>
              <button className="btn btn-primary" onClick={() => navigate('/accounts')}>
                <IconUsers size={16} />
                查看账号
              </button>
            </div>
          </div>
        ) : qrDataUrl ? (
          <div className="card qr-layout">
            <div className="qr-frame">
              <img className="qr-image" src={qrDataUrl} alt="B站登录二维码" />
            </div>
            {statusStyle && status ? (
              <span className={statusStyle.badge}>
                <statusStyle.Icon size={14} />
                {status.text}
              </span>
            ) : null}
            <p className="qr-hint">请使用 B站 App 扫描二维码登录</p>
            <button className="btn btn-ghost" onClick={() => void getQrCode()} disabled={loading}>
              <IconRefresh size={16} />
              刷新二维码
            </button>
          </div>
        ) : (
          <div className="card empty-state">
            <span className="boot-icon" style={{ margin: '0 auto' }}>
              <IconQrCode size={30} strokeWidth={1.5} />
            </span>
            <h2 className="section-title" style={{ marginTop: 16 }}>
              扫码登录 B站账号
            </h2>
            <p>安全、快速、无需输入密码</p>
            <button className="btn btn-primary" onClick={() => void getQrCode()} disabled={loading}>
              {loading ? <span className="spinner" /> : <IconQrCode size={16} />}
              {loading ? '生成中...' : '获取二维码'}
            </button>
          </div>
        )}
      </div>
    </>
  )
}
