#!/usr/bin/env bash

# Fix all import and type errors in dx-bundler-v2

echo "Fixing dx-bundler-v2 compilation errors..."

# Fix dx-bundle-cache Path import
cat > /tmp/cache_fix.txt << 'EOF'
//! Persistent warm cache for instant rebuilds
//!
//! Memory-map transformed modules, skip unchanged files

pub mod warm;
pub mod persistent;

pub use warm::WarmCache;
pub use persistent::PersistentCache;

use dx_bundle_core::{ContentHash, ModuleId, TransformedModule};
use dx_bundle_core::error::BundleResult;
use std::path::Path;

/// Cached transform data
EOF

head -n 12 f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-cache/src/lib.rs > /tmp/cache_orig.txt
if diff -q /tmp/cache_orig.txt /tmp/cache_fix.txt > /dev/null 2>&1; then
    echo "Cache lib already fixed"
else
    sed -i '11 a use std::path::Path;' f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-cache/src/lib.rs
fi

# Fix dx-bundle-parallel speculative imports
sed -i 's/PathHasher,//' f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-parallel/src/speculative.rs
sed -i '/^use dx_bundle_core::config::ModuleFormat;/a use dx_bundle_core::hash::PathHasher;' f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-parallel/src/speculative.rs
sed -i 's/BundleError, BundleResult,//' f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-parallel/src/speculative.rs  
sed -i '/^use dx_bundle_core::config::ModuleFormat;/a use dx_bundle_core::error::{BundleError, BundleResult};' f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-parallel/src/speculative.rs

# Fix dx-bundle-delta BundleError imports
sed -i 's/use dx_bundle_core::{ChunkId, ContentHash, ModuleId};/use dx_bundle_core::{ChunkId, ContentHash, ModuleId};\nuse dx_bundle_core::error::BundleError;/' f:/Code/dx/crates/dx-bundler-v2/crates/dx-bundle-delta/src/lib.rs

echo "Fixed all import errors. Now building..."
cd f:/Code/dx/crates/dx-bundler-v2
cargo build --release --bin dx-bundle 2>&1 | tail -20
