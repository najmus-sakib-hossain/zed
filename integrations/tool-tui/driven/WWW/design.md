# Design Document: DX WWW Framework Structure

## Overview

The DX WWW Framework is a binary-first, multi-language web framework that provides a file-system based routing system, component architecture, and build pipeline. This design defines the complete folder structure, routing conventions, compilation process, and developer experience for building high-performance web applications.

The framework compiles `.pg` (page) and `.cp` (component) files into `.dxob` binary format, achieving zero-parse performance. It supports multiple programming languages (Rust, Python, JavaScript, Go) in component scripts and integrates with dx-style for atomic CSS compilation to binary CSS.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Developer Source Code                    │
│  (pages/, components/, api/, layouts/, public/, styles/)    │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    DX WWW Build Pipeline                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Parser     │→ │   Compiler   │→ │  Optimizer   │     │
│  │ (.pg/.cp)    │  │  (to .dxob)  │  │  (minify)    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ CSS Compiler │  │ Asset Proc.  │  │ Route Gen.   │     │
│  │ (dx-style)   │  │ (images)     │  │ (manifest)   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Production Output                         │
│  (.dxob files, binary CSS, route manifest, static assets)   │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Runtime Environment                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Binary Loader│  │ Multi-Lang   │  │ DOM Renderer │     │
│  │ (.dxob)      │  │ Runtime      │  │ (dx-www-dom) │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

### Component Architecture

The framework follows a layered architecture:

1. **Source Layer**: Developer-authored files in standard folder structure
2. **Build Layer**: Compilation pipeline transforming source to binary format
3. **Runtime Layer**: Execution environment loading and running binary objects
4. **Rendering Layer**: DOM manipulation and UI updates

## Components and Interfaces

### 1. Project Folder Structure

The standard DX WWW project follows this structure:

```
my-app/
├── dx.config.toml          # Framework configuration
├── pages/                  # Routable pages (.pg files)
│   ├── index.pg           # Route: /
│   ├── about.pg           # Route: /about
│   ├── _layout.pg         # Root layout
│   ├── _error.pg          # Error page
│   ├── _404.pg            # 404 page
│   ├── blog/
│   │   ├── _layout.pg     # Blog layout
│   │   ├── index.pg       # Route: /blog
│   │   └── [slug].pg      # Route: /blog/:slug
│   └── user/
│       └── [id].pg        # Route: /user/:id
├── components/             # Reusable components (.cp files)
│   ├── Button.cp
│   ├── Card.cp
│   └── forms/
│       ├── Input.cp
│       └── Select.cp
├── layouts/                # Shared layouts
│   ├── MainLayout.cp
│   └── DashboardLayout.cp
├── api/                    # Server-side API routes
│   ├── users.rs           # Endpoint: /api/users
│   └── user/
│       └── [id].rs        # Endpoint: /api/user/:id
├── lib/                    # Shared utilities
│   ├── utils.rs
│   └── constants.rs
├── styles/                 # Global styles
│   └── global.css
├── public/                 # Static assets
│   ├── favicon.ico
│   └── images/
│       └── logo.png
└── .dx/                    # Build output (generated)
    ├── build/
    │   ├── pages/         # Compiled .dxob files
    │   ├── components/
    │   └── manifest.json  # Route manifest
    └── cache/             # Build cache
```

### 2. File System Router

**Interface:**

```rust
pub struct FileSystemRouter {
    routes: HashMap<String, Route>,
    dynamic_routes: Vec<DynamicRoute>,
    layouts: HashMap<String, Layout>,
}

pub struct Route {
    path: String,
    file_path: PathBuf,
    binary_path: PathBuf,
    layout_chain: Vec<String>,
    data_loader: Option<DataLoader>,
}

pub struct DynamicRoute {
    pattern: String,
    param_names: Vec<String>,
    file_path: PathBuf,
    is_catch_all: bool,
}

impl FileSystemRouter {
    pub fn new(pages_dir: &Path) -> Result<Self>;
    pub fn scan_pages(&mut self) -> Result<()>;
    pub fn match_route(&self, path: &str) -> Option<&Route>;
    pub fn extract_params(&self, route: &DynamicRoute, path: &str) -> HashMap<String, String>;
}
```

**Routing Rules:**

- `pages/index.pg` → `/`
- `pages/about.pg` → `/about`
- `pages/blog/post.pg` → `/blog/post`
- `pages/user/[id].pg` → `/user/:id` (dynamic)
- `pages/docs/[...slug].pg` → `/docs/*` (catch-all)
- `pages/_layout.pg` → Layout applied to siblings and children
- `pages/_error.pg` → Error page (not routable)
- `pages/_404.pg` → 404 page (not routable)

**Layout Resolution:**

Layouts are applied from root to leaf. For a page at `pages/blog/post/[id].pg`:

1. Check for `pages/_layout.pg` (root layout)
2. Check for `pages/blog/_layout.pg` (blog layout)
3. Check for `pages/blog/post/_layout.pg` (post layout)
4. Render page with layout chain: root → blog → post → page

### 3. Component File Format

**Page File (.pg):**

```html
<!-- pages/user/[id].pg -->
<script lang="rust">
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Props {
    id: String,
    user: User,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    name: String,
    email: String,
    avatar: String,
}

// Data loader - runs on server before rendering
pub async fn load(params: HashMap<String, String>) -> Result<Props, Error> {
    let id = params.get("id").unwrap();
    let user = fetch_user(id).await?;
    Ok(Props { id: id.clone(), user })
}

// Component logic
pub fn on_follow_click() {
    // Handle follow button click
}
</script>

<template>
  <div class="container mx-auto p-4">
    <div class="flex items-center gap-4">
      <img src={user.avatar} alt={user.name} class="w-16 h-16 rounded-full" />
      <div>
        <h1 class="text-2xl font-bold">{user.name}</h1>
        <p class="text-gray-600">{user.email}</p>
      </div>
      <button onClick={on_follow_click} class="btn btn-primary">
        Follow
      </button>
    </div>
  </div>
</template>

<style>
/* Component-scoped styles */
.container {
  max-width: 1200px;
}
</style>
```

**Component File (.cp):**

```html
<!-- components/Button.cp -->
<script lang="rust">
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Props {
    label: String,
    variant: String,
    disabled: bool,
    on_click: Option<fn()>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            label: "Button".to_string(),
            variant: "primary".to_string(),
            disabled: false,
            on_click: None,
        }
    }
}
</script>

<template>
  <button 
    class="btn btn-{variant}" 
    disabled={disabled}
    onClick={on_click}
  >
    {label}
  </button>
</template>

<style>
.btn {
  @apply px-4 py-2 rounded font-medium transition-colors;
}
.btn-primary {
  @apply bg-blue-600 text-white hover:bg-blue-700;
}
</style>
```

### 4. API Routes

**API Route File:**

```rust
// api/users.rs
use dx_www_server::{Request, Response, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    email: String,
}

// GET /api/users
pub async fn get(req: Request) -> Response {
    let users = fetch_all_users().await;
    Json(users).into()
}

// POST /api/users
pub async fn post(req: Request) -> Response {
    let user: User = req.json().await?;
    let created = create_user(user).await?;
    Json(created).with_status(201).into()
}

// PUT /api/users
pub async fn put(req: Request) -> Response {
    let user: User = req.json().await?;
    let updated = update_user(user).await?;
    Json(updated).into()
}

// DELETE /api/users
pub async fn delete(req: Request) -> Response {
    let id = req.query("id")?;
    delete_user(&id).await?;
    Response::no_content()
}
```

**Dynamic API Route:**

```rust
// api/user/[id].rs
use dx_www_server::{Request, Response, Json};

// GET /api/user/:id
pub async fn get(req: Request) -> Response {
    let id = req.param("id")?;
    let user = fetch_user(id).await?;
    Json(user).into()
}
```

### 5. Data Loading System

**Data Loader Interface:**

```rust
pub trait DataLoader {
    type Props: Serialize + Deserialize;
    type Error: Into<LoadError>;
    
    async fn load(params: HashMap<String, String>) -> Result<Self::Props, Self::Error>;
}
```

**Execution Flow:**

1. Router matches incoming request to route
2. If route has data loader, execute it with route params
3. Data loader fetches data (from DB, API, etc.)
4. Data loader returns props or error
5. If success, pass props to page component
6. If error, render error page with error info
7. Render page with props and send to client

**Caching Strategy:**

- Development: No caching (always fresh)
- Production: Cache based on route and params
- Cache invalidation: Time-based or manual
- Cache storage: In-memory or Redis

### 6. Build Pipeline

**Build Process:**

```
Source Files (.pg, .cp)
    ↓
[1] Parse Component
    ↓
[2] Extract Sections (script, template, style)
    ↓
[3] Compile Script (Rust/Python/JS/Go → WASM/Native)
    ↓
[4] Compile Template (HTML → Binary DOM Instructions)
    ↓
[5] Compile Styles (Atomic CSS → Binary CSS via dx-style)
    ↓
[6] Bundle Dependencies
    ↓
[7] Optimize (Tree-shake, Minify)
    ↓
[8] Generate Binary Object (.dxob)
    ↓
Output (.dxob + manifest)
```

**Build Pipeline Interface:**

```rust
pub struct BuildPipeline {
    config: BuildConfig,
    cache: BuildCache,
    compiler: Compiler,
    optimizer: Optimizer,
}

impl BuildPipeline {
    pub fn new(config: BuildConfig) -> Self;
    pub async fn build(&mut self) -> Result<BuildOutput>;
    pub async fn build_incremental(&mut self, changed_files: Vec<PathBuf>) -> Result<BuildOutput>;
    pub async fn watch(&mut self) -> Result<()>;
}

pub struct BuildOutput {
    pub binary_objects: Vec<BinaryObject>,
    pub manifest: RouteManifest,
    pub assets: Vec<Asset>,
    pub source_maps: Vec<SourceMap>,
}

pub struct BinaryObject {
    pub path: PathBuf,
    pub size: usize,
    pub hash: String,
    pub dependencies: Vec<String>,
}
```

**Incremental Compilation:**

- Track file dependencies in build cache
- When file changes, identify affected files
- Recompile only affected files and their dependents
- Update manifest with new binary objects
- Preserve unchanged binary objects

### 7. Configuration System

**dx.config.toml Structure:**

```toml
[project]
name = "my-app"
version = "0.1.0"

[build]
output_dir = ".dx/build"
cache_dir = ".dx/cache"
optimization_level = "release"  # "debug" | "release" | "size"
target = "web"  # "web" | "server" | "edge"
source_maps = true

[routing]
pages_dir = "pages"
api_dir = "api"
trailing_slash = false  # Add trailing slash to routes
case_sensitive = false  # Case-sensitive route matching

[dev]
port = 3000
host = "localhost"
hot_reload = true
open_browser = true

[languages]
default = "rust"
enabled = ["rust", "python", "javascript", "go"]

[css]
compiler = "dx-style"
atomic_classes = true
purge_unused = true

[assets]
public_dir = "public"
optimize_images = true
content_hash = true  # Add hash to filenames for cache busting

[server]
ssr = true  # Server-side rendering
api_prefix = "/api"
cors_enabled = false
```

**Configuration Loading:**

```rust
pub struct DxConfig {
    pub project: ProjectConfig,
    pub build: BuildConfig,
    pub routing: RoutingConfig,
    pub dev: DevConfig,
    pub languages: LanguageConfig,
    pub css: CssConfig,
    pub assets: AssetConfig,
    pub server: ServerConfig,
}

impl DxConfig {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
    pub fn merge_with_defaults(self) -> Self;
}
```

### 8. Development Server

**Dev Server Interface:**

```rust
pub struct DevServer {
    config: DevConfig,
    router: FileSystemRouter,
    watcher: FileWatcher,
    hot_reload: HotReloadSystem,
    build_pipeline: BuildPipeline,
}

impl DevServer {
    pub fn new(config: DevConfig) -> Self;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn handle_request(&self, req: Request) -> Response;
    pub async fn handle_file_change(&mut self, path: PathBuf) -> Result<()>;
}
```

**Hot Reload System:**

```rust
pub struct HotReloadSystem {
    clients: Vec<WebSocketClient>,
}

impl HotReloadSystem {
    pub fn new() -> Self;
    pub async fn notify_change(&self, change: FileChange) -> Result<()>;
    pub async fn send_update(&self, update: HotUpdate) -> Result<()>;
}

pub enum HotUpdate {
    FullReload,
    ComponentUpdate { path: String, binary: Vec<u8> },
    StyleUpdate { css: Vec<u8> },
}
```

**Error Overlay:**

When compilation or runtime errors occur, display an overlay in the browser:

```rust
pub struct ErrorOverlay {
    pub error_type: ErrorType,
    pub message: String,
    pub file_path: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code_context: Option<String>,
    pub stack_trace: Option<Vec<StackFrame>>,
}

pub enum ErrorType {
    CompilationError,
    RuntimeError,
    DataLoadError,
}
```

### 9. CLI Commands

**Command Interface:**

```rust
pub enum DxCommand {
    New { name: String, template: Option<String> },
    Dev { port: Option<u16> },
    Build { mode: BuildMode },
    Generate { item_type: GenerateType, name: String },
    Deploy { target: DeployTarget },
}

pub enum GenerateType {
    Page,
    Component,
    ApiRoute,
    Layout,
}
```

**CLI Usage:**

```bash
# Create new project
dx new my-app
dx new my-app --template=blog

# Start development server
dx dev
dx dev --port=4000

# Build for production
dx build
dx build --mode=release
dx build --target=server

# Generate files
dx generate page about
dx generate component Button
dx generate api users
dx generate layout Dashboard

# Deploy
dx deploy --target=vercel
dx deploy --target=cloudflare
```

### 10. Production Build Output

**Output Structure:**

```
.dx/build/
├── manifest.json           # Route manifest
├── pages/
│   ├── index.dxob         # Compiled page binaries
│   ├── about.dxob
│   └── blog/
│       └── [slug].dxob
├── components/
│   ├── Button.dxob        # Compiled component binaries
│   └── Card.dxob
├── styles/
│   └── main.bcss          # Binary CSS
├── assets/
│   ├── logo-a3f2b1.png    # Hashed assets
│   └── fonts/
│       └── inter-9d4c2e.woff2
└── server/
    └── api.wasm           # API routes compiled to WASM
```

**Route Manifest (manifest.json):**

```json
{
  "version": "1.0.0",
  "routes": [
    {
      "path": "/",
      "binary": "pages/index.dxob",
      "layouts": ["_layout.dxob"],
      "preload": ["components/Button.dxob"],
      "data_loader": true
    },
    {
      "path": "/blog/:slug",
      "binary": "pages/blog/[slug].dxob",
      "layouts": ["_layout.dxob", "blog/_layout.dxob"],
      "dynamic": true,
      "params": ["slug"]
    }
  ],
  "api_routes": [
    {
      "path": "/api/users",
      "handler": "server/api.wasm",
      "methods": ["GET", "POST"]
    }
  ],
  "assets": {
    "logo.png": "assets/logo-a3f2b1.png",
    "fonts/inter.woff2": "assets/fonts/inter-9d4c2e.woff2"
  }
}
```

## Data Models

### Component Model

```rust
pub struct Component {
    pub name: String,
    pub file_path: PathBuf,
    pub component_type: ComponentType,
    pub script: Script,
    pub template: Template,
    pub styles: Styles,
}

pub enum ComponentType {
    Page,
    Component,
    Layout,
}

pub struct Script {
    pub language: Language,
    pub source: String,
    pub compiled: Vec<u8>,
    pub exports: Vec<Export>,
}

pub enum Language {
    Rust,
    Python,
    JavaScript,
    Go,
}

pub struct Template {
    pub source: String,
    pub ast: TemplateAst,
    pub binary: Vec<u8>,
}

pub struct Styles {
    pub source: String,
    pub scoped: bool,
    pub binary_css: Vec<u8>,
}
```

### Route Model

```rust
pub struct Route {
    pub path: String,
    pub pattern: RoutePattern,
    pub component: Component,
    pub layouts: Vec<Component>,
    pub data_loader: Option<DataLoader>,
    pub metadata: RouteMetadata,
}

pub enum RoutePattern {
    Static(String),
    Dynamic { pattern: String, params: Vec<String> },
    CatchAll { prefix: String, param: String },
}

pub struct RouteMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub meta_tags: HashMap<String, String>,
}
```

### Binary Object Model

```rust
pub struct BinaryObject {
    pub header: BinaryHeader,
    pub script_section: Vec<u8>,
    pub template_section: Vec<u8>,
    pub style_section: Vec<u8>,
    pub dependency_section: Vec<u8>,
}

pub struct BinaryHeader {
    pub magic: [u8; 4],  // "DXOB"
    pub version: u32,
    pub component_type: u8,
    pub language: u8,
    pub flags: u32,
    pub script_offset: u64,
    pub script_size: u64,
    pub template_offset: u64,
    pub template_size: u64,
    pub style_offset: u64,
    pub style_size: u64,
    pub dependency_offset: u64,
    pub dependency_size: u64,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: File-system to route mapping preserves directory structure
*For any* page file in a nested directory structure, the generated route path should preserve the directory hierarchy, where `pages/a/b/c.pg` maps to `/a/b/c`.
**Validates: Requirements 2.3, 2.7**

### Property 2: Dynamic route pattern recognition
*For any* page file with bracket notation `[param]` or catch-all notation `[...param]`, the router should create a dynamic route that correctly extracts the parameter from matching URLs.
**Validates: Requirements 2.4, 2.5, 4.2**

### Property 3: Layout application scope
*For any* `_layout.pg` file in a directory, the framework should apply it to all pages in that directory and its subdirectories, but not to pages outside that scope.
**Validates: Requirements 2.6, 10.1**

### Property 4: Layout chain composition
*For any* page with multiple applicable layouts, the framework should wrap the page with all layouts in order from root to leaf, where each layout receives the next layer as its content.
**Validates: Requirements 10.3, 10.4**

### Property 5: Data loader execution order
*For any* page with a data loader function, the framework should execute the loader before rendering and pass its results as props to the component.
**Validates: Requirements 5.1, 5.2**

### Property 6: Data loader error handling
*For any* data loader that throws an error, the framework should catch the error, prevent page rendering, and provide error information to the error page.
**Validates: Requirements 5.3**

### Property 7: Data loader caching consistency
*For any* data loader with the same route and parameters, subsequent calls should return cached results until cache invalidation occurs.
**Validates: Requirements 5.5**

### Property 8: Source to binary compilation
*For any* valid page or component file, the build pipeline should generate a corresponding binary object file with all sections (script, template, style) properly compiled.
**Validates: Requirements 6.1, 9.1**

### Property 9: CSS compilation to binary format
*For any* atomic CSS class used in components, the build pipeline should compile it to binary CSS format and include it in the output.
**Validates: Requirements 6.2**

### Property 10: Build optimization reduces size
*For any* binary object file, the optimized version should have a size less than or equal to the unoptimized version while maintaining functional equivalence.
**Validates: Requirements 6.3**

### Property 11: Route manifest completeness
*For any* build output, the generated manifest should contain entries for all routes discovered in the pages directory, with correct mappings to binary object files.
**Validates: Requirements 6.4, 9.2**

### Property 12: Incremental compilation efficiency
*For any* source file change, the build pipeline should recompile only the changed file and its dependents, leaving unaffected files unchanged.
**Validates: Requirements 6.5**

### Property 13: Syntax validation before compilation
*For any* source file with syntax errors in its script section, the build pipeline should detect and report the errors before attempting compilation.
**Validates: Requirements 6.6**

### Property 14: Dependency bundling completeness
*For any* component with external dependencies, the generated binary object should include all required dependencies such that the component can execute independently.
**Validates: Requirements 6.7**

### Property 15: Configuration validation
*For any* invalid `dx.config.toml` file, the framework should reject it with a clear error message indicating the specific validation failure.
**Validates: Requirements 7.7**

### Property 16: Hot reload without full refresh
*For any* source file change during development, the hot reload system should update the browser with the new code without triggering a full page refresh.
**Validates: Requirements 8.2**

### Property 17: Compilation error display
*For any* compilation error, the framework should display an error overlay in the browser containing the file path, line number, and code context.
**Validates: Requirements 8.3, 12.2**

### Property 18: API route serialization
*For any* API route that returns data, the framework should serialize the response to the appropriate format (JSON, binary, etc.) based on content negotiation.
**Validates: Requirements 4.4**

### Property 19: Source map generation
*For any* production build, the build pipeline should generate source maps that correctly map compiled binary code back to original source locations.
**Validates: Requirements 9.5**

### Property 20: Tree-shaking removes unused code
*For any* production build, the build pipeline should exclude code that is not reachable from any entry point, resulting in smaller binary objects.
**Validates: Requirements 9.6**

### Property 21: Static asset path preservation
*For any* file in the `public/` directory, the framework should serve it at a URL path that preserves the file's relative path within the public directory.
**Validates: Requirements 11.1**

### Property 22: Asset optimization reduces size
*For any* image asset, the optimized version should have a file size less than or equal to the original while maintaining acceptable visual quality.
**Validates: Requirements 11.3**

### Property 23: Content hashing for cache busting
*For any* static asset in production builds, the framework should generate a filename with a content hash that changes when the file content changes.
**Validates: Requirements 11.4**

### Property 24: Asset URL resolution
*For any* asset imported in a component, the framework should resolve the import to the correct URL path, accounting for content hashing and public path configuration.
**Validates: Requirements 11.5**

### Property 25: Runtime error overlay display
*For any* runtime error during development, the framework should display an error overlay with the error message and stack trace.
**Validates: Requirements 12.1**

### Property 26: API error response handling
*For any* API route that throws an error, the framework should return an HTTP error response with an appropriate status code and error message.
**Validates: Requirements 12.5**

### Property 27: Development error logging
*For any* error in development mode, the framework should log detailed error information to the console including stack traces and context.
**Validates: Requirements 12.6**

### Property 28: Component naming convention enforcement
*For any* component file with a PascalCase name, the framework should treat it as a reusable component, and for any page file with a kebab-case name, the framework should treat it as a routable page.
**Validates: Requirements 3.3, 3.4**

## Error Handling

### Compilation Errors

**Error Types:**
- Syntax errors in script sections
- Invalid template syntax
- Invalid CSS syntax
- Missing dependencies
- Type errors (for statically typed languages)

**Error Handling Strategy:**
1. Detect errors during parsing/compilation phase
2. Collect all errors (don't stop at first error)
3. Format errors with file path, line, column, and context
4. In development: Display error overlay in browser
5. In production build: Fail build with error report

**Error Display Format:**

```
Compilation Error in pages/user/[id].pg

  × Syntax error in script section
   ╭─[pages/user/[id].pg:12:5]
 12 │     let user = fetch_user(id).await?;
 13 │     Ok(Props { id: id.clone(), user )
    ·                                     ─
    ·                                     ╰── Expected closing brace
 14 │ }
   ╰────
```

### Runtime Errors

**Error Types:**
- Data loader failures
- Component rendering errors
- API route errors
- Network errors

**Error Handling Strategy:**
1. Catch errors at appropriate boundaries
2. Log errors with full context
3. Display user-friendly error messages
4. Provide recovery mechanisms where possible
5. Support custom error pages

**Error Boundaries:**

```rust
pub trait ErrorBoundary {
    fn catch_error(&self, error: Error) -> ErrorRecovery;
}

pub enum ErrorRecovery {
    DisplayErrorPage { error: Error },
    Retry { max_attempts: u32 },
    Fallback { component: Component },
    Propagate,
}
```

### Data Loading Errors

**Handling Strategy:**
1. Catch data loader errors before rendering
2. Pass error to error page component
3. Provide retry mechanism
4. Log error details for debugging

**Error Page Props:**

```rust
pub struct ErrorPageProps {
    pub error: Error,
    pub status_code: u16,
    pub retry_url: Option<String>,
    pub stack_trace: Option<Vec<StackFrame>>,
}
```

### API Route Errors

**HTTP Error Responses:**

```rust
pub enum ApiError {
    BadRequest { message: String },
    Unauthorized { message: String },
    Forbidden { message: String },
    NotFound { message: String },
    InternalServerError { message: String },
    Custom { status: u16, message: String },
}

impl ApiError {
    pub fn to_response(&self) -> Response {
        let (status, body) = match self {
            ApiError::BadRequest { message } => (400, json!({ "error": message })),
            ApiError::Unauthorized { message } => (401, json!({ "error": message })),
            ApiError::Forbidden { message } => (403, json!({ "error": message })),
            ApiError::NotFound { message } => (404, json!({ "error": message })),
            ApiError::InternalServerError { message } => (500, json!({ "error": message })),
            ApiError::Custom { status, message } => (*status, json!({ "error": message })),
        };
        Response::json(body).with_status(status)
    }
}
```

## Testing Strategy

### Dual Testing Approach

The DX WWW Framework requires both unit tests and property-based tests for comprehensive coverage:

**Unit Tests:**
- Specific examples of routing behavior (e.g., `pages/index.pg` → `/`)
- Edge cases (empty directories, special characters in filenames)
- Error conditions (invalid config, malformed files)
- Integration points (CLI commands, dev server startup)
- Specific file format examples (`.pg`, `.cp` recognition)

**Property-Based Tests:**
- Universal routing properties (all file paths map correctly)
- Dynamic route parameter extraction (all bracket patterns work)
- Layout application scope (all layouts apply to correct pages)
- Compilation output (all valid files produce binary objects)
- Cache consistency (all identical requests return cached results)

### Property-Based Testing Configuration

**Testing Library:** Use `proptest` for Rust implementation

**Test Configuration:**
- Minimum 100 iterations per property test
- Each test references its design document property
- Tag format: `Feature: www-framework-structure, Property N: [property text]`

**Example Property Test:**

```rust
use proptest::prelude::*;

// Feature: www-framework-structure, Property 1: File-system to route mapping preserves directory structure
proptest! {
    #[test]
    fn test_route_mapping_preserves_structure(
        dirs in prop::collection::vec("[a-z]+", 0..5),
        filename in "[a-z]+\\.pg"
    ) {
        let path = format!("pages/{}/{}", dirs.join("/"), filename);
        let expected_route = format!("/{}/{}", dirs.join("/"), filename.trim_end_matches(".pg"));
        
        let router = FileSystemRouter::new(Path::new("pages"))?;
        let route = router.match_route(&expected_route);
        
        prop_assert!(route.is_some());
        prop_assert_eq!(route.unwrap().file_path, PathBuf::from(path));
    }
}
```

### Test Coverage Requirements

**Core Functionality:**
- File-system routing: 100% of routing rules
- Component compilation: All file types and languages
- Data loading: All execution paths
- Error handling: All error types
- Configuration: All config options

**Integration Tests:**
- Full build pipeline (source → binary)
- Dev server with hot reload
- Production build output
- CLI commands
- Multi-language support

**Performance Tests:**
- Build time for large projects
- Hot reload latency
- Binary object size
- Runtime performance

### Testing Best Practices

1. **Property tests for universal rules**: Use property-based testing for routing, compilation, and data flow
2. **Unit tests for specific cases**: Use unit tests for CLI commands, config parsing, and error messages
3. **Integration tests for workflows**: Test complete user workflows (create project → develop → build → deploy)
4. **Avoid over-testing**: Don't write excessive unit tests for cases covered by property tests
5. **Focus on correctness**: Prioritize tests that verify correctness properties over implementation details

## Implementation Notes

### Build Pipeline Optimization

**Caching Strategy:**
- Cache parsed ASTs to avoid re-parsing unchanged files
- Cache compiled binary objects with content-based keys
- Invalidate cache when dependencies change
- Use file modification times for quick cache checks

**Parallel Compilation:**
- Compile independent files in parallel using thread pool
- Respect dependency order for dependent files
- Use work-stealing for load balancing
- Limit parallelism based on CPU cores

### Multi-Language Support

**Language Runtime Integration:**
- Rust: Compile to native code or WASM
- Python: Use PyO3 for embedding or compile to WASM via RustPython
- JavaScript: Use QuickJS or V8 for execution
- Go: Compile to WASM via TinyGo

**Language Interop:**
- Define common interface for all languages
- Use message passing for cross-language communication
- Serialize data using binary format for efficiency
- Support language-specific optimizations

### Binary Format Design

**DXOB Format Specification:**

```
[Header: 64 bytes]
  - Magic: "DXOB" (4 bytes)
  - Version: u32 (4 bytes)
  - Component Type: u8 (1 byte)
  - Language: u8 (1 byte)
  - Flags: u32 (4 bytes)
  - Section Offsets: 6 × u64 (48 bytes)
  - Reserved: 2 bytes

[Script Section]
  - Compiled code (WASM, native, or bytecode)
  - Export table
  - Import table

[Template Section]
  - Binary DOM instructions
  - Element tree structure
  - Attribute bindings
  - Event handlers

[Style Section]
  - Binary CSS rules
  - Selector tree
  - Property values

[Dependency Section]
  - Dependency list
  - Version constraints
  - Resolution metadata

[Metadata Section]
  - Component name
  - Props schema
  - Events schema
  - Documentation
```

### Performance Targets

**Build Performance:**
- Initial build: < 5 seconds for 100 components
- Incremental build: < 500ms for single file change
- Hot reload: < 100ms from file save to browser update

**Runtime Performance:**
- Binary load time: < 10ms per component
- First paint: < 100ms
- Time to interactive: < 500ms
- Binary size: < 10KB per component (average)

**Development Experience:**
- Dev server startup: < 2 seconds
- Error display: < 50ms after error occurs
- CLI command response: < 100ms

## Deployment Strategies

### Static Deployment

**Output:**
- Pre-rendered HTML for all static routes
- Binary objects for client-side hydration
- Static assets with content hashing
- Route manifest for client-side routing

**Deployment Targets:**
- CDN (Cloudflare, Fastly, AWS CloudFront)
- Static hosting (Netlify, Vercel, GitHub Pages)
- Object storage (S3, GCS, Azure Blob)

### Server Deployment

**Output:**
- Server binary with embedded runtime
- Binary objects for all routes
- API route handlers
- SSR support for dynamic routes

**Deployment Targets:**
- Traditional servers (VPS, dedicated)
- Container platforms (Docker, Kubernetes)
- Serverless (AWS Lambda, Google Cloud Functions)

### Edge Deployment

**Output:**
- Edge-optimized binary objects
- Minimal runtime for edge execution
- Distributed data loading
- Edge caching configuration

**Deployment Targets:**
- Cloudflare Workers
- Deno Deploy
- Fastly Compute@Edge
- AWS Lambda@Edge

## Future Enhancements

### Planned Features

1. **Streaming SSR**: Stream HTML as components render
2. **Partial Hydration**: Hydrate only interactive components
3. **Islands Architecture**: Support for isolated interactive regions
4. **Incremental Static Regeneration**: Update static pages without full rebuild
5. **Edge Rendering**: Render pages at edge locations
6. **Real-time Collaboration**: Built-in WebSocket support
7. **Offline Support**: Service worker generation and offline caching
8. **Progressive Enhancement**: Graceful degradation for non-JS clients

### Research Areas

1. **Binary Protocol Optimization**: Further reduce binary object size
2. **Advanced Caching**: Predictive prefetching and smart caching
3. **AI-Assisted Development**: Code generation and optimization suggestions
4. **Visual Development**: Drag-and-drop component builder
5. **Performance Monitoring**: Built-in performance tracking and optimization
