
# DX Project Manager

Workspace and task management for monorepos.

## Installation

```bash
cargo build --release -p dx-js-project-manager ```


## Usage


```bash

# Run a task

dx-project-manager run build

# Run task in specific packages

dx-project-manager run build --filter "packages/*"

# Show affected packages

dx-project-manager affected build

# Show task dependency graph

dx-project-manager graph

# List workspaces

dx-project-manager list ```

## Features

### Workspace Detection

- Automatic workspace discovery from package.json
- Glob pattern support for workspace paths
- Nested workspace support

### Task Execution

- Dependency-ordered execution
- Parallel execution where possible
- Task caching with content hashing
- Incremental builds

### Affected Detection

- Git-based change detection
- Dependency graph analysis
- Only run tasks for changed packages

## Configuration

### package.json

```json
{ "name": "my-monorepo", "workspaces": [ "packages/*", "apps/*"
]
}
```

### dx-project.json

```json
{ "tasks": { "build": { "dependsOn": ["^build"], "outputs": ["dist/**"]
}, "test": { "dependsOn": ["build"], "cache": true }
}
}
```

## Task Dependencies

+---------+---------+
| Syntax  | Meaning |
+=========+=========+
| `build` | Task    |
+---------+---------+



## Caching

Tasks are cached based on: -Input file hashes -Environment variables -Task configuration -Dependency outputs Cache stored in `.dx/cache/` with binary format for fast loading.

## Architecture

+--------+---------+
| Format | Purpose |
+========+=========+
| BWM    | Binary  |
+--------+---------+



## License

MIT OR Apache-2.0
