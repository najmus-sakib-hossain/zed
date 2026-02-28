#!/bin/bash
# Profile-Guided Optimization (PGO) build script for dx-style
# This script builds dx-style with maximum performance using PGO

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PGO_DATA_DIR="${PROJECT_ROOT}/target/pgo-data"
PROFDATA_FILE="${PGO_DATA_DIR}/merged.profdata"

echo "🚀 Building dx-style with Profile-Guided Optimization (PGO)"
echo "============================================================"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

cd "$PROJECT_ROOT"

# Clean previous PGO data
if [ -d "$PGO_DATA_DIR" ]; then
    echo -e "${YELLOW}Cleaning previous PGO data...${NC}"
    rm -rf "$PGO_DATA_DIR"
fi
mkdir -p "$PGO_DATA_DIR"

# Step 1: Build with instrumentation
echo ""
echo -e "${BLUE}Step 1: Building with profiling instrumentation...${NC}"
RUSTFLAGS="-Cprofile-generate=${PGO_DATA_DIR}" \
    cargo build --release --profile release-pgo
echo -e "${GREEN}✓ Instrumented build complete${NC}"

# Step 2: Run workloads to collect profile data
echo ""
echo -e "${BLUE}Step 2: Collecting profile data from representative workloads...${NC}"

# Create test HTML files for profiling
mkdir -p "${PROJECT_ROOT}/playgrounds/pgo-test"

# Small HTML (fast operations)
cat > "${PROJECT_ROOT}/playgrounds/pgo-test/small.html" << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Small Test</title></head>
<body>
    <div class="flex items-center justify-between bg-blue-500 p-4 rounded-lg shadow-xl">
        <h1 class="text-2xl font-bold text-white">Header</h1>
        <button class="bg-white text-blue-600 px-4 py-2 rounded hover:bg-gray-100">Click</button>
    </div>
</body>
</html>
EOF

# Medium HTML (typical use case)
cat > "${PROJECT_ROOT}/playgrounds/pgo-test/medium.html" << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Medium Test</title></head>
<body>
    <div class="container mx-auto px-4 py-8">
        <header class="flex items-center justify-between mb-8">
            <h1 class="text-4xl font-bold text-gray-900">Dashboard</h1>
            <nav class="flex gap-4">
                <a class="text-blue-600 hover:text-blue-800 font-medium">Home</a>
                <a class="text-blue-600 hover:text-blue-800 font-medium">About</a>
                <a class="text-blue-600 hover:text-blue-800 font-medium">Contact</a>
            </nav>
        </header>
        <main class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
EOF

for i in {1..50}; do
    cat >> "${PROJECT_ROOT}/playgrounds/pgo-test/medium.html" << EOF
            <div class="bg-white p-6 rounded-lg shadow-md hover:shadow-xl transition-shadow">
                <h2 class="text-xl font-semibold text-gray-800 mb-2">Card $i</h2>
                <p class="text-gray-600 mb-4">Description for card $i with some content.</p>
                <button class="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600 transition-colors">Action</button>
            </div>
EOF
done

cat >> "${PROJECT_ROOT}/playgrounds/pgo-test/medium.html" << 'EOF'
        </main>
    </div>
</body>
</html>
EOF

# Large HTML (stress test)
cat > "${PROJECT_ROOT}/playgrounds/pgo-test/large.html" << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Large Test</title></head>
<body class="bg-gray-50">
    <div class="min-h-screen">
EOF

for i in {1..200}; do
    cat >> "${PROJECT_ROOT}/playgrounds/pgo-test/large.html" << EOF
        <div class="flex items-center justify-between bg-gradient-to-r from-blue-$((i % 9 + 1))00 to-purple-$((i % 9 + 1))00 p-$((i % 8 + 1)) rounded-lg shadow-$((i % 3 == 0 ? "xl" : "md")) hover:shadow-2xl transition-all duration-$((i % 5 * 100 + 100)) m-$((i % 6 + 1))">
            <h3 class="text-$((i % 4 + 1))xl font-bold text-white">Element $i</h3>
            <span class="text-sm text-gray-$((i % 9 + 1))00">Info</span>
        </div>
EOF
done

cat >> "${PROJECT_ROOT}/playgrounds/pgo-test/large.html" << 'EOF'
    </div>
</body>
</html>
EOF

# Grouping HTML (advanced features)
cat > "${PROJECT_ROOT}/playgrounds/pgo-test/grouping.html" << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Grouping Test</title></head>
<body>
EOF

for i in {1..30}; do
    cat >> "${PROJECT_ROOT}/playgrounds/pgo-test/grouping.html" << EOF
    <div class="card(bg-white p-4 rounded shadow hover:shadow-lg transition-all) m-$((i % 4 + 1))">
        <div class="header(flex items-center justify-between mb-2)">
            <h3 class="title(text-lg font-bold text-gray-800)">Card $i</h3>
        </div>
        <p class="content(text-gray-600 text-sm)">Content here</p>
    </div>
EOF
done

cat >> "${PROJECT_ROOT}/playgrounds/pgo-test/grouping.html" << 'EOF'
</body>
</html>
EOF

# Update config to use test files
cat > "${PROJECT_ROOT}/.dx/config.pgo.toml" << 'EOF'
[paths]
html_dir = "playgrounds/pgo-test"
index_file = "playgrounds/pgo-test/medium.html"
css_file = "playgrounds/pgo-test/output.css"

[watch]
debounce_ms = 10
EOF

# Run the instrumented binary with different workloads
echo "  Running workload: small HTML..."
DX_CONFIG="${PROJECT_ROOT}/.dx/config.pgo.toml" \
    timeout 2 "${PROJECT_ROOT}/target/release-pgo/style" || true

echo "  Running workload: medium HTML..."
sed -i.bak 's/medium\.html/medium.html/g' "${PROJECT_ROOT}/.dx/config.pgo.toml" 2>/dev/null || \
    sed -i '' 's/medium\.html/medium.html/g' "${PROJECT_ROOT}/.dx/config.pgo.toml" 2>/dev/null || true
DX_CONFIG="${PROJECT_ROOT}/.dx/config.pgo.toml" \
    timeout 2 "${PROJECT_ROOT}/target/release-pgo/style" || true

echo "  Running workload: large HTML..."
sed -i.bak 's/medium\.html/large.html/g' "${PROJECT_ROOT}/.dx/config.pgo.toml" 2>/dev/null || \
    sed -i '' 's/medium\.html/large.html/g' "${PROJECT_ROOT}/.dx/config.pgo.toml" 2>/dev/null || true
DX_CONFIG="${PROJECT_ROOT}/.dx/config.pgo.toml" \
    timeout 2 "${PROJECT_ROOT}/target/release-pgo/style" || true

echo "  Running workload: grouping HTML..."
sed -i.bak 's/large\.html/grouping.html/g' "${PROJECT_ROOT}/.dx/config.pgo.toml" 2>/dev/null || \
    sed -i '' 's/large\.html/grouping.html/g' "${PROJECT_ROOT}/.dx/config.pgo.toml" 2>/dev/null || true
DX_CONFIG="${PROJECT_ROOT}/.dx/config.pgo.toml" \
    timeout 2 "${PROJECT_ROOT}/target/release-pgo/style" || true

echo -e "${GREEN}✓ Profile data collected${NC}"

# Step 3: Merge profile data
echo ""
echo -e "${BLUE}Step 3: Merging profile data...${NC}"

# Find llvm-profdata
LLVM_PROFDATA=""
for version in 18 17 16 15 14; do
    if command -v llvm-profdata-$version &> /dev/null; then
        LLVM_PROFDATA="llvm-profdata-$version"
        break
    fi
done

if [ -z "$LLVM_PROFDATA" ] && command -v llvm-profdata &> /dev/null; then
    LLVM_PROFDATA="llvm-profdata"
fi

if [ -z "$LLVM_PROFDATA" ]; then
    echo -e "${YELLOW}Warning: llvm-profdata not found. Trying rustup component...${NC}"
    if rustup component add llvm-tools-preview 2>/dev/null; then
        LLVM_TOOLS=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | grep host | cut -d' ' -f2)/bin
        if [ -f "$LLVM_TOOLS/llvm-profdata" ]; then
            LLVM_PROFDATA="$LLVM_TOOLS/llvm-profdata"
        fi
    fi
fi

if [ -z "$LLVM_PROFDATA" ]; then
    echo -e "${YELLOW}Warning: Could not find llvm-profdata. Skipping profile merge.${NC}"
    echo -e "${YELLOW}Install with: sudo apt-get install llvm (Ubuntu) or brew install llvm (macOS)${NC}"
else
    "$LLVM_PROFDATA" merge -o "$PROFDATA_FILE" "${PGO_DATA_DIR}"/default_*.profraw
    echo -e "${GREEN}✓ Profile data merged${NC}"
fi

# Step 4: Build with PGO
echo ""
echo -e "${BLUE}Step 4: Building optimized binary with PGO...${NC}"

if [ -f "$PROFDATA_FILE" ]; then
    RUSTFLAGS="-Cprofile-use=${PROFDATA_FILE} -Cllvm-args=-pgo-warn-missing-function" \
        cargo build --release
    echo -e "${GREEN}✓ PGO-optimized build complete${NC}"
else
    echo -e "${YELLOW}Profile data not available, building without PGO...${NC}"
    cargo build --release
fi

# Cleanup
echo ""
echo -e "${BLUE}Cleaning up...${NC}"
rm -rf "${PROJECT_ROOT}/playgrounds/pgo-test"
rm -f "${PROJECT_ROOT}/.dx/config.pgo.toml"
rm -f "${PROJECT_ROOT}/.dx/config.pgo.toml.bak"

echo ""
echo -e "${GREEN}✓ Build complete!${NC}"
echo ""
echo "Optimized binary location: ${PROJECT_ROOT}/target/release/style"
echo ""
echo "Performance improvements from PGO typically range from 10-20%"
echo "Run benchmarks to verify: cargo bench"
echo ""
echo "To install: cargo install --path . --locked"
