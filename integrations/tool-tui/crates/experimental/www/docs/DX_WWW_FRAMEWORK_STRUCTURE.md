# DX WWW Framework Structure & Coding Standards

## Overview

DX WWW is a binary-first, multi-language web framework that combines the best features of Vue.js, React, Next.js, and Svelte while leveraging WebAssembly for unprecedented performance.

## Core Philosophy

- **Binary-First**: Zero parse, zero GC, zero hydration
- **Multi-Language**: Write logic in Rust, Python, JavaScript, Go, or any WASM-compatible language
- **Atomic Styling**: Tailwind-like classes compiled to binary CSS
- **File-System Routing**: Convention over configuration
- **Type-Safe**: Full type safety across languages

---

## File Extensions

| Extension | Purpose | Example |
|-----------|---------|---------|
| `.pg` | Page components (routes) | `index.pg`, `about.pg` |
| `.cp` | Reusable components | `Button.cp`, `Card.cp` |
| `.dxob` | Compiled binary output | `Button.dxob` (generated) |
| `.sr` | Binary CSS styles | `theme.sr` (generated) |

---

## Component Syntax

### Basic Component Structure

```dx
<script lang="rust">
struct Props {
    message: String,
    count: i32,
}

let mut count = 0;

fn handleClick() {
    count += 1;
}
</script>

<component>
    <div class="container mx-auto p-4">
        <h1 class="text-2xl font-bold">{message}</h1>
        <p class="text-gray-600">Count: {count}</p>
        <button 
            class="bg-blue-500 hover:bg-blue-700 text-white px-4 py-2 rounded"
            onClick={handleClick}
        >
            Click me
        </button>
    </div>
</component>
```

### Page Structure

```dx
<script lang="python">
# Multi-language support - compiles to WASM
def fetch_data():
    return {"title": "Welcome", "posts": []}

data = fetch_data()
</script>

<page>
    <div class="min-h-screen bg-gray-50">
        <h1 class="text-4xl font-bold">{data.title}</h1>
        {#each data.posts as post}
            <article class="bg-white shadow rounded p-6">
                <h2 class="text-xl">{post.title}</h2>
            </article>
        {/each}
    </div>
</page>
```

---

## Language Support

### Multi-Language Scripts

You can use multiple languages in a single file:

```dx
<script lang="rust">
struct State {
    count: i32,
}
</script>

<script lang="python">
def process_data(data):
    return data.upper()
</script>

<script lang="go">
func calculateTotal(items []int) int {
    total := 0
    for _, item := range items {
        total += item
    }
    return total
}
</script>

<component>
    <div>{process_data(message)}</div>
</component>
```

### Default Language (JavaScript/TypeScript)

For JavaScript/TypeScript, no `lang` attribute needed:

```dx
<script>
const count = 0;
const handleClick = () => count++;
</script>

<component>
    <button onClick={handleClick}>{count}</button>
</component>
```

---

## Control Flow & Directives

### Conditionals

```dx
{#if condition}
    <div>Show this</div>
{:else if otherCondition}
    <div>Or this</div>
{:else}
    <div>Or that</div>
{/if}
```

### Loops

```dx
{#each items as item, index}
    <div class="item">
        <span>{index + 1}.</span>
        <span>{item.name}</span>
    </div>
{/each}
```

### Async/Await

```dx
{#await promise}
    <p class="loading">Loading...</p>
{:then data}
    <p class="success">Data: {data}</p>
{:catch error}
    <p class="error">Error: {error.message}</p>
{/await}
```

### Key Blocks

```dx
{#key value}
    <Component />
{/key}
```

---

## Reactivity

### Reactive Declarations

```dx
<script lang="rust">
let count = 0;

// Auto-recomputes when count changes
$: doubled = count * 2;

// Reactive statements
$: if (count > 10) {
    alert("Too high!");
}
</script>

<component>
    <p>Count: {count}</p>
    <p>Doubled: {doubled}</p>
</component>
```

---

## Component Communication

### Props

```dx
<!-- Button.cp -->
<script lang="rust">
struct Props {
    text: String,
    count: i32,
    disabled: bool,
}
</script>

<component>
    <button 
        class="btn"
        disabled={disabled}
    >
        {text} ({count})
    </button>
</component>

<!-- Usage -->
<Button text="Click me" count={5} disabled={false} />
```

### Events

```dx
<!-- Child.cp -->
<script lang="rust">
fn handleClick() {
    emit("click", { value: 42 });
}
</script>

<component>
    <button onClick={handleClick}>Click</button>
</component>

<!-- Parent.pg -->
<script lang="rust">
fn onChildClick(event) {
    console.log("Received:", event.value);
}
</script>

<page>
    <Child onClick={onChildClick} />
</page>
```

### Slots

```dx
<!-- Card.cp -->
<component>
    <div class="card border rounded p-4">
        <slot name="header" />
        <slot />  <!-- default slot -->
        <slot name="footer" />
    </div>
</component>

<!-- Usage -->
<Card>
    <h1 slot="header" class="text-xl font-bold">Title</h1>
    <p>Content goes here</p>
    <button slot="footer" class="btn-primary">Action</button>
</Card>
```

---

## Special Directives

### Two-Way Binding

```dx
<script lang="rust">
let value = "";
</script>

<component>
    <input bind:value={value} class="input" />
    <p>You typed: {value}</p>
</component>
```

### Conditional Classes

```dx
<script lang="rust">
let isActive = false;
let isDisabled = true;
</script>

<component>
    <button 
        class="btn"
        class:active={isActive}
        class:disabled={isDisabled}
    >
        Button
    </button>
</component>
```

### Custom Directives

```dx
<script lang="rust">
fn tooltip(node, text) {
    // Custom directive logic
}
</script>

<component>
    <button use:tooltip="Click me">Hover</button>
</component>
```

### Transitions

```dx
<script lang="rust">
let visible = true;
</script>

<component>
    {#if visible}
        <div transition:fade>
            Fading content
        </div>
    {/if}
</component>
```

---

## Styling

### Atomic Classes (Tailwind-like)

DX WWW uses atomic CSS classes that compile to binary CSS via `dx-style`:

```dx
<component>
    <div class="flex items-center justify-between p-4 bg-white shadow-lg rounded-lg">
        <h1 class="text-2xl font-bold text-gray-900">Title</h1>
        <button class="px-4 py-2 bg-blue-500 hover:bg-blue-700 text-white rounded">
            Click
        </button>
    </div>
</component>
```

### No Separate Style Blocks

Styles are embedded in class names. The framework automatically:
1. Extracts all atomic classes
2. Compiles them to binary CSS (`.sr`)
3. Optimizes and deduplicates
4. Serves as compressed binary

---

## Naming Conventions

### Component Files (PascalCase)

```
components/
├── Button.cp
├── Card.cp
├── UserProfile.cp
├── TodoList.cp
└── NavigationBar.cp
```

### Page Files (kebab-case)

```
pages/
├── index.pg
├── about.pg
├── user-profile.pg
├── blog-post.pg
└── contact-us.pg
```

### Folders (kebab-case)

```
components/
├── ui/
│   ├── Button.cp
│   └── Input.cp
├── layout/
│   ├── Header.cp
│   └── Footer.cp
└── forms/
    └── LoginForm.cp
```

---

## File Structure Order

Components and pages follow this structure:

```dx
<!-- 1. Script blocks (multiple languages allowed) -->
<script lang="rust">
// Rust code
</script>

<script lang="python">
# Python code
</script>

<!-- 2. Template (component or page tag) -->
<component>
    <!-- HTML with directives -->
</component>

<!-- 3. No separate style block - use atomic classes -->
```

---

## Key Features Summary

1. **Multi-language scripts**: `<script lang="rust|python|js|go|etc">`
2. **Svelte-style directives**: `{#if}`, `{#each}`, `{#await}`, `{#key}`
3. **Tailwind-like atomic classes**: Compiled to binary CSS via dx-style
4. **React-style props**: `<Button count={5} />`
5. **React-style events**: `onClick={handler}`
6. **Vue-style interpolation**: `{variable}` for expressions
7. **Binary compilation**: All `.pg` and `.cp` files compile to `.dxob`
8. **Reactive declarations**: `$: doubled = count * 2`
9. **Slots**: `<slot name="header" />`
10. **Special directives**: `bind:`, `class:`, `use:`, `transition:`

---

## Next Steps

This document defines the component syntax. The next phase will cover:
- Complete folder structure (pages, components, layouts, middleware, etc.)
- Routing conventions
- Data fetching patterns
- State management
- Build configuration
- Deployment structure

---

**Version**: 1.0.0  
**Last Updated**: January 31, 2026  
**Status**: Draft - Component Syntax Specification
