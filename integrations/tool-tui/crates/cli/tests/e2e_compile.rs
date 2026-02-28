//! # End-to-End Compilation Tests
//!
//! Tests the full compilation pipeline from TSX → Binary
//!
//! NOTE: Disabled until dx_compiler crate is available

#![cfg(feature = "disabled_until_compiler_available")]

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_compile_simple_counter() -> Result<()> {
    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    let entry = src_dir.join("App.tsx");
    fs::write(
        &entry,
        r#"
import { useState } from 'dx';

export default function Counter() {
  const [count, setCount] = useState(0);
  
  return (
    <div class="counter">
      <h1>Count: {count}</h1>
      <button onClick={() => setCount(count + 1)}>Increment</button>
    </div>
  );
}
    "#,
    )?;

    let output = temp.path().join("dist");

    // Compile
    let result = dx_compiler::compile_tsx(&entry, &output, false)?;

    // Verify outputs exist
    assert!(result.htip_path.exists(), "HTIP file should exist");
    assert!(result.templates_path.exists(), "Templates JSON should exist");

    // Verify it selected micro runtime (simple app)
    assert_eq!(
        result.runtime_variant,
        dx_compiler::analyzer::RuntimeVariant::Micro,
        "Should select Micro runtime for simple counter"
    );

    // Verify .dxb was created
    let dxb_path = output.join("app.dxb");
    assert!(dxb_path.exists(), "DXB artifact should exist");

    // Verify it's actually small
    let size = fs::metadata(&dxb_path)?.len();
    assert!(size < 2000, "DXB should be < 2KB (got {})", size);

    println!("✓ Simple counter compiled to {} bytes", size);

    Ok(())
}

#[tokio::test]
async fn test_compile_complex_dashboard() -> Result<()> {
    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    let entry = src_dir.join("App.tsx");
    fs::write(
        &entry,
        r#"
import { useState, useEffect } from 'dx';

interface Metric {
  label: string;
  value: number;
}

export default function Dashboard() {
  const [metrics, setMetrics] = useState<Metric[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState('');
  const [sortOrder, setSortOrder] = useState('asc');
  const [page, setPage] = useState(1);
  const [selectedMetric, setSelectedMetric] = useState<Metric | null>(null);
  
  useEffect(() => {
    fetch('/api/metrics')
      .then(r => r.json())
      .then(data => {
        setMetrics(data);
        setLoading(false);
      });
  }, [page, filter]);
  
  if (loading) return <div>Loading...</div>;
  
  return (
    <div class="dashboard">
      <header>
        <h1>Analytics Dashboard</h1>
        <input 
          type="text"
          value={filter}
          onChange={e => setFilter(e.target.value)}
          placeholder="Filter..."
        />
      </header>
      
      <div class="metrics">
        {metrics.map(m => (
          <div 
            key={m.label} 
            class="metric-card"
            onClick={() => setSelectedMetric(m)}
          >
            <span>{m.label}</span>
            <strong>{m.value}</strong>
          </div>
        ))}
      </div>
      
      <footer>
        <button onClick={() => setPage(page - 1)}>Previous</button>
        <button onClick={() => setPage(page + 1)}>Next</button>
        <button onClick={() => setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')}>
          Sort
        </button>
      </footer>
    </div>
  );
}
    "#,
    )?;

    let output = temp.path().join("dist");

    // Compile
    let result = dx_compiler::compile_tsx(&entry, &output, false)?;

    // Verify it selected macro runtime (complex app)
    assert_eq!(
        result.runtime_variant,
        dx_compiler::analyzer::RuntimeVariant::Macro,
        "Should select Macro runtime for complex dashboard"
    );

    // Verify metrics
    assert!(result.metrics.component_count >= 1, "Should have components");
    assert!(result.metrics.total_state_vars >= 5, "Should have multiple state vars");
    assert!(result.metrics.event_handler_count >= 3, "Should have event handlers");

    println!("✓ Complex dashboard compiled");
    println!("  Components: {}", result.metrics.component_count);
    println!("  State vars: {}", result.metrics.total_state_vars);
    println!("  Events: {}", result.metrics.event_handler_count);
    println!("  Runtime: {:?}", result.runtime_variant);

    Ok(())
}

#[tokio::test]
async fn test_compile_multiple_components() -> Result<()> {
    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    let entry = src_dir.join("App.tsx");
    fs::write(
        &entry,
        r#"
import { useState } from 'dx';

function Header() {
  return <header><h1>My App</h1></header>;
}

function Footer() {
  return <footer>© 2025</footer>;
}

function Sidebar() {
  const [collapsed, setCollapsed] = useState(false);
  return (
    <aside class={collapsed ? 'collapsed' : ''}>
      <button onClick={() => setCollapsed(!collapsed)}>Toggle</button>
    </aside>
  );
}

export default function App() {
  const [theme, setTheme] = useState('light');
  
  return (
    <div class={`app theme-${theme}`}>
      <Header />
      <Sidebar />
      <main>
        <button onClick={() => setTheme(theme === 'light' ? 'dark' : 'light')}>
          Toggle Theme
        </button>
      </main>
      <Footer />
    </div>
  );
}
    "#,
    )?;

    let output = temp.path().join("dist");
    let result = dx_compiler::compile_tsx(&entry, &output, false)?;

    // Should have multiple components
    assert!(result.metrics.component_count >= 3, "Should detect multiple components");

    println!("✓ Multi-component app compiled");
    println!("  Components detected: {}", result.metrics.component_count);

    Ok(())
}

#[tokio::test]
async fn test_error_on_invalid_tsx() {
    let temp = TempDir::new().unwrap();
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let entry = src_dir.join("App.tsx");
    fs::write(
        &entry,
        r#"
export default function App() {
  // Invalid TSX - unclosed tag
  return <div><h1>Broken;
}
    "#,
    )
    .unwrap();

    let output = temp.path().join("dist");
    let result = dx_compiler::compile_tsx(&entry, &output, false);

    // Should fail gracefully
    assert!(result.is_err(), "Should fail on invalid TSX");

    println!("✓ Invalid TSX rejected correctly");
}

#[tokio::test]
async fn test_compile_preserves_semantic_meaning() -> Result<()> {
    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    let entry = src_dir.join("App.tsx");
    fs::write(
        &entry,
        r#"
export default function App() {
  return (
    <div class="container">
      <h1 id="title">Hello World</h1>
      <p class="description">This is a test</p>
      <button data-testid="btn">Click Me</button>
    </div>
  );
}
    "#,
    )?;

    let output = temp.path().join("dist");
    dx_compiler::compile_tsx(&entry, &output, false)?;

    // Read templates JSON to verify structure preserved
    let templates_json = fs::read_to_string(output.join("templates.json"))?;

    // Verify semantic elements are preserved
    assert!(templates_json.contains("container"), "Should preserve class names");
    assert!(templates_json.contains("title"), "Should preserve IDs");
    assert!(templates_json.contains("btn"), "Should preserve data attributes");

    println!("✓ Semantic HTML preserved");

    Ok(())
}

#[tokio::test]
async fn test_incremental_compilation_speed() -> Result<()> {
    let temp = TempDir::new()?;
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&src_dir)?;

    let entry = src_dir.join("App.tsx");
    fs::write(
        &entry,
        r#"
export default function App() {
  return <div>Initial</div>;
}
    "#,
    )?;

    let output = temp.path().join("dist");

    // First compilation
    let start = std::time::Instant::now();
    dx_compiler::compile_tsx(&entry, &output, false)?;
    let first_time = start.elapsed();

    // Modify file
    fs::write(
        &entry,
        r#"
export default function App() {
  return <div>Modified</div>;
}
    "#,
    )?;

    // Second compilation (should be fast)
    let start = std::time::Instant::now();
    dx_compiler::compile_tsx(&entry, &output, false)?;
    let second_time = start.elapsed();

    println!("✓ Incremental compilation");
    println!("  First:  {:?}", first_time);
    println!("  Second: {:?}", second_time);

    // Second should be reasonably fast (not necessarily faster due to no caching yet)
    assert!(second_time < std::time::Duration::from_secs(1), "Should compile in under 1s");

    Ok(())
}
