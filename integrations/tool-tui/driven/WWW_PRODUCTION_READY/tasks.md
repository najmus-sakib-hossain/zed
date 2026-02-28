# DX-WWW Production Ready: Complete Implementation Plan

## Vision
Build a 10/10 production-ready web framework that beats React/Next.js/Laravel/Django with:
- **Binary-first architecture** (HTIP protocol)
- **Multi-language support** (Rust, Python, JS, Go → WASM)
- **Svelte-style directives** with React-style props
- **Tailwind-like atomic CSS** → Binary Dawn CSS
- **File-based routing** (.pg pages, .cp components)
- **`dx create my-app`** that just works

---

## Implementation Progress Summary

### ✅ COMPLETED

#### Phase 1: Core Syntax & Parser
- [x] Created DX format parser at `crates/www/core/src/dx_parser/dx_format.rs`
- [x] Supports multi-language scripts (`<script lang="rust|python|js|go">`)
- [x] Parses Svelte-style directives: `{#if}`, `{#each}`, `{#await}`, `{#key}`
- [x] Parses expression interpolation: `{variable}`
- [x] Supports special directives: `bind:`, `class:`, `use:`, `transition:`
- [x] Extracts CSS classes for dx-style processing
- [x] Supports slots: `<slot />`, `<slot name="header" />`

#### Phase 2: File-Based Router
- [x] Created router crate at `crates/www/router/`
- [x] Supports static routes: `/about` → `pages/about.pg`
- [x] Supports dynamic routes: `/blog/[slug]` → `pages/blog/[slug].pg`
- [x] Supports catch-all routes: `/docs/[...path]` → `pages/docs/[...path].pg`
- [x] Route scanning via `scan_pages_directory()`
- [x] Route matching via `Router::match_path()`

#### Phase 5: Component Library (shadcn-style)
- [x] Created 32+ components in `crates/www/core/src/components.rs`
- [x] Primitives: Button, Input, Textarea, Select, Checkbox, Radio, Switch
- [x] Layout: Card, Container, Separator
- [x] Navigation: Tabs, Breadcrumb, Pagination
- [x] Feedback: Alert, Toast, Progress, Skeleton, Spinner
- [x] Overlay: Modal, Dialog, Drawer, Popover, Tooltip, DropdownMenu
- [x] Data Display: Table, Badge, Avatar, Accordion
- [x] Form: Form, FormField, Label
- [x] All components have A11y definitions (ARIA, keyboard nav)

#### Phase 6: CLI Commands
- [x] `dx create my-app` / `dx new my-app` - Create new project
- [x] `dx add button card modal` - Add components
- [x] `dx add --all` - Add all components
- [x] `dx add --list` - List available components
- [x] `dx generate page about` - Generate page
- [x] `dx generate component Header` - Generate component
- [x] `dx dev` - Development server
- [x] `dx build` - Production build

#### Documentation
- [x] Created 10-minute quick start tutorial at `docs/QUICK_START.md`

### 🔄 IN PROGRESS

#### Phase 1: Multi-Language WASM Compilation
- [ ] Python → WASM via Pyodide
- [ ] JavaScript → WASM via OXC
- [ ] Go → WASM via TinyGo

#### Phase 3: Binary Compilation Pipeline
- [ ] Component → HTIP Binary (.dxob)
- [ ] Page → Route Binary
- [ ] Layout → Layout Binary

---

## Phase 1: Core Syntax & Compiler (CRITICAL PATH)

### 1.1 Define DX-WWW File Format Specification
- [x] **File Extensions**
  - `.pg` for pages (short for "page")
  - `.cp` for components (short for "component")
  - `.lyt` for layouts (short for "layout")
  - `dx` for config (no extension, uses DX Serializer format)

- [x] **1.1.1 Create File Format Parser** ✅ DONE
  - Create `crates/www/core/src/dx_parser/dx_format.rs`
  - Parse `<script lang="...">` blocks (rust, python, js, go, etc.)
  - Parse `<page>` / `<component>` / `<layout>` blocks
  - Support multiple script blocks per file
  - Extract Tailwind-like class names for dx-style processing
  
- [x] **1.1.2 Implement Svelte-style Directives Parser** ✅ DONE
  - `{#if condition}...{:else if}...{:else}...{/if}`
  - `{#each items as item, index}...{/each}`
  - `{#await promise}...{:then data}...{:catch error}...{/await}`
  - `{#key value}...{/key}`
  - `{expression}` interpolation (Vue/React-style)
  
- [x] **1.1.3 Implement Special Directives** ✅ DONE
  - `bind:value={variable}` - Two-way binding
  - `class:active={isActive}` - Conditional classes
  - `use:action` - Custom directives/actions
  - `transition:fade` - Animation directives
  - `on:click={handler}` or `onClick={handler}` - Events

- [x] **1.1.4 Implement Slots & Composition** ✅ DONE
  - `<slot />` - Default slot
  - `<slot name="header" />` - Named slots
  - `<Component><div slot="header">...</div></Component>` - Slot usage

### 1.2 Multi-Language WASM Compilation
- [ ] **1.2.1 Rust Script Compiler**
  - Parse `<script lang="rust">` blocks
  - Extract Props struct definitions
  - Extract reactive state declarations
  - Compile to WASM via rustc/wasm-pack
  
- [ ] **1.2.2 Python Script Compiler**
  - Parse `<script lang="python">` blocks
  - Compile via Pyodide or RustPython → WASM
  - Support async functions
  
- [ ] **1.2.3 JavaScript/TypeScript Compiler**
  - Parse `<script>` or `<script lang="js|ts">` blocks
  - Compile via OXC or SWC → WASM (via wasm-bindgen)
  
- [ ] **1.2.4 Go Script Compiler**
  - Parse `<script lang="go">` blocks
  - Compile via TinyGo → WASM

### 1.3 Binary Compilation Pipeline
- [ ] **1.3.1 Component → HTIP Binary**
  - Parse .cp file → AST
  - Extract static template parts
  - Extract dynamic bindings
  - Generate HTIP opcodes
  - Output .dxob binary
  
- [ ] **1.3.2 Page → Route Binary**
  - Parse .pg file → AST
  - Generate route metadata
  - Compile page component
  - Generate HTIP + route manifest
  
- [ ] **1.3.3 Layout → Layout Binary**
  - Parse .lyt file → AST
  - Support nested layouts
  - Generate layout composition tree

---

## Phase 2: File-Based Router

### 2.1 Router Core
- [x] **2.1.1 Create Router Crate** ✅ DONE
  - Create `crates/www/router/Cargo.toml`
  - Create `crates/www/router/src/lib.rs`
  - Implement Route struct with pattern matching
  - Support static routes: `/about` → `pages/about.pg`
  - Support dynamic routes: `/blog/[slug]` → `pages/blog/[slug].pg`
  - Support catch-all routes: `/docs/[...path]` → `pages/docs/[...path].pg`
  - Support route groups: `(marketing)/pricing.pg` → `/pricing`

- [ ] **2.1.2 Pages Directory Scanner**
  - Scan `www/pages/` directory recursively
  - Generate route table from file structure
  - Support `_layout.lyt` for nested layouts
  - Support `_error.pg` for error pages
  - Support `_loading.pg` for loading states

- [ ] **2.1.3 Link Component**
  - Create `<Link href="/about">` component
  - Implement client-side navigation (no page reload)
  - Support prefetching on hover
  - Support active state styling

- [ ] **2.1.4 Navigation API**
  - `navigate("/path")` function
  - `useRoute()` hook for current route info
  - `useParams()` hook for dynamic params
  - Back/forward browser history support

### 2.2 Server-Side Rendering
- [ ] **2.2.1 SSR Engine**
  - Render HTIP binary to HTML string
  - Support `load()` function for data fetching
  - Inject initial state into HTML
  - Generate meta tags for SEO

- [ ] **2.2.2 Hydration**
  - Send minimal JS for hydration
  - Attach event listeners to server-rendered HTML
  - Restore state from server
  - Progressive hydration support

---

## Phase 3: Component Library (shadcn-style)

### 3.1 Design System Foundation
- [ ] **3.1.1 Create Component Registry**
  - Create `crates/www/components/Cargo.toml`
  - Create component registry system
  - Support `dx add button` to add components
  - Components are copied to project (not npm dependency)

- [ ] **3.1.2 Theme System**
  - CSS variables for colors, spacing, typography
  - Dark mode support via `class="dark"`
  - Theme customization via `dx` config

### 3.2 Form Controls
- [ ] **3.2.1 Button Component**
  - Variants: default, destructive, outline, secondary, ghost, link
  - Sizes: sm, md, lg, icon
  - States: disabled, loading
  - Accessible (ARIA)

- [ ] **3.2.2 Input Component**
  - Types: text, email, password, number, search
  - States: error, disabled, readonly
  - Icons: leading, trailing
  - Accessible labels

- [ ] **3.2.3 Select Component**
  - Native select wrapper
  - Custom dropdown (keyboard navigable)
  - Multi-select support
  - Search/filter support

- [ ] **3.2.4 Checkbox & Radio**
  - Styled checkbox with custom checkmark
  - Radio group with keyboard navigation
  - Indeterminate state for checkbox

- [ ] **3.2.5 Switch/Toggle**
  - Accessible toggle component
  - Label support
  - Sizes: sm, md, lg

- [ ] **3.2.6 Textarea**
  - Auto-resize option
  - Character count
  - Error states

### 3.3 Layout Components
- [ ] **3.3.1 Card Component**
  - Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter
  - Variants: default, bordered, elevated

- [ ] **3.3.2 Dialog/Modal Component**
  - Accessible modal with focus trap
  - Animations (fade, slide, scale)
  - Close on escape, close on backdrop click
  - DialogTrigger, DialogContent, DialogHeader, DialogFooter

- [ ] **3.3.3 Sheet/Drawer Component**
  - Side panels (left, right, top, bottom)
  - Swipe to close on mobile

- [ ] **3.3.4 Tabs Component**
  - Keyboard navigation
  - Vertical and horizontal layouts
  - Controlled and uncontrolled modes

- [ ] **3.3.5 Accordion Component**
  - Single or multiple open panels
  - Animated expand/collapse
  - Keyboard navigation

### 3.4 Data Display
- [ ] **3.4.1 Table Component**
  - Sortable columns
  - Pagination
  - Row selection
  - Sticky header
  - Responsive (horizontal scroll or card view)

- [ ] **3.4.2 Avatar Component**
  - Image with fallback to initials
  - Sizes: xs, sm, md, lg, xl
  - Avatar group with overlap

- [ ] **3.4.3 Badge Component**
  - Variants: default, secondary, destructive, outline
  - Dot indicator

- [ ] **3.4.4 Tooltip Component**
  - Hover/focus trigger
  - Positioning (top, bottom, left, right)
  - Arrow support

- [ ] **3.4.5 Progress Component**
  - Linear progress bar
  - Circular progress
  - Indeterminate state

### 3.5 Feedback Components
- [ ] **3.5.1 Alert Component**
  - Variants: default, destructive, warning, success
  - Icon support
  - Dismissible

- [ ] **3.5.2 Toast/Notification System**
  - Global toast container
  - Variants: default, success, error, warning, info
  - Auto-dismiss with configurable duration
  - Action buttons
  - Stack multiple toasts

- [ ] **3.5.3 Skeleton Component**
  - Loading placeholder
  - Various shapes (text, circle, rectangle)
  - Animation

- [ ] **3.5.4 Spinner Component**
  - Sizes: sm, md, lg
  - Color variants

### 3.6 Navigation Components
- [ ] **3.6.1 Navbar Component**
  - Logo, links, actions
  - Mobile hamburger menu
  - Sticky option

- [ ] **3.6.2 Sidebar Component**
  - Collapsible
  - Nested navigation
  - Mobile overlay

- [ ] **3.6.3 Breadcrumb Component**
  - Auto-generate from route
  - Custom separator

- [ ] **3.6.4 Pagination Component**
  - Page numbers
  - Previous/Next buttons
  - Items per page selector

- [ ] **3.6.5 Dropdown Menu**
  - Keyboard navigation
  - Nested menus
  - Icons and shortcuts

- [ ] **3.6.6 Command Palette (⌘K)**
  - Global search
  - Keyboard navigation
  - Recent items
  - Fuzzy search

### 3.7 Overlay Components
- [ ] **3.7.1 Popover Component**
  - Trigger element
  - Positioning with collision detection
  - Focus trap

- [ ] **3.7.2 Context Menu**
  - Right-click trigger
  - Nested submenus
  - Keyboard navigation

- [ ] **3.7.3 Hover Card**
  - Rich content on hover
  - Delay before showing

---

## Phase 4: State Management & Data

### 4.1 Reactive State System
- [ ] **4.1.1 Component State**
  - `let count = 0` - Reactive variable
  - `$: doubled = count * 2` - Derived/computed values
  - Automatic re-render on state change

- [ ] **4.1.2 Global Store**
  - Create stores with `createStore()`
  - Subscribe to store changes
  - Persist to localStorage option

- [ ] **4.1.3 Context API**
  - `setContext(key, value)` in parent
  - `getContext(key)` in child
  - Avoid prop drilling

### 4.2 Data Fetching (TanStack Query-like)
- [ ] **4.2.1 Query Hook**
  - `useQuery(key, fetcher)` - GET requests
  - Automatic caching with TTL
  - Stale-while-revalidate
  - Loading, error, success states
  - Refetch on window focus

- [ ] **4.2.2 Mutation Hook**
  - `useMutation(mutator)` - POST/PUT/DELETE
  - Optimistic updates
  - Error rollback
  - Invalidate queries on success

- [ ] **4.2.3 Server Functions**
  - Define functions that run on server
  - Call from client like regular functions
  - Automatic serialization

### 4.3 Form Handling
- [ ] **4.3.1 Form State Management**
  - Track field values, errors, touched state
  - `bind:value` for two-way binding
  - Submit handling

- [ ] **4.3.2 Validation**
  - Built-in validators (required, email, min, max, pattern)
  - Custom validator functions
  - Async validation (e.g., check username availability)
  - Real-time validation on blur/change

- [ ] **4.3.3 Form Actions**
  - Progressive enhancement (works without JS)
  - Handle form submission server-side
  - Return errors to client

### 4.4 Database Integration (Turso)
- [ ] **4.4.1 Turso Client**
  - Create `crates/www/db-turso/Cargo.toml`
  - libSQL client integration
  - Connection pooling
  - Transaction support

- [ ] **4.4.2 Query Builder**
  - Type-safe query builder
  - SQL template literals
  - Prepared statements
  - Migration support

- [ ] **4.4.3 ORM-like API**
  ```rust
  // Define model
  #[derive(Model)]
  struct User {
      id: i64,
      email: String,
      name: String,
  }
  
  // Query
  let users = User::find_all().where_("active = ?", true).await?;
  ```

---

## Phase 5: Developer Experience

### 5.1 CLI Tool (`dx`)
- [ ] **5.1.1 `dx create my-app`**
  - Interactive project creation
  - Template selection (blank, blog, dashboard, e-commerce)
  - Git initialization
  - Dependency installation

- [ ] **5.1.2 `dx dev`**
  - Start development server
  - Hot Module Replacement (HMR)
  - Error overlay in browser
  - Port configuration

- [ ] **5.1.3 `dx build`**
  - Production build
  - Minification and compression
  - Code splitting by route
  - Bundle analysis

- [ ] **5.1.4 `dx add [component]`**
  - Add components from registry
  - `dx add button` → copies Button.cp to project
  - `dx add --all` → add all components

- [ ] **5.1.5 `dx generate`**
  - `dx generate page about` → creates pages/about.pg
  - `dx generate component Card` → creates components/Card.cp
  - `dx generate layout dashboard` → creates layouts/dashboard.lyt

### 5.2 Hot Module Replacement
- [ ] **5.2.1 File Watcher**
  - Watch .pg, .cp, .lyt, dx files
  - Debounce rapid changes
  - Dependency tracking

- [ ] **5.2.2 Incremental Compilation**
  - Only recompile changed files
  - Update dependency graph
  - Send minimal update to browser

- [ ] **5.2.3 Client HMR Runtime**
  - WebSocket connection to dev server
  - Apply updates without page reload
  - Preserve component state during update

### 5.3 Error Handling
- [ ] **5.3.1 Compile-Time Errors**
  - Clear error messages with file/line
  - Suggestions for common mistakes
  - Links to documentation

- [ ] **5.3.2 Runtime Error Overlay**
  - Beautiful error overlay in browser
  - Stack trace with source maps
  - Click to open in editor

- [ ] **5.3.3 Error Boundaries**
  - Catch errors in components
  - Show fallback UI
  - Report errors to monitoring

### 5.4 DevTools Browser Extension
- [ ] **5.4.1 Component Inspector**
  - View component tree
  - Inspect props and state
  - Edit state live

- [ ] **5.4.2 Performance Profiler**
  - Render timing
  - Re-render reasons
  - Memory usage

- [ ] **5.4.3 Network Inspector**
  - View queries and mutations
  - Cache state
  - Replay requests

---

## Phase 6: Production Features

### 6.1 Build Optimization
- [ ] **6.1.1 Tree Shaking**
  - Remove unused code
  - Remove unused components
  - Remove unused icons

- [ ] **6.1.2 Code Splitting**
  - Split by route
  - Lazy load components
  - Prefetch on link hover

- [ ] **6.1.3 Asset Optimization**
  - Minify WASM
  - Compress with Brotli/gzip
  - Generate hashed filenames
  - Generate asset manifest

### 6.2 Deployment
- [ ] **6.2.1 Static Export**
  - Pre-render all routes to HTML
  - Works on any static host
  - `dx build --static`

- [ ] **6.2.2 Vercel Adapter**
  - Serverless functions for SSR
  - Edge functions support
  - `dx deploy --vercel`

- [ ] **6.2.3 Cloudflare Adapter**
  - Workers for edge SSR
  - D1/KV integration
  - `dx deploy --cloudflare`

- [ ] **6.2.4 Docker Support**
  - Multi-stage Dockerfile
  - Minimal runtime image
  - Health checks

### 6.3 Security
- [ ] **6.3.1 CSRF Protection**
  - Generate CSRF tokens
  - Validate on form submission

- [ ] **6.3.2 XSS Prevention**
  - Auto-escape interpolations
  - Sanitize user input
  - CSP headers

- [ ] **6.3.3 Auth Integration**
  - Session management
  - JWT support
  - OAuth providers (Google, GitHub, etc.)

### 6.4 Performance Monitoring
- [ ] **6.4.1 Core Web Vitals**
  - Track LCP, FID, CLS
  - Report to analytics

- [ ] **6.4.2 Error Tracking**
  - Capture runtime errors
  - Send to monitoring service
  - Source map support

---

## Phase 7: DX Website (www/ folder)

### 7.1 Project Setup
- [ ] **7.1.1 Configure dx config file**
  - Create `www/dx` configuration
  - Set up paths, fonts, icons, styles

- [ ] **7.1.2 Create Folder Structure**
  ```
  www/
  ├── dx                    # Config file (DX Serializer format)
  ├── pages/
  │   ├── index.pg          # Landing page
  │   ├── docs/
  │   │   ├── index.pg      # Docs home
  │   │   └── [...slug].pg  # Docs pages (catch-all)
  │   ├── pricing.pg        # Pricing page
  │   ├── blog/
  │   │   ├── index.pg      # Blog listing
  │   │   └── [slug].pg     # Blog post
  │   └── contact.pg        # Contact page
  ├── components/
  │   ├── ui/               # shadcn-style components
  │   ├── Header.cp
  │   ├── Footer.cp
  │   └── ...
  ├── layouts/
  │   ├── _layout.lyt       # Root layout
  │   ├── docs.lyt          # Docs layout with sidebar
  │   └── blog.lyt          # Blog layout
  ├── api/                  # API routes
  ├── public/               # Static assets
  ├── styles/               # Global CSS (compiled to Binary Dawn)
  └── lib/                  # Shared utilities
  ```

### 7.2 Pages Implementation
- [ ] **7.2.1 Landing Page (index.pg)**
  - Hero section with tagline
  - Feature highlights (Binary-first, Multi-language, Fast)
  - Performance metrics (338B runtime, 0ms hydration)
  - Code examples
  - Call to action (Get Started, View on GitHub)
  - Testimonials (when available)

- [ ] **7.2.2 Documentation (docs/)**
  - Getting Started guide
  - Installation instructions
  - Tutorial: Build your first app
  - API reference (auto-generated)
  - Component documentation
  - Search functionality

- [ ] **7.2.3 Pricing Page (pricing.pg)**
  - Free tier (open source)
  - Pro tier (optional services)
  - Enterprise tier (support)
  - Feature comparison table

- [ ] **7.2.4 Blog (blog/)**
  - Blog post listing with pagination
  - Individual blog post pages
  - Categories and tags
  - RSS feed

- [ ] **7.2.5 Contact Page (contact.pg)**
  - Contact form (name, email, message)
  - Form validation
  - Submit to API
  - Success/error feedback
  - Links to Discord, GitHub, Twitter

### 7.3 Design System
- [ ] **7.3.1 Color Palette**
  - Primary: Emerald/Teal (trust, innovation)
  - Neutral: Slate (professional)
  - Accent: Amber (attention)
  - Dark mode support

- [ ] **7.3.2 Typography**
  - Headings: Inter or similar sans-serif
  - Code: JetBrains Mono
  - Body: System fonts with fallbacks

- [ ] **7.3.3 Spacing & Layout**
  - 4px base unit
  - Max width containers
  - Responsive breakpoints

---

## Phase 8: Testing & Quality

### 8.1 Unit Tests
- [ ] **8.1.1 Parser Tests**
  - Test .pg/.cp file parsing
  - Test directive parsing
  - Test expression parsing

- [ ] **8.1.2 Compiler Tests**
  - Test HTIP generation
  - Test WASM output
  - Test error handling

- [ ] **8.1.3 Router Tests**
  - Test route matching
  - Test dynamic params
  - Test catch-all routes

### 8.2 Integration Tests
- [ ] **8.2.1 Component Tests**
  - Render tests
  - Event handling tests
  - State update tests

- [ ] **8.2.2 E2E Tests**
  - Navigation tests
  - Form submission tests
  - SSR tests

### 8.3 Property-Based Tests
- [ ] **8.3.1 Serialization Round-Trip**
  - Any data → Binary → Same data

- [ ] **8.3.2 Router Correctness**
  - File path → Route → Same file

- [ ] **8.3.3 Reactive Consistency**
  - State change → UI update

### 8.4 Benchmarks
- [ ] **8.4.1 vs React TodoMVC**
  - Bundle size comparison
  - Render performance
  - Memory usage

- [ ] **8.4.2 vs Next.js Blog**
  - TTFB comparison
  - LCP comparison
  - Build time

---

## Phase 9: Documentation

### 9.1 Getting Started
- [ ] **9.1.1 Quick Start (5 min)**
  - `dx create my-app`
  - `cd my-app && dx dev`
  - Edit a component
  - See it update

- [ ] **9.1.2 Tutorial (30 min)**
  - Build a todo app
  - Cover all major features
  - Deploy to production

### 9.2 Guides
- [ ] **9.2.1 Routing Guide**
- [ ] **9.2.2 Data Fetching Guide**
- [ ] **9.2.3 State Management Guide**
- [ ] **9.2.4 Styling Guide**
- [ ] **9.2.5 Deployment Guide**
- [ ] **9.2.6 Migration from React/Next.js**

### 9.3 API Reference
- [ ] **9.3.1 Auto-generate from Rust docs**
- [ ] **9.3.2 Component API docs**
- [ ] **9.3.3 CLI reference**

---

## Implementation Priority Order

### Week 1-2: Core Compiler
1. [ ] File format parser (.pg, .cp)
2. [ ] Directive parser ({#if}, {#each}, etc.)
3. [ ] Template → HTIP compiler
4. [ ] Basic CLI (dx dev, dx build)

### Week 3-4: Router & Navigation
5. [ ] File-based router
6. [ ] Client-side navigation
7. [ ] SSR engine
8. [ ] Link component

### Week 5-6: Components & Styling
9. [ ] Button, Input, Card components
10. [ ] Modal, Dropdown, Toast
11. [ ] dx-style integration (Tailwind → Binary CSS)
12. [ ] `dx add` command

### Week 7-8: State & Data
13. [ ] Reactive state system
14. [ ] useQuery/useMutation hooks
15. [ ] Form validation
16. [ ] Turso integration

### Week 9-10: DX Website
17. [ ] Landing page
18. [ ] Documentation structure
19. [ ] Blog setup
20. [ ] Deploy live

### Week 11-12: Polish & Launch
21. [ ] HMR refinement
22. [ ] Error handling
23. [ ] Performance optimization
24. [ ] Documentation completion

---

## Success Criteria

### Must Have (MVP)
- [ ] `dx create my-app` works and creates runnable project
- [ ] `dx dev` starts dev server with HMR
- [ ] `dx build` creates optimized production build
- [ ] File-based routing works (.pg files)
- [ ] Components work (.cp files)
- [ ] 10+ shadcn-style components available
- [ ] DX website running at dx-www.dev (or similar)

### Should Have (v1.0)
- [ ] Multi-language scripts (Rust + 1 other)
- [ ] Turso database integration
- [ ] All Svelte directives working
- [ ] DevTools extension (Chrome)
- [ ] 95% test coverage
- [ ] Complete documentation

### Nice to Have (v1.1+)
- [ ] All languages (Rust, Python, JS, Go)
- [ ] Firefox DevTools
- [ ] More deployment adapters
- [ ] Plugin system
- [ ] Theme marketplace

---

## Notes

- **File naming**: PascalCase for components (`Button.cp`), kebab-case or PascalCase for pages (`about.pg` or `AboutPage.pg`)
- **Binary compilation**: All .pg/.cp files compile to .dxob binary format
- **No npm dependencies**: Components are copied to project, not installed from npm
- **DX config file**: Uses DX Serializer human format (like provided example)
- **Tailwind classes**: Compiled to Binary Dawn CSS via dx-style

