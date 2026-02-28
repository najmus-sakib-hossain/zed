import { Header } from './Header';

interface LayoutProps {
    children: any;
}

export function Layout({ children }: LayoutProps) {
    return (
        <div class="layout">
            <Header />
            <main class="main-content">
                {children}
            </main>
            <footer class="site-footer">
                <p>© 2026 dx-www Blog. Built with dx-www.</p>
            </footer>
        </div>
    );
}
