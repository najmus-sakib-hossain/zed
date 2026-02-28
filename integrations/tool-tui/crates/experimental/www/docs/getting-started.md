
# Getting Started with dx-www

This guide will help you get up and running with dx-www, a high-performance web framework that compiles TSX to optimized binary format.

## Prerequisites

- Rust 1.85 or later
- Node.js 18+ (for development tooling)
- A modern web browser

## Installation

### From crates.io

```bash
cargo install dx-www-cli ```


### From Source


```bash
git clone https://github.com/dx-www/dx-www.git cd dx-www cargo build --release ```

## Quick Start

### 1. Create a New Project

```bash
mkdir my-app cd my-app dx init ```
This creates a basic project structure: @tree:my-app[]


### 2. Write Your First Component


Edit `src/App.tsx`:
```tsx
export default function App() { return ( <div class="app"> <h1>Hello, dx-www!</h1> <p>Welcome to the future of web development.</p> </div> );
}
```


### 3. Build Your Application


```bash
dx build ```
This compiles your TSX to optimized HTIP binary format: @tree:dist[]

### 4. Run the Development Server

```bash
dx dev ```
Open //localhost:3000 to see your app.


## Core Concepts



### Components


dx-www uses TSX (TypeScript JSX) for component definitions:
```tsx
interface GreetingProps { name: string;
}
export function Greeting({ name }: GreetingProps) { return <h1>Hello, {name}!</h1>;
}
```


### State Management


Use the `useState` hook for reactive state:
```tsx
import { useState } from 'dx';
export function Counter() { const [count, setCount] = useState(0);
return ( <div> <p>Count: {count}</p> <button onClick={() => setCount(count + 1)}> Increment </button> </div> );
}
```


### Event Handling


Events are handled with standard JSX syntax:
```tsx
export function Button() { const handleClick = (e: Event) => { console.log('Button clicked!');
};
return <button onClick={handleClick}>Click me</button>;
}
```


### Routing


Use the built-in router for multi-page applications:
```tsx
import { Router, Route, Link } from 'dx/router';
export function App() { return ( <Router> <nav> <Link to="/">Home</Link> <Link to="/about">About</Link> </nav> <Route path="/" component={Home} /> <Route path="/about" component={About} /> </Router> );
}
```


## Configuration



### dx.config.json


```json
{ "entry": "src/App.tsx", "output": "dist", "runtime": "auto", "features": { "ssr": true, "hydration": true, "treeShaking": true }
}
```


### Runtime Selection


dx-www automatically selects the optimal runtime: -Micro Runtime (338B): For simple, static components -Macro Runtime (7.5KB): For complex, interactive applications You can override this with `"runtime": "micro"` or `"runtime": "macro"`.


## Server-Side Rendering


Enable SSR for better SEO and initial load performance:
```tsx
// src/App.tsx export default function App() { return <div>Server-rendered content</div>;
}
// Server configuration import { createServer } from 'dx/server';
const server = createServer({ entry: './dist/app.htip', ssr: true, });
server.listen(3000);
```


## Production Deployment



### Build for Production


```bash
dx build --release ```
This enables: -Tree shaking (removes unused code) -Minification -Delta patching (efficient updates)

### Deploy

The `dist/` folder contains everything needed for deployment:
```bash


# Deploy to any static host


cp -r dist/* /var/www/html/


# Or use the built-in server


dx serve --port 8080 ```


## Next Steps


- API Documentation (./api/README.md)
- Examples (./examples/README.md)
- Architecture Guide (./architecture.md)
- Contributing (../CONTRIBUTING.md)


## Getting Help


- GitHub Issues
- Discord Community
- Stack Overflow
