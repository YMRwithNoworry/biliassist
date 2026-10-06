import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import Dialog from '../components/Dialog'
import PageHeader from '../components/PageHeader'
import StatusBar, { type StatusTone } from '../components/StatusBar'
import {
  IconCheck,
  IconCloud,
  IconDownload,
  IconRefresh,
  IconTrash,
  IconUpload,
  IconUsers,
} from '../components/icons'
import { api, errorMessage, type Account } from '../lib/ipc'
import { useAuth } from '../state/auth'

export default function AccountsView() {
  const { email, isAuthenticated } = useAuth()
  const navigate = useNavigate()

  const [accounts, setAccounts] = useState<Account[]>([])
  const [loading, setLoading] = useState(true)
  const [syncing, setSyncing] = useState(false)
  const [notice, setNotice] = useState<{ tone: StatusTone; text: string } | null>(null)
  const [brokenAvatars, setBrokenAvatars] = useState<string[]>([])
  const [pendingDelete, setPendingDelete] = useState<Account | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setAccounts(await api.getAccounts())
    } catch (e) {
      setNotice({ tone: 'error', text: `加载账号失败：${errorMessage(e)}` })
      setAccounts([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const activate = async (uid: string) => {
    setNotice(null)
    try {
      await api.activateAccount(uid)
      await load()
      setNotice({ tone: 'success', text: '已切换当前账号' })
    } catch (e) {
      setNotice({ tone: 'error', text: `切换账号失败：${errorMessage(e)}` })
    }
  }

  const confirmDelete = async () => {
    if (!pendingDelete) return
    const { uid, name } = pendingDelete
    setPendingDelete(null)
    setNotice(null)
    try {
      await api.deleteAccount(uid)
      await load()
      setNotice({ tone: 'success', text: `已删除账号 ${name}` })
    } catch (e) {
      setNotice({ tone: 'error', text: `删除账号失败：${errorMessage(e)}` })
    }
  }

  const upload = async () => {
    if (!isAuthenticated) {
      setNotice({ tone: 'error', text: '请先登录应用账号后再同步' })
      return
    }
    setSyncing(true)
    setNotice(null)
    try {
      setNotice({ tone: 'success', text: await api.cloudUploadAll() })
    } catch (e) {
      setNotice({ tone: 'error', text: `上传失败：${errorMessage(e)}` })
    } finally {
      setSyncing(false)
    }
  }

  const download = async () => {
    if (!isAuthenticated) {
      setNotice({ tone: 'error', text: '请先登录应用账号后再同步' })
      return
    }
    setSyncing(true)
    setNotice(null)
    try {
      const result = await api.cloudDownloadAll()
      await load()
      setNotice(
        result.downloadedCount > 0
          ? { tone: 'success', text: `已从云端同步 ${result.downloadedCount} 项数据` }
          : { tone: 'info', text: '云端暂无可同步的数据' },
      )
    } catch (e) {
      setNotice({ tone: 'error', text: `下载失败：${errorMessage(e)}` })
    } finally {
      setSyncing(false)
    }
  }

  return (
    <>
      <PageHeader
        title="账号管理"
        subtitle={`本地已保存 ${accounts.length} 个账号`}
        actions={
          <>
            <button className="btn btn-ghost btn-sm" onClick={() => void load()} disabled={loading}>
              <IconRefresh size={15} />
              刷新
            </button>
            <button className="btn btn-primary btn-sm" onClick={() => navigate('/login')}>
              <IconUsers size={15} />
              添加账号
            </button>
          </>
        }
      />

      <div className="view-body">
        <div className="sync-bar">
          <div className="sync-info">
            <IconCloud size={16} />
            <span className="sync-label">全量数据同步</span>
            <span className="sync-user" title={email}>
              {email || '未登录应用账号'}
            </span>
          </div>
          <div className="sync-actions">
            <button className="btn btn-ghost btn-sm" onClick={() => void upload()} disabled={syncing}>
              <IconUpload size={15} />
              上传
            </button>
            <button className="btn btn-ghost btn-sm" onClick={() => void download()} disabled={syncing}>
              <IconDownload size={15} />
              下载
            </button>
          </div>
        </div>

        {notice ? <StatusBar tone={notice.tone}>{notice.text}</StatusBar> : null}

        {loading ? (
          <div className="loading-block">
            <span className="spinner spinner-lg" />
            <span>加载中...</span>
          </div>
        ) : accounts.length === 0 ? (
          <div className="card empty-state">
            <span className="boot-icon" style={{ margin: '0 auto' }}>
              <IconUsers size={30} strokeWidth={1.5} />
            </span>
            <h2 className="section-title" style={{ marginTop: 16 }}>
              暂无 B站账号
            </h2>
            <p>请先扫码登录，或从云端下载已保存的数据</p>
            <button className="btn btn-primary" onClick={() => navigate('/login')}>
              扫码登录
            </button>
          </div>
        ) : (
          <div className="account-list">
            {accounts.map((account) => (
              <div
                key={account.uid}
                className={account.active ? 'account-card active' : 'account-card'}
              >
                <div className="account-avatar">
                  {account.avatar && !brokenAvatars.includes(account.uid) ? (
                    <img
                      className="avatar"
                      src={account.avatar}
                      alt={account.name}
                      referrerPolicy="no-referrer"
                      onError={() => setBrokenAvatars((prev) => [...prev, account.uid])}
                    />
                  ) : (
                    <span className="avatar-fallback">
                      {account.name.charAt(0).toUpperCase()}
                    </span>
                  )}
                  {account.active ? <span className="active-dot" /> : null}
                </div>

                <div className="account-info">
                  <div className="account-name">{account.name}</div>
                  <div className="account-uid">UID: {account.uid}</div>
                </div>

                <div className="account-actions">
                  {account.active ? (
                    <span className="badge badge-plus">
                      <IconCheck size={13} />
                      当前
                    </span>
                  ) : (
                    <button className="btn btn-ghost btn-sm" onClick={() => void activate(account.uid)}>
                      设为当前
                    </button>
                  )}
                  <button
                    className="icon-btn"
                    title="删除账号"
                    aria-label={`删除账号 ${account.name}`}
                    onClick={() => setPendingDelete(account)}
                  >
                    <IconTrash size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {pendingDelete ? (
        <Dialog
          title="删除账号"
          onClose={() => setPendingDelete(null)}
          footer={
            <>
              <button className="btn btn-ghost" onClick={() => setPendingDelete(null)}>
                取消
              </button>
              <button className="btn btn-accent" onClick={() => void confirmDelete()}>
                删除
              </button>
            </>
          }
        >
          <p className="dialog-desc">
            确定要删除账号「{pendingDelete.name}」吗？本地保存的 Cookie 会一并移除，且无法恢复。
          </p>
        </Dialog>
      ) : null}
    </>
  )
}
