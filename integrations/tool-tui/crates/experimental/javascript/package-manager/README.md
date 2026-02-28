
# DX Package Manager

⚠️ Early Development (v0.0.1) - Many features are experimental. APIs may change. npm-compatible package manager with O(1) cached installs.

## Installation

```bash
cargo build --release -p dx-pkg-cli ```


## Usage


```bash

# Initialize project

dx init

# Add packages

dx add lodash dx add react@18 dx add typescript --dev

# Install all dependencies

dx install

# Remove package

dx remove lodash

# Update packages

dx update

# Run scripts

dx run build dx run test

# Execute commands

dx exec eslint .
dx dlx create-react-app my-app

# List packages

dx list dx outdated

# Security

dx audit ```

## Features

### Package Installation

- npm registry compatible
- Semantic versioning (^, ~, >=, ranges)
- Lockfile generation with SHA-512 integrity
- Content-addressable storage
- Symlink-based linking (O(1) warm installs)

### Workspaces

- Monorepo support via `workspaces` field
- `workspace:` protocol for local dependencies
- `--filter` flag for targeted script execution

### Private Registries

- `.npmrc` configuration support
- Scoped registry configuration
- Token and basic authentication

### Lifecycle Scripts

- preinstall, install, postinstall
- prepare (for git dependencies)
- Custom scripts via `dx run`

### Peer Dependencies

- Automatic peer dependency installation
- Conflict detection and warnings

## Cache Architecture

@tree:~/.dx[] First install downloads and extracts packages. Subsequent installs symlink to cached layouts.

## Configuration

### package.json

```json
{ "name": "my-project", "version": "1.0.0", "dependencies": { "lodash": "^4.17.21"
}, "devDependencies": { "typescript": "^5.0.0"
}, "workspaces": [ "packages/*"
], "scripts": { "build": "tsc", "test": "dx-test"
}
}
```

###.npmrc

```ini
registry=https://registry.npmjs.org/ @myorg:registry=https://npm.myorg.com/ //npm.myorg.com/:_authToken=${NPM_TOKEN}
```

## License

MIT OR Apache-2.0
