
# DX-Forge Component Injection Examples

## Overview

This document provides practical examples of using the DX-Forge component injection system to automatically fetch, cache, and inject reusable components into your project.

## Basic Concepts

### What is Component Injection?

Component injection automatically: -Detects DX component usage in your code (`<dxButton>`, `<dxModal>`, etc.) -Fetches the component from Cloudflare R2 storage -Verifies integrity with SHA-256 hashing -Caches locally for performance -Injects into your project

### Component Structure

@tree:components[]

## Example 1: Basic Button Component

### Usage in Your Code

```tsx
// src/App.tsx import React from 'react';
export function App() { return ( <div> <dxButton variant="primary" size="lg"> Click Me </dxButton> </div> );
}
```

### Automatic Injection

When you save the file, DX-Forge: -Detects the `<dxButton>` tag -Fetches from R2: `components/dxButton/component.tsx` -Verifies SHA-256: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` -Caches to: `.dx-cache/dxButton/` -Injects import: `import { dxButton } from '.dx-cache/dxButton/component';`

### Result

```tsx
// src/App.tsx (after injection)
import React from 'react';
import { dxButton as Button } from '.dx-cache/dxButton/component';
export function App() { return ( <div> <Button variant="primary" size="lg"> Click Me </Button> </div> );
}
```

## Example 2: Modal with Props

### Component Definition (R2 Storage)

Location: `s3://forge-blobs/components/dxModal/component.tsx`
```tsx
export interface DxModalProps { isOpen: boolean;
onClose: () => void;
title: string;
children: React.ReactNode;
}
export function dxModal({ isOpen, onClose, title, children }: DxModalProps) { if (!isOpen) return null;
return ( <div className="dx-modal-overlay" onClick={onClose}> <div className="dx-modal-content" onClick={e => e.stopPropagation()}> <div className="dx-modal-header"> <h2>{title}</h2> <button onClick={onClose}>×</button> </div> <div className="dx-modal-body">{children}</div> </div> </div> );
}
```

### Usage

```tsx
// src/UserProfile.tsx import React, { useState } from 'react';
export function UserProfile() { const [showModal, setShowModal] = useState(false);
return ( <> <button onClick={() => setShowModal(true)}> Edit Profile </button> <dxModal isOpen={showModal}
onClose={() => setShowModal(false)}
title="Edit Profile"
> <form> <input placeholder="Name" /> <button type="submit">Save</button> </form> </dxModal> </> );
}
```

## Example 3: Multiple Components

### Code

```tsx
// src/Dashboard.tsx export function Dashboard() { return ( <dxLayout> <dxHeader> <dxLogo /> <dxNav items={navItems} /> </dxHeader> <dxSidebar> <dxMenu items={menuItems} /> </dxSidebar> <dxMain> <dxCard title="Statistics"> <dxChart data={stats} /> </dxCard> </dxMain> </dxLayout> );
}
```

### Injection Process

DX-Forge detects 7 components: -`dxLayout` -`dxHeader` -`dxLogo` -`dxNav` -`dxSidebar` -`dxMenu` -`dxCard` -`dxMain` -`dxChart` Fetches in parallel (max 10 concurrent):
```rust
// src/injection.rs let handles: Vec<_> = components.iter()
.map(|comp| tokio::spawn(fetch_component(comp)))
.collect();
```
Caches to: @tree:.dx-cache[]

## Example 4: Custom Component Upload

### Creating a Component

File: `my-components/dxCustomButton/component.tsx`
```tsx
export interface DxCustomButtonProps { onClick: () => void;
variant: 'primary' | 'secondary';
children: React.ReactNode;
}
export function dxCustomButton({ onClick, variant, children }: DxCustomButtonProps) { return ( <button className={`dx-button dx-button-${variant}`}
onClick={onClick}
> {children}
</button> );
}
```

### Upload to R2

```rust
use dx_forge::storage::r2::{R2Storage, R2Config};
use dx_forge::storage::blob::Blob;


#[tokio::main]


async fn main() -> Result<()> { // Load config let config = R2Config::from_env()?;
let storage = R2Storage::new(config)?;
// Read component file let content = std::fs::read("my-components/dxCustomButton/component.tsx")?;
// Create blob let blob = Blob { id: "dxCustomButton-component".to_string(), data: content, content_type: "text/tsx".to_string(), metadata: Default::default(), };
// Upload let key = storage.upload_component("Button", "dxCustomButton", &blob).await?;
println!("Uploaded to: {}", key);
Ok(())
}
```

## Example 5: Sync Local Components to R2

### Scenario

You have local components and want to make them available via R2.

### Code

```rust
use dx_forge::storage::r2::{R2Storage, R2Config};


#[tokio::main]


async fn main() -> Result<()> { let config = R2Config::from_env()?;
let storage = R2Storage::new(config)?;
// Local components let local_components = vec![ "dxButton".to_string(), "dxModal".to_string(), "dxCard".to_string(), ];
// Sync (uploads missing components)
storage.sync_components( "Button", &local_components,
|comp| println!("Downloading: {}", comp),
|comp| println!("Uploading: {}", comp), ).await?;
println!("Sync complete!");
Ok(())
}
```

## Example 6: Component Metadata

### meta.json

```json
{ "name": "dxButton", "version": "1.2.0", "author": "DX-Forge Team", "description": "Customizable button component with variants", "props": { "variant": { "type": "string", "values": ["primary", "secondary", "danger"], "default": "primary"
}, "size": { "type": "string", "values": ["sm", "md", "lg"], "default": "md"
}, "disabled": { "type": "boolean", "default": false }
}, "dependencies": { "react": "^18.0.0"
}, "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}
```

### Access in Code

```rust
use serde_json::Value;
let meta_json = std::fs::read_to_string(".dx-cache/dxButton/meta.json")?;
let meta: Value = serde_json::from_str(&meta_json)?;
println!("Component: {}", meta["name"]);
println!("Version: {}", meta["version"]);
println!("Hash: {}", meta["hash"]);
```

## Example 7: Integration with Build Tools

### Vite Plugin

```typescript
// vite-plugin-dx-forge.ts import { Plugin } from 'vite';
import { execSync } from 'child_process';
export function dxForgePlugin(): Plugin { return { name: 'dx-forge-inject', async transform(code: string, id: string) { if (id.endsWith('.tsx') || id.endsWith('.ts')) {
// Run DX-Forge analysis const result = execSync(`forge analyze ${id}`, { encoding: 'utf-8' });
// Inject imports if needed if (result.includes('injection_required')) { return { code: injectImports(code), map: null };
}
}
return null;
}
};
}
```

### Usage

```typescript
// vite.config.ts import { defineConfig } from 'vite';
import { dxForgePlugin } from './vite-plugin-dx-forge';
export default defineConfig({ plugins: [dxForgePlugin()]
});
```

## Configuration

###.forgerc.json

```json
{ "componentSources": { "r2": { "enabled": true, "bucket": "forge-blobs", "prefix": "components/"
}, "npm": { "enabled": false, "registry": "https://registry.npmjs.org"
}
}, "cache": { "directory": ".dx-cache", "ttl": 86400, "maxSize": 1073741824 }, "injection": { "autoInject": true, "verifyHash": true, "retries": 3 }
}
```

## CLI Commands

### Analyze File

```bash
forge analyze src/App.tsx


# Output: Found 3 DX components: dxButton, dxModal, dxCard


```

### Fetch Component

```bash
forge fetch dxButton


# Downloads and caches dxButton


```

### List Cached Components

```bash
forge list --cached


# dxButton (v1.2.0)



# dxModal (v0.9.1)



# dxCard (v1.0.0)


```

### Clear Cache

```bash
forge cache clear


# Removed 15 cached components


```

## Troubleshooting

### Component Not Found

```
Error: Component 'dxNewComponent' not found in R2 ```
Solution: Upload component first or check spelling.


### Hash Mismatch


```
Error: SHA-256 hash mismatch for 'dxButton' Expected: abc123...
Got: def456...
```
Solution: Component corrupted during transfer. Clear cache and re-fetch.


### Circular Dependencies


```
Error: Circular dependency detected: dxA -> dxB -> dxA ```
Solution: Refactor components to remove circular reference.

## Best Practices

- Version Components: Use semantic versioning in `meta.json`
- Document Props: Add TypeScript interfaces for all components
- Test Before Upload: Verify components work locally first
- Use Hash Verification: Always enable `verifyHash: true`
- Optimize Bundle Size: Only inject components used
- Cache Strategy: Set appropriate TTL based on update frequency

## Resources

- R2 Storage API
- Injection Manager
- Component Schema
