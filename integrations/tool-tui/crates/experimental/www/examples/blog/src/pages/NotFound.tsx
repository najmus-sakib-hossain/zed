import { Link } from 'dx/router';

export function NotFound() {
    return (
        <div class="not-found">
            <h1>404</h1>
            <h2>Page Not Found</h2>
            <p>The page you're looking for doesn't exist or has been moved.</p>
            <Link to="/" class="home-link">Go to Home</Link>
        </div>
    );
}
