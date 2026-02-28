export interface Post {
    slug: string;
    title: string;
    excerpt: string;
    content: string;
    date: string;
    author: string;
    tags?: string[];
}

export const posts: Post[] = [
    {
        slug: 'getting-started-with-dx-www',
        title: 'Getting Started with dx-www',
        excerpt: 'Learn how to build your first application with dx-www, the high-performance web framework.',
        content: `dx-www is a revolutionary web framework that compiles TSX to optimized binary format. In this post, we'll walk through setting up your first project.

First, install the dx-www CLI using cargo:

cargo install dx-www-cli

Then create a new project:

dx init my-app
cd my-app

Your project structure will look like this:

my-app/
├── src/
│   └── App.tsx
├── public/
│   └── index.html
└── dx.config.json

Now you can start the development server:

dx dev

Open http://localhost:3000 to see your app running!`,
        date: '2026-01-08',
        author: 'DX Team',
        tags: ['tutorial', 'getting-started', 'dx-www'],
    },
    {
        slug: 'understanding-htip-protocol',
        title: 'Understanding the HTIP Protocol',
        excerpt: 'A deep dive into the Hyper Text Interchange Protocol that powers dx-www.',
        content: `HTIP (Hyper Text Interchange Protocol) is the binary format that makes dx-www so fast. Instead of sending HTML and JavaScript, dx-www sends a compact binary stream.

The HTIP format consists of:

1. Header (8 bytes): Magic bytes, version, and flags
2. Template Definitions: Pre-compiled HTML templates
3. Instructions: Operations to build and update the DOM

This approach has several advantages:

- Smaller payload size (typically 50-70% smaller than HTML)
- Faster parsing (binary is faster than text)
- Efficient updates (delta patching)

The client runtime is only 338 bytes (Brotli compressed) for simple apps, making initial load incredibly fast.`,
        date: '2026-01-05',
        author: 'DX Team',
        tags: ['architecture', 'htip', 'performance'],
    },
    {
        slug: 'server-side-rendering-guide',
        title: 'Server-Side Rendering with dx-www',
        excerpt: 'How to implement SSR for better SEO and initial load performance.',
        content: `Server-side rendering (SSR) is essential for SEO and fast initial page loads. dx-www makes SSR simple and efficient.

To enable SSR, update your dx.config.json:

{
    "entry": "src/App.tsx",
    "output": "dist",
    "features": {
        "ssr": true,
        "hydration": true
    }
}

Then create a server entry point:

import { createServer } from 'dx/server';

const server = createServer({
    entry: './dist/app.htip',
    ssr: true,
});

server.listen(3000);

The server will render your components to HTML on the first request, then hydrate on the client for interactivity.

dx-www's SSR is unique because it streams the HTIP binary format directly, avoiding the overhead of HTML serialization.`,
        date: '2026-01-02',
        author: 'DX Team',
        tags: ['ssr', 'seo', 'performance'],
    },
];
