import type { ReactNode } from 'react'
import {
  Dialog as DialogRoot,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

/** 应用统一的表单对话框：保持「传 children + footer」的简单用法，底层是 shadcn Dialog。 */
export default function Dialog({
  title,
  description,
  children,
  footer,
  onClose,
}: {
  title: string
  description?: string
  children: ReactNode
  footer?: ReactNode
  onClose: () => void
}) {
  return (
    <DialogRoot
      open
      onOpenChange={(next) => {
        if (!next) onClose()
      }}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description ? <DialogDescription>{description}</DialogDescription> : null}
        </DialogHeader>
        <div className="space-y-4">{children}</div>
        {footer ? <DialogFooter>{footer}</DialogFooter> : null}
      </DialogContent>
    </DialogRoot>
  )
}
