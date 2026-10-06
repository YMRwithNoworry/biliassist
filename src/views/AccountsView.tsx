import { useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Check, Download, Plus, RefreshCw, Trash2, Upload, Users } from 'lucide-react'
import { toast } from 'sonner'
import Dialog from '@/components/Dialog'
import EmptyState from '@/components/EmptyState'
import SectionCard from '@/components/SectionCard'
import StatusBar from '@/components/StatusBar'
import ViewShell from '@/components/ViewShell'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { api, errorMessage, type Account } from '@/lib/ipc'
import { useAuth } from '@/state/auth'
import { cn } from '@/lib/utils'

export default function AccountsView() {
  const { email, isAuthenticated } = useAuth()
  const navigate = useNavigate()

  const [accounts, setAccounts] = useState<Account[]>([])
  const [loading, setLoading] = useState(true)
  const [syncing, setSyncing] = useState(false)
  const [loadError, setLoadError] = useState('')
  const [pendingDelete, setPendingDelete] = useState<Account | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setLoadError('')
    try {
      setAccounts(await api.getAccounts())
    } catch (e) {
      setLoadError(`加载账号失败：${errorMessage(e)}`)
      setAccounts([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const activate = async (uid: string) => {
    try {
      await api.activateAccount(uid)
      await load()
      toast.success('已切换当前账号')
    } catch (e) {
      toast.error(`切换账号失败：${errorMessage(e)}`)
    }
  }

  const confirmDelete = async () => {
    if (!pendingDelete) return
    const { uid, name } = pendingDelete
    setPendingDelete(null)
    try {
      await api.deleteAccount(uid)
      await load()
      toast.success(`已删除账号 ${name}`)
    } catch (e) {
      toast.error(`删除账号失败：${errorMessage(e)}`)
    }
  }

  const upload = async () => {
    if (!isAuthenticated) {
      toast.error('请先登录应用账号后再同步')
      return
    }
    setSyncing(true)
    try {
      toast.success(await api.cloudUploadAll())
    } catch (e) {
      toast.error(`上传失败：${errorMessage(e)}`)
    } finally {
      setSyncing(false)
    }
  }

  const download = async () => {
    if (!isAuthenticated) {
      toast.error('请先登录应用账号后再同步')
      return
    }
    setSyncing(true)
    try {
      const result = await api.cloudDownloadAll()
      await load()
      if (result.downloadedCount > 0) {
        toast.success(`已从云端同步 ${result.downloadedCount} 项数据`)
      } else {
        toast.info('云端暂无可同步的数据')
      }
    } catch (e) {
      toast.error(`下载失败：${errorMessage(e)}`)
    } finally {
      setSyncing(false)
    }
  }

  return (
    <ViewShell
      title="账号管理"
      subtitle={`本地已保存 ${accounts.length} 个账号`}
      actions={
        <>
          <Button variant="ghost" size="sm" onClick={() => void load()} disabled={loading}>
            <RefreshCw />
            刷新
          </Button>
          <Button size="sm" onClick={() => navigate('/login')}>
            <Plus />
            添加账号
          </Button>
        </>
      }
    >
      {loadError ? <StatusBar tone="error">{loadError}</StatusBar> : null}

      <SectionCard
        title="全量数据同步"
        description={email ? `应用账号：${email}` : '未登录应用账号，请先登录后再同步'}
        actions={
          <>
            <Button variant="outline" size="sm" onClick={() => void upload()} disabled={syncing}>
              <Upload />
              上传
            </Button>
            <Button variant="outline" size="sm" onClick={() => void download()} disabled={syncing}>
              <Download />
              下载
            </Button>
          </>
        }
      >
        <p className="text-sm text-muted-foreground">
          上传会把本地账号、自动回复配置与去重记录同步到云端；下载会用云端数据覆盖本地配置。
        </p>
      </SectionCard>

      {loading ? (
        <div className="grid gap-3">
          {[0, 1, 2].map((index) => (
            <Skeleton key={index} className="h-20 rounded-xl" />
          ))}
        </div>
      ) : accounts.length === 0 ? (
        <EmptyState
          icon={Users}
          title="暂无 B站账号"
          description="请先扫码登录，或从云端下载已保存的数据"
          action={<Button onClick={() => navigate('/login')}>扫码登录</Button>}
        />
      ) : (
        <div className="grid gap-3">
          {accounts.map((account) => (
            <div
              key={account.uid}
              className={cn(
                'flex items-center gap-4 rounded-xl border bg-card p-4 transition-colors',
                account.active && 'border-brand bg-brand/5',
              )}
            >
              <div className="relative">
                <Avatar size="lg" className="size-12">
                  <AvatarImage
                    src={account.avatar || undefined}
                    alt={account.name}
                    referrerPolicy="no-referrer"
                  />
                  <AvatarFallback className="text-base font-semibold">
                    {account.name.charAt(0).toUpperCase()}
                  </AvatarFallback>
                </Avatar>
                {account.active ? (
                  <span className="absolute -right-0.5 -bottom-0.5 size-3.5 rounded-full border-2 border-background bg-brand" />
                ) : null}
              </div>

              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-semibold">{account.name}</p>
                <p className="text-xs text-muted-foreground">UID: {account.uid}</p>
              </div>

              <div className="flex shrink-0 items-center gap-2">
                {account.active ? (
                  <Badge
                    variant="outline"
                    className="border-success/30 bg-success-tint text-success"
                  >
                    <Check />
                    当前
                  </Badge>
                ) : (
                  <Button variant="outline" size="sm" onClick={() => void activate(account.uid)}>
                    设为当前
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-foreground hover:text-destructive"
                  title="删除账号"
                  aria-label={`删除账号 ${account.name}`}
                  onClick={() => setPendingDelete(account)}
                >
                  <Trash2 />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {pendingDelete ? (
        <Dialog
          title="删除账号"
          description={`确定要删除账号「${pendingDelete.name}」吗？本地保存的 Cookie 会一并移除，且无法恢复。`}
          onClose={() => setPendingDelete(null)}
          footer={
            <>
              <Button variant="outline" onClick={() => setPendingDelete(null)}>
                取消
              </Button>
              <Button variant="destructive" onClick={() => void confirmDelete()}>
                删除
              </Button>
            </>
          }
        >
          <p className="text-sm text-muted-foreground">UID: {pendingDelete.uid}</p>
        </Dialog>
      ) : null}
    </ViewShell>
  )
}
