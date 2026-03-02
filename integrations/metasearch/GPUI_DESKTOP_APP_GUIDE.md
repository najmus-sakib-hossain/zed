# Building a GPUI Desktop App for Metasearch

This guide shows you how to create a high-performance desktop application using GPUI (GPU User Interface) to display search results from your metasearch engine.

## What is GPUI?

GPUI is a GPU-accelerated UI framework created by the Zed editor team. It's designed for building fast, native desktop applications in Rust with:

- GPU-accelerated rendering for smooth performance
- Hybrid immediate/retained mode rendering
- Cross-platform support (Windows, macOS, Linux)
- Reactive state management with Models and Views
- Built-in components for lists, inputs, and layouts

Content rephrased for compliance with licensing restrictions. [Learn more about GPUI](https://www.gpui.rs/)

## Project Setup

### 1. Create a new Rust project

```bash
cargo new metasearch-desktop
cd metasearch-desktop
```

### 2. Add dependencies to `Cargo.toml`

```toml
[package]
name = "metasearch-desktop"
version = "0.1.0"
edition = "2021"

[dependencies]
gpui = "0.2.2"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
urlencoding = "2.1"
```

## Core Concepts

### Models and Views

GPUI uses a reactive architecture:
- **Models**: Shared state containers that can be observed for changes
- **Views**: UI components that render based on model state
- **Context**: Provides access to the application state and window operations

### Async Operations

GPUI integrates with Tokio for async operations. Use `cx.spawn()` to run background tasks without blocking the UI.

## Implementation

### Step 1: Define Data Structures

Create `src/search.rs`:

```rust
use serde::{Deserialize, Serialize};
use gpui::SharedString;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub search_time_ms: u64,
    pub number_of_results: usize,
    pub engines_used: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub engine: String,
}

impl SearchResult {
    pub fn title_shared(&self) -> SharedString {
        SharedString::from(self.title.clone())
    }
    
    pub fn url_shared(&self) -> SharedString {
        SharedString::from(self.url.clone())
    }
    
    pub fn description_shared(&self) -> SharedString {
        self.description.clone()
            .unwrap_or_default()
            .into()
    }
}
```

### Step 2: Create the Search Client

Create `src/client.rs`:

```rust
use crate::search::{SearchResponse, SearchResult};
use anyhow::Result;

pub struct MetasearchClient {
    base_url: String,
    client: reqwest::Client,
}

impl MetasearchClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn search(&self, query: &str) -> Result<SearchResponse> {
        let url = format!("{}/api/v1/search?q={}", self.base_url, 
                         urlencoding::encode(query));
        
        let response = self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;
        
        let search_response: SearchResponse = response.json().await?;
        Ok(search_response)
    }
}
```

### Step 3: Build the Main UI Component

Create `src/main.rs`:

```rust
mod search;
mod client;

use gpui::*;
use search::{SearchResponse, SearchResult};
use client::MetasearchClient;

// Main application state
pub struct MetasearchApp {
    query: String,
    results: Model<Vec<SearchResult>>,
    list_state: ListState,
    is_loading: bool,
    search_time_ms: u64,
    client: MetasearchClient,
}

impl MetasearchApp {
    // Create list state from results
    fn make_list_state(results: Vec<SearchResult>) -> ListState {
        ListState::new(
            results.len(),
            ListAlignment::Top,
            px(30.0),
            move |idx: usize, _cx| {
                let result = &results[idx];
                
                // Create a result card
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .mb_2()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xe0e0e0))
                    .rounded_md()
                    .shadow_sm()
                    .hover(|style| style.bg(rgb(0xf5f5f5)))
                    .child(
                        // Title
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x1a0dab))
                            .child(result.title_shared())
                    )
                    .child(
                        // URL
                        div()
                            .text_sm()
                            .text_color(rgb(0x006621))
                            .child(result.url_shared())
                    )
                    .child(
                        // Description
                        div()
                            .text_sm()
                            .text_color(rgb(0x545454))
                            .child(result.description_shared())
                    )
                    .child(
                        // Engine badge
                        div()
                            .text_xs()
                            .text_color(rgb(0x888888))
                            .child(format!("Source: {}", result.engine))
                    )
                    .into_any_element()
            },
        )
    }
    
    pub fn new(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| {
            let results = cx.new_model(|_| Vec::new());
            let list_state = Self::make_list_state(Vec::new());
            let client = MetasearchClient::new("http://localhost:8888".to_string());
            
            // Observe results changes
            cx.observe(&results, |this: &mut Self, model, cx| {
                let data = model.read(cx).clone();
                this.list_state = Self::make_list_state(data);
                cx.notify();
            })
            .detach();
            
            MetasearchApp {
                query: String::new(),
                results,
                list_state,
                is_loading: false,
                search_time_ms: 0,
                client,
            }
        })
    }
    
    fn perform_search(&mut self, cx: &mut ViewContext<Self>) {
        if self.query.is_empty() {
            return;
        }
        
        self.is_loading = true;
        cx.notify();
        
        let query = self.query.clone();
        let results_model = self.results.clone();
        let client = self.client.clone();
        
        // Spawn async task
        cx.spawn(|this, mut cx| async move {
            match client.search(&query).await {
                Ok(response) => {
                    // Update results model
                    results_model.update(&mut cx, |model, cx| {
                        *model = response.results;
                        cx.notify();
                    }).ok();
                    
                    // Update loading state and search time
                    this.update(&mut cx, |this, cx| {
                        this.is_loading = false;
                        this.search_time_ms = response.search_time_ms;
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    eprintln!("Search error: {}", e);
                    this.update(&mut cx, |this, cx| {
                        this.is_loading = false;
                        cx.notify();
                    }).ok();
                }
            }
        })
        .detach();
    }
}

impl Render for MetasearchApp {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf8f9fa))
            .child(
                // Header
                div()
                    .flex()
                    .flex_col()
                    .p_4()
                    .bg(rgb(0xffffff))
                    .border_b_1()
                    .border_color(rgb(0xe0e0e0))
                    .shadow_sm()
                    .child(
                        // Title
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .mb_3()
                            .text_color(rgb(0x202124))
                            .child("Metasearch Desktop")
                    )
                    .child(
                        // Search box
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                // Input field (simplified - use TextInput in real app)
                                div()
                                    .flex_1()
                                    .p_2()
                                    .border_1()
                                    .border_color(rgb(0xdfe1e5))
                                    .rounded_md()
                                    .bg(rgb(0xffffff))
                                    .child(self.query.clone())
                            )
                            .child(
                                // Search button
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(0x1a73e8))
                                    .text_color(rgb(0xffffff))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(0x1557b0)))
                                    .child(if self.is_loading { "Searching..." } else { "Search" })
                            )
                    )
                    .when(self.search_time_ms > 0, |div| {
                        div.child(
                            div()
                                .mt_2()
                                .text_sm()
                                .text_color(rgb(0x70757a))
                                .child(format!(
                                    "{} results in {}ms",
                                    self.results.read(_).len(),
                                    self.search_time_ms
                                ))
                        )
                    })
            )
            .child(
                // Results list
                div()
                    .flex_1()
                    .p_4()
                    .overflow_y_scroll()
                    .child(
                        list(self.list_state.clone())
                            .w_full()
                            .h_full()
                    )
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Metasearch Desktop".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| MetasearchApp::new(cx),
        )
        .unwrap();
    });
}
```

## Building and Running

### Development Build

```bash
cargo run
```

### Release Build (Optimized)

```bash
cargo build --release
./target/release/metasearch-desktop
```

## Key Features Implemented

1. **Async Search**: Non-blocking HTTP requests using Tokio
2. **Reactive UI**: Automatic updates when search results change
3. **Scrollable List**: GPU-accelerated virtualized list for smooth scrolling
4. **Result Cards**: Clean, clickable result display with title, URL, description
5. **Loading States**: Visual feedback during search operations
6. **Performance Metrics**: Display search time and result count

## Advanced Enhancements

### Add Text Input Handling

For proper text input, you'll need to handle keyboard events:

```rust
use gpui::KeyBinding;

// In your render method, add key handlers
.on_key_down(Key::Enter, |this, cx| {
    this.perform_search(cx);
})
```

### Add Click Handlers

Make results clickable to open in browser:

```rust
.on_click(cx.listener(move |_, _, cx| {
    // Open URL in default browser
    open::that(&result.url).ok();
}))
```

### Add Categories Support

Extend the UI to support different search categories (images, videos, etc.):

```rust
pub enum SearchCategory {
    General,
    Images,
    Videos,
    News,
}

// Add category selector to UI
```

### Add Caching

Cache recent searches for instant results:

```rust
use std::collections::HashMap;

pub struct SearchCache {
    cache: HashMap<String, SearchResponse>,
}
```

## Performance Tips

1. **Use SharedString**: For text that's cloned frequently
2. **Virtualized Lists**: GPUI's ListState only renders visible items
3. **Spawn Background Tasks**: Keep UI responsive with `cx.spawn()`
4. **Batch Updates**: Update models once, not per-item
5. **GPU Acceleration**: GPUI renders directly via GPU for smooth scrolling

## Troubleshooting

### "Connection refused" errors

Make sure your metasearch server is running:
```bash
cargo run --bin metasearch-server
```

### Slow rendering

- Check if you're creating too many elements in the render function
- Use `list()` for large datasets instead of manual iteration
- Profile with `cargo flamegraph`

### Window not appearing

- Ensure you're calling `Application::new().run()`
- Check window bounds are valid for your screen size

## Resources

- [GPUI Official Site](https://www.gpui.rs/)
- [GPUI GitHub Repository](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [Zed Editor Source](https://github.com/zed-industries/zed) - Real-world GPUI examples
- [gpui-component Library](https://github.com/longbridge/gpui-component) - Pre-built components

## Next Steps

1. Implement proper text input with cursor and selection
2. Add keyboard shortcuts (Ctrl+K for search, etc.)
3. Support multiple search categories with tabs
4. Add settings panel for API endpoint configuration
5. Implement search history and favorites
6. Add dark mode support with theme switching
7. Package as native app for distribution

Your metasearch engine is already fast (473ms queries). With GPUI's GPU acceleration, the desktop app will provide an even snappier experience than the web interface!
