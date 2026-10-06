import type { ReactNode } from 'react'
import PageHeader from '@/components/PageHeader'
import { cn } from '@/lib/utils'

/** 页面骨架：固定页头 + 可滚动内容区。所有视图都用它，保证间距一致。 */
export default function ViewShell({
  title,
  subtitle,
  actions,
  children,
  maxWidth = 'max-w-5xl',
}: {
  title: string
  subtitle?: string
  actions?: ReactNode
  children: ReactNode
  maxWidth?: string
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title={title} subtitle={subtitle} actions={actions} />
      <div className="min-h-0 flex-1 overflow-y-auto p-8">
        <div className={cn('mx-auto flex w-full flex-col gap-6', maxWidth)}>{children}</div>
      </div>
    </div>
  )
}
