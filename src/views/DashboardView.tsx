import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import Dialog from '../components/Dialog'
import PageHeader from '../components/PageHeader'
import StatusBar, { type StatusTone } from '../components/StatusBar'
import {
  IconChat,
  IconCheck,
  IconChevronRight,
  IconKey,
  IconLock,
  IconPulse,
  IconRefresh,
  IconStar,
  IconUsers,
} from '../components/icons'
import { api, errorMessage, type Account, type AutoReplySettings, type MsgSource } from '../lib/ipc'
import { useAuth } from '../state/auth'

const SOURCE_LABEL: Record<MsgSource, string> = {
  comment: '视频评论',
  dynamic: '动态评论',
  directMessage: '私信',
  follow: '关注',
}

const HISTORY_LIMIT = 5

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
  const [error, setError] = useState('')
  const [notice, setNotice] = useState<{ tone: StatusTone; text: string } | null>(null)

  const [keyDialog, setKeyDialog] = useState(false)
  const [licenseKey, setLicenseKey] = useState('')
  const [keySubmitting, setKeySubmitting] = useState(false)
  const [keyError, setKeyError] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
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
      setError(errorMessage(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const toggleAutostart = async (enabled: boolean) => {
    setNotice(null)
    try {
      await api.setAutostart(enabled)
      setAutostart(enabled)
      setNotice({ tone: 'success', text: enabled ? '已开启开机自启' : '已关闭开机自启' })
    } catch (e) {
      setNotice({ tone: 'error', text: `设置开机自启失败：${errorMessage(e)}` })
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
      setNotice({ tone: 'success', text: '激活成功，已解锁全部 Plus 功能' })
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

  const entries = [
    { title: '扫码登录', desc: '使用 B站 App 扫码添加账号', Icon: IconLock, to: '/login' },
    { title: '账号管理', desc: '切换、删除与云端同步账号', Icon: IconUsers, to: '/accounts' },
    { title: '自动回复', desc: '配置各渠道的自动回复策略', Icon: IconChat, to: '/auto-reply' },
    { title: '支持项目', desc: '赞助开发者，支持持续维护', Icon: IconStar, to: '/sponsor' },
  ]

  return (
    <>
      <PageHeader
        title="概览"
        subtitle={email ? `应用账号：${email}` : undefined}
        actions={
          <button className="btn btn-ghost btn-sm" onClick={() => void load()} disabled={loading}>
            <IconRefresh size={15} />
            刷新
          </button>
        }
      />

      <div className="view-body">
        {notice ? <StatusBar tone={notice.tone}>{notice.text}</StatusBar> : null}
        {error ? <StatusBar tone="error">{error}</StatusBar> : null}

        {loading && accounts.length === 0 && !settings ? (
          <div className="loading-block">
            <span className="spinner spinner-lg" />
            <span>加载中...</span>
          </div>
        ) : (
          <>
            <section className="stat-grid">
              <div className="stat-card">
                <span className="stat-head">
                  <IconUsers size={15} />
                  当前账号
                </span>
                <span className="stat-value">{currentAccount?.name ?? '未登录'}</span>
                <span className="stat-hint">
                  {currentAccount ? `UID: ${currentAccount.uid}` : '请先扫码登录 B站账号'}
                </span>
              </div>

              <div className="stat-card">
                <span className="stat-head">
                  <IconUsers size={15} />
                  本地账号
                </span>
                <span className="stat-value">{accounts.length}</span>
                <span className="stat-hint">已保存在本机的 B站账号数量</span>
              </div>

              <div className="stat-card">
                <span className="stat-head">
                  <IconPulse size={15} />
                  运行状态
                </span>
                <span className="stat-value">
                  {running ? (
                    <span className="badge badge-success">运行中</span>
                  ) : (
                    <span className="badge badge-muted">已停止</span>
                  )}
                </span>
                <span className="stat-hint">
                  {settings
                    ? `完整扫描 ${settings.interval} 秒 · 快速通道 ${settings.fastInterval} 秒`
                    : '暂无自动回复配置'}
                </span>
              </div>

              <div className="stat-card">
                <span className="stat-head">
                  <IconChat size={15} />
                  已启用渠道
                </span>
                <span className="stat-value">{enabledChannels} / {channels.length || 4}</span>
                <span className="stat-hint">
                  {settings && settings.trackedVideos.length > 0
                    ? `另有 ${settings.trackedVideos.length} 个指定视频`
                    : '视频评论 · 动态评论 · 私信 · 关注'}
                </span>
              </div>
            </section>

            <section>
              <h2 className="section-title">快捷入口</h2>
              <div className="entry-grid">
                {entries.map(({ title, desc, Icon, to }) => (
                  <button key={to} className="entry-card" onClick={() => navigate(to)}>
                    <span className="entry-icon">
                      <Icon size={20} />
                    </span>
                    <span className="entry-content">
                      <span className="entry-title">{title}</span>
                      <span className="entry-desc">{desc}</span>
                    </span>
                    <IconChevronRight size={18} className="entry-arrow" />
                  </button>
                ))}
              </div>
            </section>

            <section className="card">
              <div className="card-header-row">
                <h2 className="section-title">最近回复记录</h2>
                {recentReplies.length > 0 ? (
                  <button className="btn btn-ghost btn-sm" onClick={() => navigate('/auto-reply')}>
                    全部记录
                  </button>
                ) : null}
              </div>
              {recentReplies.length === 0 ? (
                <p className="dialog-desc">暂无回复记录，开启自动回复后这里会显示最近处理的评论与私信。</p>
              ) : (
                <div className="history-list">
                  {recentReplies.map((reply, index) => (
                    <div className="history-item" key={`${reply.time}-${index}`}>
                      <div className="history-main">
                        <div className="history-user">{reply.user}</div>
                        <div className="history-message">{reply.message}</div>
                      </div>
                      <div className="history-meta">
                        <span className="badge badge-info">{SOURCE_LABEL[reply.source]}</span>
                        <span className="history-time">{formatTime(reply.time)}</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>

            <section className="card">
              <h2 className="section-title">账户与系统</h2>
              <div className="toggle-row">
                <div>
                  <div className="toggle-label">Plus 授权</div>
                  <div className="toggle-desc">
                    {isPlus ? '已解锁自动回复、自动点赞等全部功能' : '输入激活密钥解锁自动回复、自动点赞等全部功能'}
                  </div>
                </div>
                {isPlus ? (
                  <span className="badge badge-plus">
                    <IconCheck size={13} />
                    已激活
                  </span>
                ) : (
                  <button className="btn btn-accent btn-sm" onClick={() => setKeyDialog(true)}>
                    <IconKey size={15} />
                    激活 Plus
                  </button>
                )}
              </div>
              <div className="toggle-row">
                <div>
                  <div className="toggle-label">开机自启</div>
                  <div className="toggle-desc">系统启动后自动运行本工具</div>
                </div>
                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={autostart}
                    onChange={(event) => void toggleAutostart(event.target.checked)}
                  />
                  <span className="toggle-track" />
                </label>
              </div>
            </section>
          </>
        )}
      </div>

      {keyDialog ? (
        <Dialog
          title="输入激活密钥"
          onClose={() => setKeyDialog(false)}
          footer={
            <>
              <button className="btn btn-ghost" onClick={() => setKeyDialog(false)}>
                取消
              </button>
              <button
                className="btn btn-accent"
                onClick={() => void submitLicense()}
                disabled={!licenseKey.trim() || keySubmitting}
              >
                {keySubmitting ? '验证中...' : '激活'}
              </button>
            </>
          }
        >
          <p className="dialog-desc">
            在爱发电购买 Plus 方案后会收到激活码，粘贴到下方即可升级。
          </p>
          <input
            className="input"
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
    </>
  )
}
