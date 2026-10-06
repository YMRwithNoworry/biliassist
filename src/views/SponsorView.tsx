import { useState } from 'react'
import { Check, Copy, ExternalLink, Heart, ZoomIn } from 'lucide-react'
import { toast } from 'sonner'
import SectionCard from '@/components/SectionCard'
import StatusBar from '@/components/StatusBar'
import ViewShell from '@/components/ViewShell'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { api, errorMessage } from '@/lib/ipc'
import { cn } from '@/lib/utils'

const PURCHASE_URL = 'https://www.ifdian.net/a/Alkut?tab=home'

const FEATURES = [
  '持续维护与问题修复',
  '新功能开发与体验优化',
  '服务器与基础设施开销',
  '文档与使用教程制作',
]

export default function SponsorView() {
  const [zoomed, setZoomed] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)

  const openPurchasePage = async () => {
    setError('')
    try {
      await api.openExternalUrl(PURCHASE_URL)
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  const copyPurchaseLink = async () => {
    setError('')
    try {
      await api.copyTextToClipboard(PURCHASE_URL)
      setCopied(true)
      toast.success('链接已复制到剪贴板')
      window.setTimeout(() => setCopied(false), 2000)
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  return (
    <ViewShell title="赞助支持" subtitle="您的支持是持续维护的动力" maxWidth="max-w-3xl">
      {error ? <StatusBar tone="error">{error}</StatusBar> : null}

      <Card>
        <CardContent className="flex flex-col items-center gap-3 text-center">
          <span className="flex size-14 items-center justify-center rounded-full bg-brand text-brand-foreground">
            <Heart className="size-7" />
          </span>
          <h2 className="text-lg font-semibold">支持 B站账号管理工具</h2>
          <p className="max-w-xl text-sm text-muted-foreground">
            感谢您的使用！如果这个工具对您有帮助，欢迎扫码赞助。您的支持会用于修复问题、
            开发新功能，并保证项目长期可用。
          </p>
        </CardContent>
      </Card>

      <SectionCard title="扫码赞助" description="使用微信或支付宝扫描下方二维码">
        <div className="flex flex-col items-center gap-3">
          <button
            type="button"
            aria-label={zoomed ? '单击缩小二维码' : '单击放大二维码'}
            onClick={() => setZoomed((value) => !value)}
            className={cn(
              'flex items-center justify-center rounded-xl border bg-card p-4 transition-all',
              zoomed ? 'cursor-zoom-out' : 'cursor-zoom-in',
            )}
          >
            <img
              className={cn('rounded-lg transition-all', zoomed ? 'size-80' : 'size-52')}
              src="/sponsor-qr.png"
              alt="赞助二维码"
            />
          </button>
          <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <ZoomIn className="size-3.5" />
            {zoomed ? '单击缩小' : '单击放大'}
          </p>
        </div>
      </SectionCard>

      <SectionCard title="爱发电" description="订阅 Plus 方案解锁全部功能">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <Button variant="brand" className="sm:flex-1" onClick={() => void openPurchasePage()}>
            <ExternalLink />
            爱发电购买 Plus 方案
          </Button>
          <Button variant="outline" onClick={() => void copyPurchaseLink()}>
            {copied ? <Check /> : <Copy />}
            {copied ? '已复制' : '复制链接'}
          </Button>
        </div>
        <p className="mt-3 text-xs text-muted-foreground">ifdian.net/a/Alkut</p>
      </SectionCard>

      <SectionCard title="您的支持将用于">
        <ul className="space-y-2.5">
          {FEATURES.map((feature) => (
            <li className="flex items-center gap-2.5 text-sm" key={feature}>
              <span className="size-1.5 shrink-0 rounded-full bg-brand" />
              <span>{feature}</span>
            </li>
          ))}
        </ul>
      </SectionCard>

      <Card>
        <CardContent className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
          <Heart className="size-4 text-brand" />
          感谢您的每一份支持！
        </CardContent>
      </Card>
    </ViewShell>
  )
}
