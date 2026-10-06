import type { ReactNode } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

/** 带标题栏的卡片：应用里所有「设置分组 / 列表分组」都走这个，保证间距与描边一致。 */
export default function SectionCard({
  title,
  description,
  actions,
  children,
  className,
  contentClassName,
}: {
  title?: string
  description?: string
  actions?: ReactNode
  children: ReactNode
  className?: string
  contentClassName?: string
}) {
  const hasHeader = Boolean(title || description || actions)

  return (
    <Card className={cn('gap-0 overflow-hidden py-0', className)}>
      {hasHeader ? (
        <CardHeader className="flex flex-row items-center justify-between gap-4 border-b px-6 py-4">
          <div className="space-y-1">
            {title ? <CardTitle className="text-base font-semibold">{title}</CardTitle> : null}
            {description ? <CardDescription>{description}</CardDescription> : null}
          </div>
          {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
        </CardHeader>
      ) : null}
      <CardContent className={cn('px-6 py-5', contentClassName)}>{children}</CardContent>
    </Card>
  )
}
