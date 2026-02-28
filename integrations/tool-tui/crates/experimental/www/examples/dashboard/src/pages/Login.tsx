import { useState, useContext } from 'dx';
import { useNavigate } from 'dx/router';
import { AuthContext } from '../components/AuthProvider';

export function Login() {
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState('');
    const [isLoading, setIsLoading] = useState(false);

    const auth = useContext(AuthContext);
    const navigate = useNavigate();

    // Redirect if already logged in
    if (auth?.isAuthenticated) {
        navigate('/');
        return null;
    }

    const handleSubmit = async (e: Event) => {
        e.preventDefault();
        setError('');
        setIsLoading(true);

        try {
            const success = await auth?.login(email, password);
            if (success) {
                navigate('/');
            } else {
                setError('Invalid email or password');
            }
        } catch (err) {
            setError('An error occurred. Please try again.');
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <div class="login-page">
            <div class="login-card">
                <h1>Sign In</h1>
                <p class="subtitle">Welcome back! Please sign in to continue.</p>

                <form onSubmit={handleSubmit}>
                    {error && (
                        <div class="error-message" role="alert">
                            {error}
                        </div>
                    )}

                    <div class="form-group">
                        <label htmlFor="email">Email</label>
                        <input
                            id="email"
                            type="email"
                            value={email}
                            onChange={(e) => setEmail((e.target as HTMLInputElement).value)}
                            placeholder="you@example.com"
                            required
                            disabled={isLoading}
                        />
                    </div>

                    <div class="form-group">
                        <label htmlFor="password">Password</label>
                        <input
                            id="password"
                            type="password"
                            value={password}
                            onChange={(e) => setPassword((e.target as HTMLInputElement).value)}
                            placeholder="••••••••"
                            required
                            disabled={isLoading}
                        />
                    </div>

                    <button
                        type="submit"
                        class="submit-btn"
                        disabled={isLoading}
                    >
                        {isLoading ? 'Signing in...' : 'Sign In'}
                    </button>
                </form>

                <div class="demo-credentials">
                    <p>Demo credentials:</p>
                    <ul>
                        <li>Admin: admin@example.com / admin123</li>
                        <li>User: user@example.com / user123</li>
                    </ul>
                </div>
            </div>
        </div>
    );
}
