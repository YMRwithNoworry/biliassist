import { NavLink, useNavigate } from 'react-router-dom'
import { useAuth } from '../state/auth'
import {
  IconChat,
  IconHome,
  IconLock,
  IconLogout,
  IconStar,
  IconUsers,
} from './icons'

const NAV_ITEMS = [
  { to: '/', label: '概览', Icon: IconHome, end: true },
  { to: '/login', label: '扫码登录', Icon: IconLock, end: false },
  { to: '/accounts', label: '账号管理', Icon: IconUsers, end: false },
  { to: '/auto-reply', label: '自动回复', Icon: IconChat, end: false },
  { to: '/sponsor', label: '支持项目', Icon: IconStar, end: false },
]

export default function Sidebar() {
  const { email, isPlus, logout } = useAuth()
  const navigate = useNavigate()

  const handleLogout = async () => {
    await logout()
    navigate('/auth', { replace: true })
  }

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <span className="brand-mark">
          <IconLock size={17} />
        </span>
        <span>B站账号管理</span>
      </div>

      <nav className="nav-list">
        {NAV_ITEMS.map(({ to, label, Icon, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            className={({ isActive }) => (isActive ? 'nav-item active' : 'nav-item')}
          >
            <span className="nav-item-icon">
              <Icon size={18} />
            </span>
            <span style={{ flex: 1 }}>{label}</span>
            {to === '/auto-reply' && !isPlus ? (
              <span className="badge badge-basic">Plus</span>
            ) : null}
          </NavLink>
        ))}
      </nav>

      <div className="sidebar-footer">
        <span className="sidebar-user" title={email}>
          {email || '未登录'}
        </span>
        <div className="sidebar-actions">
          <span className={isPlus ? 'badge badge-plus' : 'badge badge-basic'}>
            {isPlus ? 'Plus' : 'Basic'}
          </span>
          <button className="icon-btn" onClick={handleLogout} title="退出登录" aria-label="退出登录">
            <IconLogout size={16} />
          </button>
        </div>
      </div>
    </aside>
  )
}
