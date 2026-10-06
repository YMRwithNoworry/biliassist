import type { ComponentType, ReactNode, SVGProps } from 'react'
import { Card, CardContent } from '@/components/ui/card'
import { cn } from '@/lib/utils'

export default function StatCard({
  icon: Icon,
  label,
  value,
  hint,
  className,
}: {
  icon?: ComponentType<SVGProps<SVGSVGElement>>
  label: string
  value: ReactNode
  hint?: string
  className?: string
}) {
  return (
    <Card className={cn('gap-0 py-0', className)}>
      <CardContent className="space-y-2 px-5 py-4">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          {Icon ? <Icon className="size-4 shrink-0" /> : null}
          <span>{label}</span>
        </div>
        <div className="truncate text-2xl font-semibold tracking-tight">{value}</div>
        {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
      </CardContent>
    </Card>
  )
}
