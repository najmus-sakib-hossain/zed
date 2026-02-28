# DX CLI Self-Update System

## Features

### 1. Automatic Background Updates
- Daemon runs 24/7 checking for updates every hour
- Auto-applies updates from GitHub releases or DX API
- Ed25519 signature verification for security
- Delta patches for smaller downloads when available

### 2. Manual Update Commands
```bash
# Check and apply updates
dx self update

# Force update even if on latest
dx self update --force

# Skip confirmation prompt
dx self update -y

# Show CLI info
dx self info

# Uninstall CLI
dx self uninstall
```

### 3. Configuration
```bash
# Disable auto-updates
export DX_NO_AUTO_UPDATE=1

# Update check interval (default: 3600 seconds)
# Configured in daemon
```

## Architecture

### Update Flow
1. **Checker** - Fetches latest release from GitHub API
2. **Downloader** - Downloads binary with progress bar
3. **Signature** - Verifies Ed25519 signature
4. **Applier** - Atomic replacement with backup/restore

### File Structure
```
crates/cli/src/utils/update/
├── mod.rs          - Public API
├── checker.rs      - Update availability check
├── downloader.rs   - HTTP download with progress
├── daemon.rs       - Background update daemon
├── applier.rs      - Binary replacement logic
├── signature.rs    - Ed25519 verification
└── types.rs        - Data structures
```

## Security

- Ed25519 signature verification on all updates
- Atomic file replacement with automatic rollback
- Backup created before applying updates
- HTTPS-only downloads

## Integration

The daemon starts automatically in `main.rs`:
```rust
let daemon_config = utils::update::DaemonConfig::default();
let daemon_manager = utils::update::DaemonManager::new(daemon_config);
daemon_manager.start().await?;
```

Disable with: `DX_NO_AUTO_UPDATE=1 dx`

## Format System Integration

The self-update system works seamlessly with DX's 3-format system:
- **Human format** - Update docs stay on real disk (SELF_UPDATE.md)
- **LLM format** - Generated in `.dx/markdown/SELF_UPDATE.llm`
- **Machine format** - Binary in `.dx/markdown/SELF_UPDATE.machine`
