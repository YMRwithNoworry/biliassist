import { Moon, Sun } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useTheme } from '@/state/theme'
import { cn } from '@/lib/utils'

/** 只切换日间/夜间；图标展示的是「点下去会变成什么」。 */
export default function ThemeToggle({
  className,
  variant = 'ghost',
}: {
  className?: string
  variant?: 'ghost' | 'outline' | 'secondary'
}) {
  const { theme, toggle } = useTheme()
  const dark = theme === 'dark'
  const label = dark ? '切换到日间模式' : '切换到夜间模式'

  return (
    <Button
      type="button"
      variant={variant}
      size="icon-sm"
      className={cn('text-muted-foreground', className)}
      onClick={toggle}
      title={label}
      aria-label={label}
      aria-pressed={dark}
    >
      {dark ? <Sun /> : <Moon />}
    </Button>
  )
}
