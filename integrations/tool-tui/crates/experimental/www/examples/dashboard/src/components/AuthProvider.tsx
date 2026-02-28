import { useState, useEffect, createContext } from 'dx';

export interface User {
    id: string;
    email: string;
    name: string;
    role: 'user' | 'admin';
}

export interface AuthContextType {
    user: User | null;
    isLoading: boolean;
    login: (email: string, password: string) => Promise<boolean>;
    logout: () => void;
    isAuthenticated: boolean;
}

export const AuthContext = createContext<AuthContextType | null>(null);

const TOKEN_KEY = 'dx-auth-token';

// Mock users for demo
const MOCK_USERS: Record<string, { password: string; user: User }> = {
    'admin@example.com': {
        password: 'admin123',
        user: { id: '1', email: 'admin@example.com', name: 'Admin User', role: 'admin' },
    },
    'user@example.com': {
        password: 'user123',
        user: { id: '2', email: 'user@example.com', name: 'Regular User', role: 'user' },
    },
};

interface AuthProviderProps {
    children: any;
}

export function AuthProvider({ children }: AuthProviderProps) {
    const [user, setUser] = useState<User | null>(null);
    const [isLoading, setIsLoading] = useState(true);

    // Check for existing session on mount
    useEffect(() => {
        const token = localStorage.getItem(TOKEN_KEY);
        if (token) {
            try {
                // In a real app, validate token with server
                const decoded = JSON.parse(atob(token));
                setUser(decoded);
            } catch (e) {
                localStorage.removeItem(TOKEN_KEY);
            }
        }
        setIsLoading(false);
    }, []);

    const login = async (email: string, password: string): Promise<boolean> => {
        // Simulate API call
        await new Promise(resolve => setTimeout(resolve, 500));

        const mockUser = MOCK_USERS[email];
        if (mockUser && mockUser.password === password) {
            const token = btoa(JSON.stringify(mockUser.user));
            localStorage.setItem(TOKEN_KEY, token);
            setUser(mockUser.user);
            return true;
        }
        return false;
    };

    const logout = () => {
        localStorage.removeItem(TOKEN_KEY);
        setUser(null);
    };

    const value: AuthContextType = {
        user,
        isLoading,
        login,
        logout,
        isAuthenticated: !!user,
    };

    return (
        <AuthContext.Provider value={value}>
            {children}
        </AuthContext.Provider>
    );
}
