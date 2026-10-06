import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Eye, EyeOff, Loader2, Lock, Mail, QrCode } from 'lucide-react'
import StatusBar from '@/components/StatusBar'
import ThemeToggle from '@/components/ThemeToggle'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { errorMessage } from '@/lib/ipc'
import { useAuth } from '@/state/auth'

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
    <div className="flex min-h-screen items-center justify-center bg-background px-6 py-12">
      <ThemeToggle variant="outline" className="fixed right-6 top-6" />

      <div className="w-full max-w-md space-y-6">
        <header className="flex flex-col items-center gap-3 text-center">
          <span className="flex size-14 items-center justify-center rounded-2xl bg-brand text-brand-foreground">
            <Lock className="size-6" />
          </span>
          <div className="space-y-1">
            <h1 className="text-2xl font-semibold tracking-tight">B站账号管理工具</h1>
            <p className="text-sm text-muted-foreground">登录后可使用云端同步与全部功能</p>
          </div>
        </header>

        <Card>
          <CardContent className="space-y-4">
            <Tabs value={mode} onValueChange={(value) => switchMode(value as Mode)}>
              <TabsList className="w-full">
                <TabsTrigger value="otp">
                  <Mail />
                  验证码登录
                </TabsTrigger>
                <TabsTrigger value="password">
                  <Lock />
                  密码登录
                </TabsTrigger>
              </TabsList>

              {error ? <StatusBar tone="error" className="mt-4">{error}</StatusBar> : null}
              {success ? (
                <StatusBar tone="success" className="mt-4">
                  {success}
                </StatusBar>
              ) : null}

              <TabsContent value="otp" className="space-y-4 pt-4">
                <div className="space-y-2">
                  <Label htmlFor="otp-email">邮箱地址</Label>
                  <Input
                    id="otp-email"
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
                    <div className="space-y-2">
                      <Label htmlFor="otp-code">验证码</Label>
                      <Input
                        id="otp-code"
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

                    <Button
                      className="w-full"
                      variant="brand"
                      onClick={() => void handleVerifyOtp()}
                      disabled={otpCode.length !== 6 || busy}
                    >
                      {busy ? <Loader2 className="animate-spin" /> : null}
                      {busy ? '验证中...' : '登录'}
                    </Button>

                    <div className="flex items-center justify-between text-sm">
                      <Button
                        variant="link"
                        size="sm"
                        className="h-auto px-0"
                        onClick={() => void handleSendOtp()}
                        disabled={countdown > 0 || busy}
                      >
                        {countdown > 0 ? `${countdown}s 后重新发送` : '重新发送验证码'}
                      </Button>
                      <Button
                        variant="link"
                        size="sm"
                        className="h-auto px-0 text-muted-foreground"
                        onClick={resetOtp}
                      >
                        更换邮箱
                      </Button>
                    </div>

                    <p className="text-xs text-muted-foreground">
                      验证码已发送至 <span className="font-medium text-foreground">{email}</span>
                    </p>
                  </>
                ) : (
                  <Button
                    className="w-full"
                    variant="brand"
                    onClick={() => void handleSendOtp()}
                    disabled={!isValidEmail || busy}
                  >
                    {busy ? <Loader2 className="animate-spin" /> : null}
                    {busy ? '发送中...' : '发送验证码'}
                  </Button>
                )}
              </TabsContent>

              <TabsContent value="password" className="space-y-4 pt-4">
                <div className="space-y-2">
                  <Label htmlFor="login-email">邮箱地址</Label>
                  <Input
                    id="login-email"
                    type="email"
                    placeholder="请输入邮箱"
                    value={email}
                    onChange={(event) => setEmail(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') void handlePasswordLogin()
                    }}
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="login-password">密码</Label>
                  <div className="relative">
                    <Input
                      id="login-password"
                      className="pr-10"
                      type={showPassword ? 'text' : 'password'}
                      placeholder="请输入密码"
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter') void handlePasswordLogin()
                      }}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      className="absolute right-1 top-1/2 -translate-y-1/2 text-muted-foreground"
                      aria-label={showPassword ? '隐藏密码' : '显示密码'}
                      onClick={() => setShowPassword((value) => !value)}
                    >
                      {showPassword ? <EyeOff /> : <Eye />}
                    </Button>
                  </div>
                </div>

                <Button
                  className="w-full"
                  variant="brand"
                  onClick={() => void handlePasswordLogin()}
                  disabled={!isValidEmail || !password || busy}
                >
                  {busy ? <Loader2 className="animate-spin" /> : null}
                  {busy ? '登录中...' : '登录'}
                </Button>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

        <p className="flex items-center justify-center gap-1.5 text-center text-xs text-muted-foreground">
          <QrCode className="size-3.5" />
          登录后即可使用扫码添加 B站账号与云端同步
        </p>
      </div>
    </div>
  )
}
