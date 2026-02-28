Here's a systematic program for analyzing the Zed codebase from your integrations folder:

## Zed Codebase Analysis Program

### Phase 1: Identify Lightweight vs Heavy Crates

**Step 1: Count dependencies**
```bash
# For each crate you're interested in:
wc -l integrations/zed/crates/CRATE_NAME/Cargo.toml
cat integrations/zed/crates/CRATE_NAME/Cargo.toml | grep "workspace = true" | wc -l
```

**Step 2: Check lines of code**
```bash
find integrations/zed/crates/CRATE_NAME -name "*.rs" | xargs wc -l | tail -1
```

**Step 3: Identify dependency chains**
```bash
# Look for these heavy dependencies in Cargo.toml:
# - project (huge, LSP integration)
# - workspace (huge, window management)
# - editor (huge, text editing)
# - language (medium, tree-sitter)
# - client (medium, networking)
# - call (medium, video/audio)
```

### Phase 2: Categorize Crates

**Lightweight (< 10 deps, < 2000 lines):**
- `theme` - Theme system
- `picker` - Fuzzy picker UI
- `menu` - Context menus
- `component` - Base components
- `icons` - Icon system
- `breadcrumbs` - Breadcrumb navigation

**Medium (10-20 deps, 2000-5000 lines):**
- `ui` - UI component library
- `buffer_diff` - Git diff manager
- `markdown` - Markdown rendering
- `file_icons` - File type icons
- `fuzzy` - Fuzzy matching

**Heavy (20+ deps, 5000+ lines):**
- `editor` - Text editor (51,709 lines)
- `workspace` - Window/pane management
- `project` - Project/LSP integration
- `collab_ui` - Collaboration UI
- `language` - Language support

### Phase 3: Extract Reusable Components

**For each lightweight crate:**

1. **Read the main file**
```bash
cat integrations/zed/crates/CRATE_NAME/src/CRATE_NAME.rs | head -100
```

2. **Check dependencies**
```bash
cat integrations/zed/crates/CRATE_NAME/Cargo.toml
```

3. **Identify Zed-specific dependencies to remove:**
   - `settings` → Replace with your config system
   - `util` → Copy needed utilities
   - `assets` → Replace with your asset system
   - `zed_actions` → Remove or replace
   - `workspace` → Remove (heavy)
   - `project` → Remove (heavy)
   - `client` → Remove (heavy)

4. **List public API**
```bash
grep "pub struct\|pub enum\|pub fn\|pub trait" integrations/zed/crates/CRATE_NAME/src/*.rs
```

### Phase 4: Dependency Analysis

**Check what each crate depends on:**
```bash
cat integrations/zed/crates/CRATE_NAME/Cargo.toml | grep -A 1 "\[dependencies\]" | grep "workspace = true"
```

**Red flags (avoid these):**
- Depends on `workspace`
- Depends on `project`
- Depends on `editor`
- Depends on `language` (unless you need tree-sitter)
- Depends on `client` (networking)
- Depends on `call` (video/audio)
- Depends on `lsp` (language server)

**Green flags (safe to use):**
- Only depends on `gpui`
- Depends on `theme`
- Depends on `ui`
- Depends on `collections`
- Depends on `util` (small utilities)

### Phase 5: Create Reusability Matrix

**For each crate, answer:**

1. **Lines of code?** `find integrations/zed/crates/X -name "*.rs" | xargs wc -l`
2. **Number of dependencies?** `cat Cargo.toml | grep workspace | wc -l`
3. **Depends on heavy crates?** Check for workspace/project/editor
4. **Self-contained?** Can it work without Zed infrastructure?
5. **GPUI-only?** Does it only need GPUI + theme?

### Phase 6: Extraction Strategy

**For lightweight crates you want to use:**

1. Copy the crate folder to your project
2. Update Cargo.toml to remove Zed-specific deps
3. Create adapter traits for:
   - Settings → Your config system
   - Theme → Your theme system (or keep Zed's)
   - Assets → Your asset loading
4. Replace `workspace = true` with actual versions
5. Test compilation with minimal deps

### Quick Reference: What to Use

**✅ Safe to extract (lightweight):**
- `ui` - 40+ reusable components
- `picker` - Fuzzy picker
- `menu` - Context menus
- `theme` - Theme system
- `icons` - Icon components
- `component` - Base component traits
- `breadcrumbs` - Breadcrumb UI

**⚠️ Medium effort (need adaptation):**
- `buffer_diff` - Git diff (remove language dep)
- `markdown` - Markdown rendering
- `fuzzy` - Fuzzy matching
- `file_icons` - File icons

**❌ Avoid (too heavy):**
- `editor` - 51K lines, 50+ deps
- `workspace` - 30+ deps, window management
- `project` - LSP, file watching, heavy
- `collab_ui` - Needs client/networking
- `language` - Tree-sitter, heavy

### Analysis Commands

Run these to analyze any crate:

```bash
# Lines of code
find integrations/zed/crates/CRATE_NAME -name "*.rs" | xargs wc -l | tail -1

# Dependency count
grep "workspace = true" integrations/zed/crates/CRATE_NAME/Cargo.toml | wc -l

# Check for heavy deps
grep -E "workspace|project|editor|language|client|call" integrations/zed/crates/CRATE_NAME/Cargo.toml

# List public API
grep -r "pub struct\|pub enum\|pub fn" integrations/zed/crates/CRATE_NAME/src/ | head -20

# Check imports
grep -r "use " integrations/zed/crates/CRATE_NAME/src/*.rs | grep -v "^//" | head -20
```

This program gives you a systematic way to evaluate any Zed crate for reusability in your GPUI desktop app without needing to build the entire Zed codebase.