# GitHub Copilot & Kiro AI Instructions Setup

This document explains the AI instruction files created for the Zed editor project.

## Files Created

### 1. `.github/copilot-instructions.md`
**Purpose:** Instructs GitHub Copilot about project-specific context and best practices.

**Location:** `.github/copilot-instructions.md`

**What it covers:**
- Current system specifications (Windows 11, 7.3 GB RAM, 5 GB free disk, Git Bash)
- Why low-resource build strategy is mandatory for this system
- Low-resource build commands (Git Bash primary, PowerShell alternative)
- Icon CLI tool usage and available packs
- Rust coding guidelines
- GPUI framework concepts and limitations
- Testing best practices
- Pull request standards
- Windows-specific development notes and disk space management

**How GitHub Copilot uses it:**
GitHub Copilot automatically reads instructions from `.github/copilot-instructions.md` when providing code suggestions and chat responses in this repository.

### 2. `.kiro/steering/project-context.md`
**Purpose:** Provides Kiro AI with essential project context for all interactions.

**Location:** `.kiro/steering/project-context.md`

**What it covers:**
- Current system environment (Windows 11, 7.3 GB RAM, 5 GB free disk, Git Bash)
- Why this system requires special build configuration
- Critical build configuration with Git Bash commands
- Icon CLI tool reference and examples
- GPUI framework capabilities and limitations
- Testing commands
- Windows development notes
- Common build issues and solutions for this system
- Proactive suggestion guidelines

**How Kiro uses it:**
The file includes `inclusion: auto` frontmatter, which means Kiro automatically loads this context in every session. This ensures consistent, project-aware assistance.

## Key Information Communicated

### 1. Large Codebase Build Strategy

Both AI assistants now understand that:
- This is a large Rust project running on a **low-resource Windows 11 system**
- **System specs:** 7.3 GB RAM, 5 GB free disk space (95% full on F: drive)
- **Primary shell:** Git Bash (bash commands are default)
- Standard builds **will fail** on this system due to RAM and disk constraints
- Builds **must use** single-job compilation with incremental artifacts
- Always target specific crates (`-p zed`) instead of the full workspace
- Use `--locked` flag to respect Cargo.lock
- Monitor disk space before and during builds

**Example build command (Git Bash):**
```bash
export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"
cargo run -p zed --locked
```

### 2. Icon CLI Tool

Both AI assistants know about the `icon` CLI tool installed on your system:

**Quick reference:**
```bash
# Search for icons
icon search home
icon s arrow --limit 20

# Export to directory
icon export search ./icons --pack lucide

# Export to desktop assets
icon desktop search:lucide home:solar
icon d menu:heroicons

# List packs
icon packs
```

**Popular packs:** lucide, solar, heroicons, feather, material-symbols, tabler, carbon

### 3. GPUI Framework Awareness

Both AI assistants understand GPUI's capabilities and limitations:

**Supported:**
- Linear gradients, box shadows, fade/slide animations, flexbox layout

**Not supported:**
- Radial/mesh gradients, backdrop blur, SVG animations, CSS transforms

They can now suggest appropriate workarounds when you request unsupported features.

## Benefits

### For GitHub Copilot:
- More accurate code suggestions that respect build constraints
- Understands the specific system limitations (7.3 GB RAM, 5 GB disk)
- Provides Git Bash commands by default (not PowerShell)
- Proper icon CLI usage in suggestions
- GPUI-aware UI component suggestions
- Follows project coding standards automatically
- Warns about disk space issues

### For Kiro:
- Consistent project context in every session
- Aware of critical system constraints (RAM, disk space, shell)
- Provides Git Bash syntax as primary option
- Proactive suggestions for icon CLI usage
- Build command recommendations that work on this specific system
- Awareness of framework limitations
- Can diagnose common build failures specific to this environment

## Verification

Both files are now active and include specific system information:

**System Details Communicated:**
- OS: Windows 11 Pro
- Shell: Git Bash (bash)
- RAM: 7.3 GB
- Disk: 5 GB free on F: (95% used)
- Build path: F:\Desktop\

**GitHub Copilot:** Will use `.github/copilot-instructions.md` automatically  
**Kiro:** Will load `.kiro/steering/project-context.md` automatically (due to `inclusion: auto`)

Both AIs now understand this is a low-resource system and will provide appropriate commands and warnings.

## Maintenance

These instruction files should be updated when:
- Build strategy changes
- New icon packs are added
- GPUI capabilities expand
- New project-wide patterns emerge

## References

- Build details: `BUILD.md`
- Icon CLI manual: `ICONS.md`
- Coding guidelines: `.rules`
- Agent guidelines: `AGENTS.md`

---

**Created:** February 25, 2026
**Purpose:** Provide professional-grade AI instructions for the Zed editor project
