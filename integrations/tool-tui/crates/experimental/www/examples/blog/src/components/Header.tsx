import { Link, useLocation } from 'dx/router';

export function Header() {
    const location = useLocation();

    const isActive = (path: string) => {
        if (path === '/') {
            return location.pathname === '/';
        }
        return location.pathname.startsWith(path);
    };

    return (
        <header class="site-header">
            <nav class="nav">
                <Link to="/" class="logo">
                    dx-www Blog
                </Link>
                <ul class="nav-links">
                    <li>
                        <Link
                            to="/"
                            class={isActive('/') && location.pathname === '/' ? 'active' : ''}
                        >
                            Home
                        </Link>
                    </li>
                    <li>
                        <Link
                            to="/about"
                            class={isActive('/about') ? 'active' : ''}
                        >
                            About
                        </Link>
                    </li>
                </ul>
            </nav>
        </header>
    );
}
