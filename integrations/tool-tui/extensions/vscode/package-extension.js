#!/usr/bin/env node
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// Run vsce package using npx
console.log('Packaging extension...');
execSync('npx @vscode/vsce package', { stdio: 'inherit' });

// Find the generated .vsix file (should be dx-{version}.vsix)
const files = fs.readdirSync(__dirname);
const vsixFile = files.find(f => f.match(/^dx-\d+\.\d+\.\d+\.vsix$/));

if (vsixFile) {
  const sourcePath = path.join(__dirname, vsixFile);
  const targetPath = path.join(__dirname, 'dx.vsix');
  
  // Rename to dx.vsix (removes the versioned file)
  fs.renameSync(sourcePath, targetPath);
  console.log(`Created dx.vsix (removed ${vsixFile})`);
} else {
  console.error('No versioned .vsix file found!');
  process.exit(1);
}
