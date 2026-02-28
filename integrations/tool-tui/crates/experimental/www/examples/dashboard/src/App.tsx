import { Router, Route } from 'dx/router';
import { AuthProvider } from './components/AuthProvider';
import { ProtectedRoute } from './components/ProtectedRoute';
import { Login } from './pages/Login';
import { Dashboard } from './pages/Dashboard';
import { Settings } from './pages/Settings';
import { Admin } from './pages/Admin';

export default function App() {
    return (
        <AuthProvider>
            <Router>
                <Route path="/login" component={Login} />
                <ProtectedRoute path="/" component={Dashboard} />
                <ProtectedRoute path="/settings" component={Settings} />
                <ProtectedRoute path="/admin" component={Admin} requiredRole="admin" />
            </Router>
        </AuthProvider>
    );
}
