import type { ComponentType, SVGProps } from 'react'
import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  Activity,
  ChevronRight,
  KeyRound,
  MessageSquare,
  QrCode,
  RefreshCw,
  Star,
  UserRound,
  Users,
} from 'lucide-react'
import { toast } from 'sonner'
import Dialog from '@/components/Dialog'
import SectionCard from '@/components/SectionCard'
import StatCard from '@/components/StatCard'
import StatusBar from '@/components/StatusBar'
import ViewShell from '@/components/ViewShell'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Separator } from '@/components/ui/separator'
import { Skeleton } from '@/components/ui/skeleton'
import { Switch } from '@/components/ui/switch'
import {
  api,
  errorMessage,
  type Account,
  type AutoReplySettings,
  type MsgSource,
} from '@/lib/ipc'
import { useAuth } from '@/state/auth'

type IconType = ComponentType<SVGProps<SVGSVGElement>>

const SOURCE_LABEL: Record<MsgSource, string> = {
  comment: '视频评论',
  dynamic: '动态评论',
  directMessage: '私信',
  follow: '关注',
}

const HISTORY_LIMIT = 5

const ENTRIES: { title: string; desc: string; Icon: IconType; to: string }[] = [
  { title: '扫码登录', desc: '使用 B站 App 扫码添加账号', Icon: QrCode, to: '/login' },
  { title: '账号管理', desc: '切换、删除与云端同步账号', Icon: Users, to: '/accounts' },
  { title: '自动回复', desc: '配置各渠道的自动回复策略', Icon: MessageSquare, to: '/auto-reply' },
  { title: '支持项目', desc: '赞助开发者，支持持续维护', Icon: Star, to: '/sponsor' },
]

function formatTime(value: string): string {
  const parsed = new Date(value)
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString('zh-CN')
}

export default function DashboardView() {
  const { isPlus, email, activateLicense } = useAuth()
  const navigate = useNavigate()

  const [accounts, setAccounts] = useState<Account[]>([])
  const [settings, setSettings] = useState<AutoReplySettings | null>(null)
  const [autostart, setAutostart] = useState(false)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState('')

  const [keyDialog, setKeyDialog] = useState(false)
  const [licenseKey, setLicenseKey] = useState('')
  const [keySubmitting, setKeySubmitting] = useState(false)
  const [keyError, setKeyError] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    setLoadError('')
    try {
      const [accountList, autoReply, autostartEnabled] = await Promise.all([
        api.getAccounts(),
        api.getAutoReplySettings(),
        api.getAutostartStatus(),
      ])
      setAccounts(accountList)
      setSettings(autoReply)
      setAutostart(autostartEnabled)
    } catch (e) {
      setLoadError(errorMessage(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const toggleAutostart = async (enabled: boolean) => {
    try {
      await api.setAutostart(enabled)
      setAutostart(enabled)
      toast.success(enabled ? '已开启开机自启' : '已关闭开机自启')
    } catch (e) {
      toast.error(`设置开机自启失败：${errorMessage(e)}`)
    }
  }

  const submitLicense = async () => {
    if (!licenseKey.trim() || keySubmitting) return
    setKeySubmitting(true)
    setKeyError('')
    try {
      await activateLicense(licenseKey.trim())
      setKeyDialog(false)
      setLicenseKey('')
      toast.success('激活成功，已解锁全部 Plus 功能')
    } catch (e) {
      setKeyError(errorMessage(e))
    } finally {
      setKeySubmitting(false)
    }
  }

  const currentAccount = accounts.find((account) => account.active) ?? null
  const channels = settings ? Object.values(settings.channels) : []
  const enabledChannels = channels.filter((channel) => channel.enabled).length
  const running = settings?.enabled === true
  const recentReplies = settings ? [...settings.history].slice(-HISTORY_LIMIT).reverse() : []
  const initialLoading = loading && accounts.length === 0 && !settings

  return (
    <ViewShell
      title="概览"
      subtitle={email ? `应用账号：${email}` : undefined}
      actions={
        <Button variant="ghost" size="sm" onClick={() => void load()} disabled={loading}>
          <RefreshCw />
          刷新
        </Button>
      }
    >
      {loadError ? <StatusBar tone="error">{loadError}</StatusBar> : null}

      {initialLoading ? (
        <>
          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            {[0, 1, 2, 3].map((index) => (
              <Skeleton key={index} className="h-28 rounded-xl" />
            ))}
          </div>
          <Skeleton className="h-48 rounded-xl" />
        </>
      ) : (
        <>
          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <StatCard
              icon={UserRound}
              label="当前账号"
              value={currentAccount?.name ?? '未登录'}
              hint={currentAccount ? `UID: ${currentAccount.uid}` : '请先扫码登录 B站账号'}
            />
            <StatCard
              icon={Users}
              label="本地账号"
              value={accounts.length}
              hint="已保存在本机的 B站账号数量"
            />
            <StatCard
              icon={Activity}
              label="运行状态"
              value={
                running ? (
                  <Badge
                    variant="outline"
                    className="border-success/30 bg-success-tint text-success"
                  >
                    运行中
                  </Badge>
                ) : (
                  <Badge variant="secondary">已停止</Badge>
                )
              }
              hint={
                settings
                  ? `完整扫描 ${settings.interval} 秒 · 快速通道 ${settings.fastInterval} 秒`
                  : '暂无自动回复配置'
              }
            />
            <StatCard
              icon={MessageSquare}
              label="已启用渠道"
              value={`${enabledChannels} / ${channels.length || 4}`}
              hint={
                settings && settings.trackedVideos.length > 0
                  ? `另有 ${settings.trackedVideos.length} 个指定视频`
                  : '视频评论 · 动态评论 · 私信 · 关注'
              }
            />
          </div>

          <SectionCard title="快捷入口">
            <div className="grid gap-3 sm:grid-cols-2">
              {ENTRIES.map(({ title, desc, Icon, to }) => (
                <button
                  key={to}
                  type="button"
                  onClick={() => navigate(to)}
                  className="group flex items-center gap-4 rounded-xl border bg-card p-4 text-left transition-colors hover:border-brand/40 hover:bg-accent"
                >
                  <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-brand/10 text-brand">
                    <Icon className="size-5" />
                  </span>
                  <span className="min-w-0 flex-1 space-y-1">
                    <span className="block text-sm font-semibold">{title}</span>
                    <span className="block text-xs text-muted-foreground">{desc}</span>
                  </span>
                  <ChevronRight className="size-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
                </button>
              ))}
            </div>
          </SectionCard>

          <SectionCard
            title="最近回复记录"
            description={recentReplies.length > 0 ? `最近 ${recentReplies.length} 条` : undefined}
            actions={
              recentReplies.length > 0 ? (
                <Button variant="ghost" size="sm" onClick={() => navigate('/auto-reply')}>
                  全部记录
                </Button>
              ) : null
            }
          >
            {recentReplies.length === 0 ? (
              <p className="py-2 text-sm text-muted-foreground">
                暂无回复记录，开启自动回复后这里会显示最近处理的评论与私信。
              </p>
            ) : (
              <div className="divide-y">
                {recentReplies.map((reply, index) => (
                  <div
                    key={`${reply.time}-${index}`}
                    className="flex items-start gap-4 py-3 first:pt-0 last:pb-0"
                  >
                    <div className="min-w-0 flex-1 space-y-1">
                      <p className="text-sm font-medium">{reply.user}</p>
                      <p className="text-sm break-words text-muted-foreground">{reply.message}</p>
                    </div>
                    <div className="flex shrink-0 flex-col items-end gap-1.5">
                      <Badge variant="secondary">{SOURCE_LABEL[reply.source]}</Badge>
                      <span className="text-xs whitespace-nowrap text-muted-foreground">
                        {formatTime(reply.time)}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </SectionCard>

          <SectionCard title="账户与系统">
            <div className="space-y-4">
              <div className="flex items-center justify-between gap-4">
                <div className="space-y-1">
                  <p className="text-sm font-medium">Plus 授权</p>
                  <p className="text-xs text-muted-foreground">
                    {isPlus
                      ? '已解锁自动回复、自动点赞等全部功能'
                      : '输入激活密钥解锁自动回复、自动点赞等全部功能'}
                  </p>
                </div>
                {isPlus ? (
                  <Badge
                    variant="outline"
                    className="border-success/30 bg-success-tint text-success"
                  >
                    Plus 已激活
                  </Badge>
                ) : (
                  <Button variant="brand" size="sm" onClick={() => setKeyDialog(true)}>
                    <KeyRound />
                    激活 Plus
                  </Button>
                )}
              </div>

              <Separator />

              <div className="flex items-center justify-between gap-4">
                <div className="space-y-1">
                  <p className="text-sm font-medium">开机自启</p>
                  <p className="text-xs text-muted-foreground">系统启动后自动运行本工具</p>
                </div>
                <Switch
                  checked={autostart}
                  onCheckedChange={(checked) => void toggleAutostart(checked)}
                  aria-label="开机自启"
                />
              </div>
            </div>
          </SectionCard>
        </>
      )}

      {keyDialog ? (
        <Dialog
          title="输入激活密钥"
          description="在爱发电购买 Plus 方案后会收到激活码，粘贴到下方即可升级。"
          onClose={() => setKeyDialog(false)}
          footer={
            <>
              <Button variant="outline" onClick={() => setKeyDialog(false)}>
                取消
              </Button>
              <Button
                variant="brand"
                onClick={() => void submitLicense()}
                disabled={!licenseKey.trim() || keySubmitting}
              >
                {keySubmitting ? '验证中...' : '激活'}
              </Button>
            </>
          }
        >
          <Input
            value={licenseKey}
            placeholder="请输入激活密钥"
            autoFocus
            onChange={(event) => setLicenseKey(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') void submitLicense()
            }}
          />
          {keyError ? <StatusBar tone="error">{keyError}</StatusBar> : null}
        </Dialog>
      ) : null}
    </ViewShell>
  )
}
