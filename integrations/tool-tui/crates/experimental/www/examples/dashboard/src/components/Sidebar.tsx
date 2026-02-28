import { useContext } from 'dx';
import { Link, useLocation } from 'dx/router';
import { AuthContext } from './AuthProvider';

export function Sidebar() {
    const auth = useContext(AuthContext);
    const location = useLocation();

    if (!auth?.user) return null;

    const isActive = (path: string) => location.pathname === path;

    return (
        <aside class="sidebar">
            <div class="user-info">
                <div class="avatar">
                    {auth.user.name.charAt(0).toUpperCase()}
                </div>
                <div class="user-details">
                    <span class="name">{auth.user.name}</span>
                    <span class="role">{auth.user.role}</span>
                </div>
            </div>

            <nav class="sidebar-nav">
                <ul>
                    <li>
                        <Link
                            to="/"
                            class={isActive('/') ? 'active' : ''}
                        >
                            <span class="icon">📊</span>
                            Dashboard
                        </Link>
                    </li>
                    <li>
                        <Link
                            to="/settings"
                            class={isActive('/settings') ? 'active' : ''}
                        >
                            <span class="icon">⚙️</span>
                            Settings
                        </Link>
                    </li>
                    {auth.user.role === 'admin' && (
                        <li>
                            <Link
                                to="/admin"
                                class={isActive('/admin') ? 'active' : ''}
                            >
                                <span class="icon">🔐</span>
                                Admin
                            </Link>
                        </li>
                    )}
                </ul>
            </nav>

            <button class="logout-btn" onClick={auth.logout}>
                <span class="icon">🚪</span>
                Logout
            </button>
        </aside>
    );
}
