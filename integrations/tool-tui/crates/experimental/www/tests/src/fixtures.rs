//! Test fixtures for integration tests
//!
//! Contains sample TSX applications used for testing the full compilation pipeline.

/// Minimal Hello World TSX component
pub const HELLO_WORLD_TSX: &str = r#"
export function HelloWorld() {
    return <div>Hello, World!</div>;
}
"#;

/// Counter app with state management
pub const COUNTER_APP_TSX: &str = r#"
import { useState } from 'dx';

export function Counter() {
    const [count, setCount] = useState(0);
    
    return (
        <div class="counter">
            <h1>Count: {count}</h1>
            <button onClick={() => setCount(count + 1)}>Increment</button>
            <button onClick={() => setCount(count - 1)}>Decrement</button>
        </div>
    );
}
"#;

/// Form with validation
pub const FORM_VALIDATION_TSX: &str = r#"
import { useState } from 'dx';

interface FormData {
    email: string;
    password: string;
}

export function LoginForm() {
    const [form, setForm] = useState<FormData>({ email: '', password: '' });
    const [errors, setErrors] = useState<Record<string, string>>({});
    
    const validate = () => {
        const newErrors: Record<string, string> = {};
        if (!form.email.includes('@')) {
            newErrors.email = 'Invalid email address';
        }
        if (form.password.length < 8) {
            newErrors.password = 'Password must be at least 8 characters';
        }
        setErrors(newErrors);
        return Object.keys(newErrors).length === 0;
    };
    
    const handleSubmit = (e: Event) => {
        e.preventDefault();
        if (validate()) {
            console.log('Form submitted:', form);
        }
    };
    
    return (
        <form onSubmit={handleSubmit}>
            <div>
                <label htmlFor="email">Email</label>
                <input
                    id="email"
                    type="email"
                    value={form.email}
                    onChange={(e) => setForm({ ...form, email: e.target.value })}
                />
                {errors.email && <span class="error">{errors.email}</span>}
            </div>
            <div>
                <label htmlFor="password">Password</label>
                <input
                    id="password"
                    type="password"
                    value={form.password}
                    onChange={(e) => setForm({ ...form, password: e.target.value })}
                />
                {errors.password && <span class="error">{errors.password}</span>}
            </div>
            <button type="submit">Login</button>
        </form>
    );
}
"#;

/// Multi-page routing example
pub const ROUTING_TSX: &str = r#"
import { Router, Route, Link } from 'dx/router';

function Home() {
    return (
        <div>
            <h1>Home</h1>
            <p>Welcome to the home page</p>
            <Link to="/about">Go to About</Link>
        </div>
    );
}

function About() {
    return (
        <div>
            <h1>About</h1>
            <p>This is the about page</p>
            <Link to="/">Go to Home</Link>
        </div>
    );
}

export function App() {
    return (
        <Router>
            <nav>
                <Link to="/">Home</Link>
                <Link to="/about">About</Link>
            </nav>
            <Route path="/" component={Home} />
            <Route path="/about" component={About} />
        </Router>
    );
}
"#;

/// SSR hydration example
pub const SSR_HYDRATION_TSX: &str = r#"
import { useState, useEffect } from 'dx';
import { hydrate } from 'dx/hydration';

interface Props {
    initialCount: number;
}

export function HydratableCounter({ initialCount }: Props) {
    const [count, setCount] = useState(initialCount);
    const [isClient, setIsClient] = useState(false);
    
    useEffect(() => {
        setIsClient(true);
    }, []);
    
    return (
        <div class="hydratable-counter" data-hydrate="true">
            <h1>Count: {count}</h1>
            <p>{isClient ? 'Client-side rendered' : 'Server-side rendered'}</p>
            <button onClick={() => setCount(count + 1)}>Increment</button>
        </div>
    );
}

// Server-side render function
export function renderToString(props: Props): string {
    return `<div class="hydratable-counter" data-hydrate="true">
        <h1>Count: ${props.initialCount}</h1>
        <p>Server-side rendered</p>
        <button>Increment</button>
    </div>`;
}
"#;

/// Expected HTML output for Hello World
pub const HELLO_WORLD_EXPECTED_HTML: &str = "<div>Hello, World!</div>";

/// Expected HTML output for Counter (initial state)
pub const COUNTER_EXPECTED_HTML: &str = r#"<div class="counter"><h1>Count: 0</h1><button>Increment</button><button>Decrement</button></div>"#;
