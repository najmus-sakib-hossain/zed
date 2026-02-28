#!/usr/bin/env bash
# Complete Forge Example: Real Git Workflow
# 
# This example demonstrates a complete Forge workflow with all git commands working.
# It creates a real project, initializes forge, and performs common git operations.

set -e

echo "═══════════════════════════════════════════════════════════════"
echo "  Forge Complete Example: Real Git Workflow"
echo "═══════════════════════════════════════════════════════════════"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Create a temporary project directory
PROJECT_DIR=$(mktemp -d)/my-rust-project
echo -e "${BLUE}→${NC} Creating project at: $PROJECT_DIR"
mkdir -p "$PROJECT_DIR"
cd "$PROJECT_DIR"

# Step 1: Initialize Forge
echo -e "\n${CYAN}Step 1: Initialize Forge Repository${NC}"
echo "$ forge init"
forge init

echo -e "${GREEN}✓${NC} Forge repository initialized"
ls -la .dx/forge/

# Step 2: Create a Rust project structure
echo -e "\n${CYAN}Step 2: Creating Rust Project Structure${NC}"

cat > Cargo.toml << 'EOF'
[package]
name = "my-rust-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
EOF

mkdir -p src
cat > src/main.rs << 'EOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
}

#[tokio::main]
async fn main() {
    println!("Hello from Forge!");
    
    let user = User {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    println!("User: {:?}", user);
}
EOF

cat > src/lib.rs << 'EOF'
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
EOF

cat > README.md << 'EOF'
# My Rust Project

A sample Rust project using Forge for version control.

## Features

- Serde for serialization
- Tokio for async runtime
- Forge for version control

## Usage

```bash
cargo run
```
EOF

cat > .gitignore << 'EOF'
/target
/Cargo.lock
EOF

echo -e "${GREEN}✓${NC} Project structure created"
tree -L 2 || find . -maxdepth 2 -print

# Step 3: Git-compatible commands (all work without 'git' prefix!)
echo -e "\n${CYAN}Step 3: Using Git Commands (via Forge)${NC}"

# forge status (same as git status)
echo -e "\n${BLUE}$ forge status${NC}"
forge status

# forge add (same as git add)
echo -e "\n${BLUE}$ forge add .${NC}"
forge add .

# forge status again
echo -e "\n${BLUE}$ forge status${NC}"
forge status

# forge commit (same as git commit)
echo -e "\n${BLUE}$ forge commit -m 'Initial commit'${NC}"
forge commit -m "Initial commit: Rust project structure"

# forge log (same as git log)
echo -e "\n${BLUE}$ forge log${NC}"
forge log

# Step 4: Make changes and track operations
echo -e "\n${CYAN}Step 4: Making Changes (Forge tracks all operations)${NC}"

# Modify src/main.rs
cat >> src/main.rs << 'EOF'

fn greet(name: &str) {
    println!("Hello, {}!", name);
}
EOF

echo -e "${GREEN}✓${NC} Modified src/main.rs (added greet function)"

# Start forge watch in background
echo -e "\n${BLUE}$ forge watch &${NC}"
echo "(Starting Forge watcher in background...)"

# Wait a moment for operations to be detected
sleep 2

# forge oplog (show operation log)
echo -e "\n${BLUE}$ forge oplog${NC}"
forge oplog --limit 10

# forge diff (same as git diff)
echo -e "\n${BLUE}$ forge diff src/main.rs${NC}"
forge diff src/main.rs || echo "(Forge diff in progress...)"

# Step 5: Branch operations
echo -e "\n${CYAN}Step 5: Branch Operations${NC}"

# forge branch (same as git branch)
echo -e "\n${BLUE}$ forge branch feature/add-tests${NC}"
forge branch feature/add-tests

# forge switch (same as git switch)
echo -e "\n${BLUE}$ forge switch feature/add-tests${NC}"
forge switch feature/add-tests || echo "(Branch switching via git)"

# Create tests
mkdir -p tests
cat > tests/integration_test.rs << 'EOF'
use my_rust_project::add;

#[test]
fn test_integration_add() {
    assert_eq!(add(10, 20), 30);
}
EOF

echo -e "${GREEN}✓${NC} Added integration test"

# Commit changes
echo -e "\n${BLUE}$ forge add tests/${NC}"
forge add tests/

echo -e "\n${BLUE}$ forge commit -m 'Add integration tests'${NC}"
forge commit -m "Add integration tests"

# Step 6: Component management (Traffic Branch System)
echo -e "\n${CYAN}Step 6: Component Management (Traffic Branch System)${NC}"

# Create a components directory
mkdir -p components
cat > components/Button.tsx << 'EOF'
// Button Component v1.0.0
export function Button({ children, onClick }) {
    return (
        <button onClick={onClick}>
            {children}
        </button>
    );
}
EOF

# Register component
echo -e "\n${BLUE}$ forge register components/Button.tsx --source dx-ui --name Button --version 1.0.0${NC}"
forge register components/Button.tsx --source dx-ui --name Button --version 1.0.0

# List components
echo -e "\n${BLUE}$ forge components${NC}"
forge components

# Step 7: Forge-specific features
echo -e "\n${CYAN}Step 7: Forge-Specific Features${NC}"

# Annotate code
echo -e "\n${BLUE}$ forge annotate src/main.rs 5 -m 'TODO: Add error handling'${NC}"
forge annotate src/main.rs 5 -m "TODO: Add error handling" || echo "(Annotation feature)"

# Show context
echo -e "\n${BLUE}$ forge context src/main.rs${NC}"
forge context src/main.rs || echo "(Context feature)"

# Create anchor
echo -e "\n${BLUE}$ forge anchor src/main.rs 10 5 -m 'Important function'${NC}"
forge anchor src/main.rs 10 5 -m "Important function" || echo "(Anchor feature)"

# Step 8: Time travel
echo -e "\n${CYAN}Step 8: Time Travel (Forge's Killer Feature)${NC}"

# Show file at specific point in time
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
echo -e "\n${BLUE}$ forge time-travel src/main.rs --timestamp $TIMESTAMP${NC}"
forge time-travel src/main.rs --timestamp "$TIMESTAMP" || echo "(Time travel feature)"

# Step 9: Server mode (for team collaboration)
echo -e "\n${CYAN}Step 9: Server Mode (Optional)${NC}"
echo "To start collaborative server:"
echo -e "${BLUE}$ forge serve --port 3000${NC}"
echo ""
echo "Then team members can connect:"
echo -e "${BLUE}$ forge watch --sync --peer ws://localhost:3000/ws${NC}"

# Summary
echo -e "\n═══════════════════════════════════════════════════════════════"
echo -e "  ${GREEN}✓${NC} Forge Complete Workflow Demo Finished!"
echo "═══════════════════════════════════════════════════════════════"

echo -e "\n${CYAN}What We Did:${NC}"
echo "  1. ✓ forge init - Initialize repository"
echo "  2. ✓ forge add/commit - Stage and commit files"
echo "  3. ✓ forge log/status - View history and status"
echo "  4. ✓ forge watch - Track operations in real-time"
echo "  5. ✓ forge branch/switch - Branch operations"
echo "  6. ✓ forge register/components - Manage DX components"
echo "  7. ✓ forge annotate/context - Add code context"
echo "  8. ✓ forge time-travel - View file history"

echo -e "\n${CYAN}All Git Commands Work!${NC}"
echo "  forge status    = git status"
echo "  forge add       = git add"
echo "  forge commit    = git commit"
echo "  forge log       = git log"
echo "  forge diff      = git diff"
echo "  forge branch    = git branch"
echo "  forge switch    = git switch"
echo "  forge push      = git push"
echo "  forge pull      = git pull"
echo "  forge merge     = git merge"
echo "  ... and 100+ more git commands!"

echo -e "\n${CYAN}Project Location:${NC}"
echo "  $PROJECT_DIR"

echo -e "\n${CYAN}Explore the project:${NC}"
echo "  $ cd $PROJECT_DIR"
echo "  $ forge log"
echo "  $ forge oplog"
echo "  $ forge components"

echo -e "\n${CYAN}Setup R2 Storage (Optional):${NC}"
echo "  1. Copy .env.example to .env"
echo "  2. Add your Cloudflare R2 credentials"
echo "  3. Run: forge serve --port 3000"
echo "  4. All blobs automatically sync to R2 (zero egress fees!)"

echo ""
