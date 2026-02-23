import { useApp } from '../context/AppContext';

interface NavItemProps {
  icon: string;
  label: string;
  active?: boolean;
  badge?: boolean;
  onClick: () => void;
}

function NavItem({ icon, label, active, badge, onClick }: NavItemProps) {
  return (
    <div className={`nav-item tooltip ${active ? 'active' : ''}`} data-tooltip={label} onClick={onClick}>
      <i className={`fas ${icon}`}></i>
      {badge && <span className="badge"></span>}
    </div>
  );
}

interface NavigationPanelProps {
  onSettingsClick?: () => void;
}

export function NavigationPanel({ onSettingsClick }: NavigationPanelProps) {
  const { state, dispatch } = useApp();

  const navItems = [
    { icon: 'fa-comments', label: 'Sessions', key: 'sessions' },
    { icon: 'fa-search', label: 'Search', key: 'search' },
    { icon: 'fa-brain', label: 'Memory', key: 'memory' },
    { icon: 'fa-folder', label: 'Files', key: 'files' },
  ];

  const bottomItems = [
    { icon: 'fa-chart-line', label: 'Analytics', key: 'analytics' },
    { icon: 'fa-history', label: 'Operations', key: 'operations' },
    { icon: 'fa-code-branch', label: 'Branches', key: 'branches' },
    { icon: 'fa-bell', label: 'Notifications', key: 'notifications', badge: false },
    { icon: 'fa-cog', label: 'Settings', key: 'settings' },
  ];

  return (
    <nav className="nav-panel">
      <div className="nav-logo">MS</div>

      {navItems.map((item) => (
        <NavItem
          key={item.key}
          icon={item.icon}
          label={item.label}
          active={state.activeNav === item.key}
          onClick={() => dispatch({ type: 'SET_ACTIVE_NAV', payload: item.key })}
        />
      ))}

      <div className="nav-divider"></div>

      {bottomItems.slice(0, 3).map((item) => (
        <NavItem
          key={item.key}
          icon={item.icon}
          label={item.label}
          active={state.activeNav === item.key}
          badge={item.badge}
          onClick={() => dispatch({ type: 'SET_ACTIVE_NAV', payload: item.key })}
        />
      ))}

      <div className="nav-spacer"></div>

      {bottomItems.slice(3).map((item) => (
        <NavItem
          key={item.key}
          icon={item.icon}
          label={item.label}
          active={state.activeNav === item.key}
          badge={item.badge}
          onClick={() => {
            dispatch({ type: 'SET_ACTIVE_NAV', payload: item.key });
            if (item.key === 'settings' && onSettingsClick) {
              onSettingsClick();
            }
          }}
        />
      ))}
    </nav>
  );
}
