#!/bin/bash
# Script to rename all dx references to dx

echo "Renaming dx to dx in all files..."

# Rename in JSON files
find . -type f -name "*.json" -exec sed -i 's/dx/dx/g' {} +
find . -type f -name "*.json" -exec sed -i 's/Dx/Dx/g' {} +
find . -type f -name "*.json" -exec sed -i 's/DX/DX/g' {} +

# Rename in TypeScript/JavaScript files
find . -type f \( -name "*.ts" -o -name "*.js" -o -name "*.mts" -o -name "*.mjs" \) -exec sed -i 's/dx/dx/g' {} +
find . -type f \( -name "*.ts" -o -name "*.js" -o -name "*.mts" -o -name "*.mjs" \) -exec sed -i 's/Dx/Dx/g' {} +
find . -type f \( -name "*.ts" -o -name "*.js" -o -name "*.mts" -o -name "*.mjs" \) -exec sed -i 's/DX/DX/g' {} +

# Rename in Markdown files
find . -type f -name "*.md" -exec sed -i 's/dx/dx/g' {} +
find . -type f -name "*.md" -exec sed -i 's/Dx/Dx/g' {} +
find . -type f -name "*.md" -exec sed -i 's/DX/DX/g' {} +

# Rename plugin config files
find . -type f -name "dx.plugin.json" -exec bash -c 'mv "$0" "${0/dx.plugin/dx.plugin}"' {} \;

# Rename in YAML files
find . -type f \( -name "*.yml" -o -name "*.yaml" \) -exec sed -i 's/dx/dx/g' {} +
find . -type f \( -name "*.yml" -o -name "*.yaml" \) -exec sed -i 's/Dx/Dx/g' {} +
find . -type f \( -name "*.yml" -o -name "*.yaml" \) -exec sed -i 's/DX/DX/g' {} +

# Rename in shell scripts
find . -type f -name "*.sh" -exec sed -i 's/dx/dx/g' {} +
find . -type f -name "*.sh" -exec sed -i 's/Dx/Dx/g' {} +
find . -type f -name "*.sh" -exec sed -i 's/DX/DX/g' {} +

# Rename in Swift files (for Swabble)
find ../swabble -type f -name "*.swift" -exec sed -i 's/dx/dx/g' {} + 2>/dev/null || true
find ../swabble -type f -name "*.swift" -exec sed -i 's/Dx/Dx/g' {} + 2>/dev/null || true
find ../swabble -type f -name "*.swift" -exec sed -i 's/DX/DX/g' {} + 2>/dev/null || true

echo "Renaming complete!"
echo "Files updated:"
echo "  - *.json (package.json, plugin configs)"
echo "  - *.ts, *.js (TypeScript/JavaScript)"
echo "  - *.md (Documentation)"
echo "  - *.yml, *.yaml (Config files)"
echo "  - *.sh (Shell scripts)"
echo "  - *.swift (Swift code in swabble/)"
