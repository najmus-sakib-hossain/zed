//! Embedded project templates

/// Counter template - Simple counter example (recommended for beginners)
pub const COUNTER: &str = r#"import { useState } from 'dx';

export default function App() {
  const [count, setCount] = useState(0);

  return (
    <div class="min-h-screen bg-gray-900 text-white flex items-center justify-center">
      <div class="text-center">
        <h1 class="text-4xl font-bold mb-8">DX Counter</h1>
        <p class="text-6xl font-mono mb-8">{count}</p>
        <div class="space-x-4">
          <button
            onClick={() => setCount(count - 1)}
            class="px-6 py-3 bg-red-600 rounded-lg hover:bg-red-700"
          >
            -
          </button>
          <button
            onClick={() => setCount(count + 1)}
            class="px-6 py-3 bg-green-600 rounded-lg hover:bg-green-700"
          >
            +
          </button>
        </div>
      </div>
    </div>
  );
}
"#;

/// Minimal template - Bare minimum setup
pub const MINIMAL: &str = r#"import { mount } from 'dx';

function App() {
  return <h1>Hello, DX!</h1>;
}

mount(<App />, document.getElementById('root'));
"#;

/// Dashboard template - SaaS dashboard with charts and tables
#[allow(dead_code)]
pub const DASHBOARD: &str = r#"import { useState, useEffect } from 'dx';

interface DataPoint {
  name: string;
  value: number;
}

export default function Dashboard() {
  const [data, setData] = useState<DataPoint[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Simulate data fetch
    setTimeout(() => {
      setData([
        { name: 'Users', value: 1234 },
        { name: 'Revenue', value: 45678 },
        { name: 'Orders', value: 890 },
      ]);
      setLoading(false);
    }, 500);
  }, []);

  if (loading) {
    return <div class="flex items-center justify-center h-screen">Loading...</div>;
  }

  return (
    <div class="min-h-screen bg-gray-100 p-8">
      <h1 class="text-3xl font-bold mb-8">Dashboard</h1>
      <div class="grid grid-cols-3 gap-6">
        {data.map(item => (
          <div class="bg-white rounded-lg shadow p-6">
            <h2 class="text-gray-500 text-sm">{item.name}</h2>
            <p class="text-3xl font-bold">{item.value.toLocaleString()}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
"#;

/// API template - API server only (no frontend)
#[allow(dead_code)]
pub const API: &str = r#"import { createServer, json } from 'dx/server';

const server = createServer({
  port: 3000,
});

server.get('/api/health', (req, res) => {
  res.json({ status: 'ok', timestamp: Date.now() });
});

server.get('/api/users', async (req, res) => {
  // Example API endpoint
  res.json([
    { id: 1, name: 'Alice' },
    { id: 2, name: 'Bob' },
  ]);
});

server.post('/api/users', async (req, res) => {
  const body = await json(req);
  res.json({ created: true, user: body });
});

console.log('Server running at http://localhost:3000');
"#;

/// Hackernews template - Hacker News clone (real-world example)
#[allow(dead_code)]
pub const HACKERNEWS: &str = r#"import { useState, useEffect } from 'dx';

interface Story {
  id: number;
  title: string;
  url: string;
  score: number;
  by: string;
  time: number;
}

export default function HackerNews() {
  const [stories, setStories] = useState<Story[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchStories() {
      const res = await fetch('https://hacker-news.firebaseio.com/v0/topstories.json');
      const ids = await res.json();
      
      const storyPromises = ids.slice(0, 30).map(async (id: number) => {
        const storyRes = await fetch(`https://hacker-news.firebaseio.com/v0/item/${id}.json`);
        return storyRes.json();
      });
      
      const stories = await Promise.all(storyPromises);
      setStories(stories);
      setLoading(false);
    }
    
    fetchStories();
  }, []);

  if (loading) {
    return (
      <div class="min-h-screen bg-orange-50 flex items-center justify-center">
        <div class="text-orange-600">Loading stories...</div>
      </div>
    );
  }

  return (
    <div class="min-h-screen bg-orange-50">
      <header class="bg-orange-500 text-white p-4">
        <h1 class="text-xl font-bold">Hacker News (DX Clone)</h1>
      </header>
      <main class="max-w-4xl mx-auto p-4">
        <ol class="space-y-2">
          {stories.map((story, index) => (
            <li class="bg-white p-4 rounded shadow">
              <span class="text-gray-500 mr-2">{index + 1}.</span>
              <a href={story.url} class="text-blue-600 hover:underline">
                {story.title}
              </a>
              <div class="text-sm text-gray-500 mt-1">
                {story.score} points by {story.by}
              </div>
            </li>
          ))}
        </ol>
      </main>
    </div>
  );
}
"#;
