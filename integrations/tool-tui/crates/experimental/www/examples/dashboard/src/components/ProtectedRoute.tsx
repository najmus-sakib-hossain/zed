import { useContext } from 'dx';
import { Navigate, Route } from 'dx/router';
import { AuthContext } from './AuthProvider';

interface ProtectedRouteProps {
    path: string;
    component: () => JSX.Element;
    requiredRole?: 'user' | 'admin';
}

export function ProtectedRoute({ path, component: Component, requiredRole }: ProtectedRouteProps) {
    const auth = useContext(AuthContext);

    if (!auth) {
        return null;
    }

    if (auth.isLoading) {
        return (
            <div class="loading">
                <p>Loading...</p>
            </div>
        );
    }

    if (!auth.isAuthenticated) {
        return <Navigate to="/login" />;
    }

    if (requiredRole && auth.user?.role !== requiredRole) {
        return (
            <div class="unauthorized">
                <h1>Unauthorized</h1>
                <p>You don't have permission to access this page.</p>
            </div>
        );
    }

    return <Route path={path} component={Component} />;
}
