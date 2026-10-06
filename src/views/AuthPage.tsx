import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import StatusBar from '../components/StatusBar'
import { IconEye, IconEyeOff, IconLock, IconMail, IconQrCode } from '../components/icons'
import { errorMessage } from '../lib/ipc'
import { useAuth } from '../state/auth'
import ThemeToggle from '../components/ThemeToggle'

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
const RESEND_SECONDS = 60

type Mode = 'otp' | 'password'

export default function AuthPage() {
  const { signIn, sendOtp, verifyOtp } = useAuth()
  const navigate = useNavigate()

  const [mode, setMode] = useState<Mode>('otp')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [otpCode, setOtpCode] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [otpSent, setOtpSent] = useState(false)
  const [countdown, setCountdown] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')

  const isValidEmail = EMAIL_PATTERN.test(email)

  useEffect(() => {
    if (countdown <= 0) return
    const timer = window.setTimeout(() => setCountdown((value) => value - 1), 1000)
    return () => window.clearTimeout(timer)
  }, [countdown])

  const switchMode = (next: Mode) => {
    setMode(next)
    setError('')
    setSuccess('')
  }

  const handleSendOtp = async () => {
    if (!isValidEmail || busy) return
    setBusy(true)
    setError('')
    setSuccess('')
    try {
      await sendOtp(email)
      setOtpSent(true)
      setCountdown(RESEND_SECONDS)
      setSuccess('验证码已发送到您的邮箱')
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setBusy(false)
    }
  }

  const handleVerifyOtp = async () => {
    if (otpCode.length !== 6 || busy) return
    setBusy(true)
    setError('')
    try {
      await verifyOtp(email, otpCode)
      navigate('/', { replace: true })
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setBusy(false)
    }
  }

  const handlePasswordLogin = async () => {
    if (!isValidEmail || !password || busy) return
    setBusy(true)
    setError('')
    setSuccess('')
    try {
      await signIn(email, password)
      navigate('/', { replace: true })
    } catch (e) {
      setError(errorMessage(e))
    } finally {
      setBusy(false)
    }
  }

  const resetOtp = () => {
    setOtpSent(false)
    setOtpCode('')
    setCountdown(0)
    setError('')
    setSuccess('')
  }

  return (
    <div className="auth-screen">
      <ThemeToggle className="theme-toggle-floating" />
      <div className="auth-container">
        <div className="auth-header">
          <span className="auth-logo">
            <IconLock size={26} />
          </span>
          <h1 className="auth-title">B站账号管理工具</h1>
          <p className="auth-subtitle">登录后可使用云端同步与全部功能</p>
        </div>

        <div className="card auth-card">
          <div className="auth-tabs">
            <button
              className={mode === 'otp' ? 'auth-tab active' : 'auth-tab'}
              onClick={() => switchMode('otp')}
            >
              <IconMail size={16} />
              验证码登录
            </button>
            <button
              className={mode === 'password' ? 'auth-tab active' : 'auth-tab'}
              onClick={() => switchMode('password')}
            >
              <IconLock size={16} />
              密码登录
            </button>
          </div>

          {error ? <StatusBar tone="error">{error}</StatusBar> : null}
          {success ? <StatusBar tone="success">{success}</StatusBar> : null}

          {mode === 'otp' ? (
            <>
              <div className="field">
                <label className="field-label" htmlFor="otp-email">
                  邮箱地址
                </label>
                <input
                  id="otp-email"
                  className="input"
                  type="email"
                  placeholder="请输入邮箱"
                  value={email}
                  disabled={otpSent}
                  onChange={(event) => setEmail(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' && !otpSent) void handleSendOtp()
                  }}
                />
              </div>

              {otpSent ? (
                <>
                  <div className="field">
                    <label className="field-label" htmlFor="otp-code">
                      验证码
                    </label>
                    <input
                      id="otp-code"
                      className="input"
                      inputMode="numeric"
                      maxLength={6}
                      placeholder="请输入 6 位验证码"
                      value={otpCode}
                      onChange={(event) => setOtpCode(event.target.value.replace(/\D/g, ''))}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter') void handleVerifyOtp()
                      }}
                    />
                  </div>

                  <button
                    className="btn btn-primary btn-block"
                    onClick={() => void handleVerifyOtp()}
                    disabled={otpCode.length !== 6 || busy}
                  >
                    {busy ? <span className="spinner" /> : null}
                    {busy ? '验证中...' : '登录'}
                  </button>

                  <div className="form-footer">
                    <button
                      className="btn-text"
                      onClick={() => void handleSendOtp()}
                      disabled={countdown > 0 || busy}
                    >
                      {countdown > 0 ? `${countdown}s 后重新发送` : '重新发送验证码'}
                    </button>
                    <button className="btn-text" onClick={resetOtp}>
                      更换邮箱
                    </button>
                  </div>

                  <p className="form-note">
                    验证码已发送至 <strong>{email}</strong>
                  </p>
                </>
              ) : (
                <button
                  className="btn btn-primary btn-block"
                  onClick={() => void handleSendOtp()}
                  disabled={!isValidEmail || busy}
                >
                  {busy ? <span className="spinner" /> : null}
                  {busy ? '发送中...' : '发送验证码'}
                </button>
              )}
            </>
          ) : (
            <>
              <div className="field">
                <label className="field-label" htmlFor="login-email">
                  邮箱地址
                </label>
                <input
                  id="login-email"
                  className="input"
                  type="email"
                  placeholder="请输入邮箱"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') void handlePasswordLogin()
                  }}
                />
              </div>

              <div className="field">
                <label className="field-label" htmlFor="login-password">
                  密码
                </label>
                <div className="password-field">
                  <input
                    id="login-password"
                    className="input"
                    type={showPassword ? 'text' : 'password'}
                    placeholder="请输入密码"
                    value={password}
                    onChange={(event) => setPassword(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') void handlePasswordLogin()
                    }}
                  />
                  <button
                    className="password-toggle"
                    type="button"
                    aria-label={showPassword ? '隐藏密码' : '显示密码'}
                    onClick={() => setShowPassword((value) => !value)}
                  >
                    {showPassword ? <IconEyeOff size={16} /> : <IconEye size={16} />}
                  </button>
                </div>
              </div>

              <button
                className="btn btn-primary btn-block"
                onClick={() => void handlePasswordLogin()}
                disabled={!isValidEmail || !password || busy}
              >
                {busy ? <span className="spinner" /> : null}
                {busy ? '登录中...' : '登录'}
              </button>
            </>
          )}
        </div>

        <p className="auth-footer">
          <IconQrCode size={13} style={{ verticalAlign: '-2px', marginRight: 4 }} />
          登录后即可使用扫码添加 B站账号与云端同步
        </p>
      </div>
    </div>
  )
}
