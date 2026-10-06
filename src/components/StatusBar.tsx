import { CircleCheck, Info, TriangleAlert } from 'lucide-react'
import type { ComponentType, ReactNode, SVGProps } from 'react'
import { cn } from '@/lib/utils'

export type StatusTone = 'success' | 'error' | 'info' | 'warning'

const TONES: Record<
  StatusTone,
  { Icon: ComponentType<SVGProps<SVGSVGElement>>; className: string }
> = {
  success: { Icon: CircleCheck, className: 'bg-success-tint text-success' },
  error: { Icon: TriangleAlert, className: 'bg-destructive/10 text-destructive' },
  warning: { Icon: TriangleAlert, className: 'bg-warning-tint text-warning' },
  info: { Icon: Info, className: 'bg-info-tint text-info' },
}

export default function StatusBar({
  tone,
  children,
  className,
}: {
  tone: StatusTone
  children: ReactNode
  className?: string
}) {
  const { Icon, className: toneClassName } = TONES[tone]

  return (
    <div
      role="status"
      className={cn(
        'flex items-start gap-2 rounded-lg px-3 py-2 text-sm',
        toneClassName,
        className,
      )}
    >
      <Icon className="mt-0.5 size-4 shrink-0" />
      <span className="min-w-0 break-words">{children}</span>
    </div>
  )
}
