#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

// This script runs after vsce package to rename the output file
const extensionDir = __dirname;
const files = fs.readdirSync(extensionDir);

// Find any .vsix file that's not dx.vsix
const vsixFile = files.find(f => f.endsWith('.vsix') && f !== 'dx.vsix');

if (vsixFile) {
  const sourcePath = path.join(extensionDir, vsixFile);
  const targetPath = path.join(extensionDir, 'dx.vsix');
  
  // Remove old dx.vsix if it exists
  if (fs.existsSync(targetPath)) {
    fs.unlinkSync(targetPath);
  }
  
  // Copy (not rename) so the original stays
  fs.copyFileSync(sourcePath, targetPath);
  console.log(`Created dx.vsix from ${vsixFile}`);
}
