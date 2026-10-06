import type { ReactNode } from 'react'
import { IconAlert, IconCheckCircle, IconClock } from './icons'

export type StatusTone = 'success' | 'error' | 'info' | 'warning'

const toneIcon: Record<StatusTone, (props: { size?: number }) => ReactNode> = {
  success: IconCheckCircle,
  error: IconAlert,
  warning: IconAlert,
  info: IconClock,
}

export default function StatusBar({
  tone,
  children,
}: {
  tone: StatusTone
  children: ReactNode
}) {
  const Glyph = toneIcon[tone]
  return (
    <div className={`status-bar status-bar-${tone}`} role="status">
      <Glyph size={16} />
      <span>{children}</span>
    </div>
  )
}
