import { Home, LogOut, MessageSquare, QrCode, Star, Users } from 'lucide-react'
import { NavLink, useNavigate } from 'react-router-dom'
import ThemeToggle from '@/components/ThemeToggle'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useAuth } from '@/state/auth'
import { cn } from '@/lib/utils'

const NAV_ITEMS = [
  { to: '/', label: '概览', Icon: Home, end: true },
  { to: '/login', label: '扫码登录', Icon: QrCode, end: false },
  { to: '/accounts', label: '账号管理', Icon: Users, end: false },
  { to: '/auto-reply', label: '自动回复', Icon: MessageSquare, end: false },
  { to: '/sponsor', label: '支持项目', Icon: Star, end: false },
]

export default function Sidebar() {
  const { email, isPlus, logout } = useAuth()
  const navigate = useNavigate()

  const handleLogout = async () => {
    await logout()
    navigate('/auth', { replace: true })
  }

  return (
    <aside className="flex w-64 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <div className="flex items-center gap-2.5 px-5 py-5">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-[10px] bg-brand text-brand-foreground">
          <QrCode className="size-4" />
        </span>
        <span className="text-[15px] font-semibold">B站账号管理</span>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3">
        {NAV_ITEMS.map(({ to, label, Icon, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            className={({ isActive }) =>
              cn(
                'flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
                isActive
                  ? 'bg-sidebar-accent text-foreground'
                  : 'text-muted-foreground hover:bg-sidebar-accent hover:text-foreground',
              )
            }
          >
            <Icon className="size-4 shrink-0" />
            <span className="flex-1">{label}</span>
            {to === '/auto-reply' && !isPlus ? (
              <Badge variant="outline" className="border-warning/30 bg-warning-tint text-warning">
                Plus
              </Badge>
            ) : null}
          </NavLink>
        ))}
      </nav>

      <div className="mt-auto space-y-3 border-t border-sidebar-border px-5 py-4">
        <p className="truncate text-xs text-muted-foreground" title={email}>
          {email || '未登录'}
        </p>
        <div className="flex items-center gap-2">
          {isPlus ? (
            <Badge variant="outline" className="border-success/30 bg-success-tint text-success">
              Plus
            </Badge>
          ) : (
            <Badge variant="secondary">Basic</Badge>
          )}
          <div className="ml-auto flex items-center gap-1">
            <ThemeToggle />
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-muted-foreground"
              onClick={handleLogout}
              title="退出登录"
              aria-label="退出登录"
            >
              <LogOut />
            </Button>
          </div>
        </div>
      </div>
    </aside>
  )
}
