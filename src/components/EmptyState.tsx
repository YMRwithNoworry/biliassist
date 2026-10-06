import type { ComponentType, ReactNode, SVGProps } from 'react'
import { cn } from '@/lib/utils'

export default function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: {
  icon?: ComponentType<SVGProps<SVGSVGElement>>
  title: string
  description?: string
  action?: ReactNode
  className?: string
}) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed px-6 py-12 text-center',
        className,
      )}
    >
      {Icon ? <Icon className="size-7 text-muted-foreground" /> : null}
      <div className="space-y-1">
        <p className="font-medium">{title}</p>
        {description ? <p className="text-sm text-muted-foreground">{description}</p> : null}
      </div>
      {action}
    </div>
  )
}
