import type { ReactNode } from 'react'

export default function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string
  subtitle?: string
  actions?: ReactNode
}) {
  return (
    <header className="view-header">
      <div className="view-header-text">
        <h1 className="view-title">{title}</h1>
        {subtitle ? <p className="view-subtitle">{subtitle}</p> : null}
      </div>
      {actions ? <div className="view-header-actions">{actions}</div> : null}
    </header>
  )
}
