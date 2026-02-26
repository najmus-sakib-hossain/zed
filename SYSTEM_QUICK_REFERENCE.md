# System Quick Reference - Zed Editor Build Environment

## Current System Specifications

| Component | Details |
|-----------|---------|
| **OS** | Windows 11 Pro |
| **Shell** | Git Bash (bash) |
| **RAM** | 7,471 MB (~7.3 GB) |
| **Disk Free** | ~5 GB on F: drive |
| **Disk Usage** | 95% full - **CRITICAL** |
| **Build Path** | F:\Desktop\ |
| **Target Dir** | F:\Desktop\target\ |

## Why Low-Resource Builds Are Required

❌ **Standard Cargo Build Would:**
- Consume 10-15+ GB RAM during linking → **Out of Memory**
- Require 20+ GB temporary disk space → **Disk Full**
- Use multi-job compilation → **Memory spikes crash build**

✅ **Low-Resource Build Strategy:**
- Single-job compilation (`CARGO_BUILD_JOBS=1`)
- Incremental artifacts reuse
- Target specific crate only (`-p zed`)
- Reduced codegen units

## Essential Build Commands (Git Bash)

### Set Environment Variables (One-Time Setup)

Add to `~/.bashrc` for persistence:

```bash
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"
```

Or set for current session:

```bash
export CARGO_BUILD_JOBS="1" CARGO_INCREMENTAL="1" CARGO_PROFILE_DEV_CODEGEN_UNITS="1" CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"
```

### Build Zed Editor

```bash
# Standard build (ALWAYS use this)
cargo run -p zed --locked

# Build without running
cargo build -p zed --locked

# Run already-built executable (no rebuild)
./target/debug/zed.exe
```

### Check System Resources

```bash
# Check disk space
df -h .

# Check memory
free -h

# Monitor during build
watch -n 5 'df -h . && free -h'
```

## Icon CLI Commands

```bash
# Search for icons
icon search home
icon s arrow --limit 20

# Export to directory
icon export search ./icons --pack lucide --limit 5

# Export to desktop assets (apps/desktop/assets/icons/)
icon desktop search:lucide home:solar
icon d menu:heroicons

# List available packs
icon packs
```

### Popular Icon Packs

- `lucide` - Modern, clean (1000+)
- `solar` - Bold, linear (7000+)
- `heroicons` - Tailwind UI (500+)
- `feather` - Simple, beautiful (280+)
- `material-symbols` - Google Material (10,000+)

## Common Build Issues

### Out of Memory

**Symptoms:** Build crashes, linker fails, system freezes

**Solution:**
```bash
# Ensure environment variables are set
echo $CARGO_BUILD_JOBS  # Should output: 1

# If not set, export them
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"

# Rebuild
cargo run -p zed --locked
```

### Disk Full

**Symptoms:** `LNK1201` error, "No space left on device"

**Solution:**
```bash
# Check disk space
df -h .

# Move target directory to C: drive
export CARGO_TARGET_DIR="C:\zed-target"

# Clean old artifacts (only if necessary)
rm -rf F:\Desktop\target

# Rebuild
cargo run -p zed --locked
```

### rc.exe Not Found

**Symptoms:** "rc.exe" selection fails

**Solution:**
```bash
# Set RC toolkit path (adjust SDK version)
export ZED_RC_TOOLKIT_PATH="C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64"

# Rebuild
cargo run -p zed --locked
```

### Long Path Errors

**Symptoms:** "path too long" errors

**Solution:**
```bash
# Enable long paths in Git
git config --system core.longpaths true

# Enable in Windows (requires admin PowerShell + reboot)
# New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

## Disk Space Management

### Current Status
- **Total:** 98 GB
- **Used:** 93 GB (95%)
- **Free:** 5 GB ⚠️

### Recommendations

1. **Move target directory to C: drive:**
   ```bash
   export CARGO_TARGET_DIR="C:\zed-target"
   ```

2. **Clean old build artifacts (if safe):**
   ```bash
   # Only do this if you have a backup or can rebuild
   cargo clean -p zed
   ```

3. **Monitor disk usage:**
   ```bash
   df -h .
   ```

4. **Free up space before builds:**
   - Delete temporary files
   - Clear browser cache
   - Remove old downloads

## Testing Commands

```bash
# Run clippy (use project script)
./script/clippy

# Run tests for specific crate
cargo test -p zed --locked

# Run specific test
cargo test -p zed test_name --locked
```

## Git Bash vs PowerShell

**Default:** Use Git Bash commands

**Git Bash:**
```bash
export VAR="value"
echo $VAR
```

**PowerShell (if needed):**
```powershell
$env:VAR = "value"
echo $env:VAR
```

## AI Instructions Files

Both GitHub Copilot and Kiro now understand this system:

- **GitHub Copilot:** `.github/copilot-instructions.md`
- **Kiro:** `.kiro/steering/project-context.md`
- **Documentation:** `COPILOT_SETUP.md`

They will provide appropriate commands and warnings based on system constraints.

## Quick Checklist Before Building

- [ ] Environment variables set (`echo $CARGO_BUILD_JOBS` should be `1`)
- [ ] Disk space available (`df -h .` should show >2 GB free)
- [ ] Using `-p zed` flag (not building entire workspace)
- [ ] Using `--locked` flag (respects Cargo.lock)
- [ ] Git Bash shell active (not PowerShell)

## Emergency: Build Completely Fails

```bash
# 1. Check disk space
df -h .

# 2. Move target to C: drive
export CARGO_TARGET_DIR="C:\zed-target"

# 3. Verify environment variables
echo $CARGO_BUILD_JOBS
echo $CARGO_INCREMENTAL

# 4. Set if not set
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"

# 5. Try build again
cargo run -p zed --locked

# 6. If still fails, check system resources
free -h
df -h .
```

## Documentation References

- **Build Instructions:** `BUILD.md`
- **Icon CLI Manual:** `ICONS.md`
- **Coding Guidelines:** `.rules`
- **Agent Guidelines:** `AGENTS.md`
- **AI Setup:** `COPILOT_SETUP.md`

---

**Last Updated:** February 25, 2026  
**System:** Windows 11 Pro, 7.3 GB RAM, 5 GB free disk, Git Bash
