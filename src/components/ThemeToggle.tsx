import { useTheme } from '../state/theme'
import { IconMoon, IconSun } from './icons'

/** 只切换日间/夜间；图标展示的是「点下去会变成什么」。 */
export default function ThemeToggle({ className = '' }: { className?: string }) {
  const { theme, toggle } = useTheme()
  const dark = theme === 'dark'
  const label = dark ? '切换到日间模式' : '切换到夜间模式'

  return (
    <button
      type="button"
      className={`icon-btn theme-toggle ${className}`.trim()}
      onClick={toggle}
      title={label}
      aria-label={label}
      aria-pressed={dark}
    >
      {dark ? <IconSun size={16} /> : <IconMoon size={16} />}
    </button>
  )
}
