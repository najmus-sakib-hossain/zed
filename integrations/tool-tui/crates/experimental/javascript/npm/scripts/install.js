#!/usr/bin/env node
/**
 * DX JavaScript Runtime - Binary Installation Script
 * 
 * This script downloads the appropriate pre-built binary for the user's platform
 * and verifies its integrity using SHA256 checksums.
 * 
 * Requirements: 8.1, 8.2
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const crypto = require('crypto');
const zlib = require('zlib');
const tar = require('tar');

const PACKAGE_VERSION = require('../package.json').version;
const GITHUB_REPO = 'nicholasgrose/dx-js';
const BINARY_NAME = process.platform === 'win32' ? 'dx-js.exe' : 'dx-js';

/**
 * Get the platform-specific binary name
 */
function getBinaryInfo() {
  const platform = process.platform;
  const arch = process.arch;
  
  const platformMap = {
    'darwin-x64': { name: 'dx-js-macos-x86_64', ext: 'tar.gz' },
    'darwin-arm64': { name: 'dx-js-macos-arm64', ext: 'tar.gz' },
    'linux-x64': { name: 'dx-js-linux-x86_64', ext: 'tar.gz' },
    'linux-arm64': { name: 'dx-js-linux-arm64', ext: 'tar.gz' },
    'win32-x64': { name: 'dx-js-windows-x86_64.exe', ext: 'zip' },
  };
  
  const key = `${platform}-${arch}`;
  const info = platformMap[key];
  
  if (!info) {
    throw new Error(
      `Unsupported platform: ${platform}-${arch}\n` +
      `Supported platforms: ${Object.keys(platformMap).join(', ')}`
    );
  }
  
  return info;
}

/**
 * Download a file from URL
 */
function download(url) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      // Handle redirects
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        return download(response.headers.location).then(resolve).catch(reject);
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: HTTP ${response.statusCode}`));
        return;
      }
      
      const chunks = [];
      response.on('data', (chunk) => chunks.push(chunk));
      response.on('end', () => resolve(Buffer.concat(chunks)));
      response.on('error', reject);
    });
    
    request.on('error', reject);
    request.setTimeout(60000, () => {
      request.destroy();
      reject(new Error('Download timeout'));
    });
  });
}

/**
 * Verify SHA256 checksum
 */
function verifyChecksum(data, expectedHash) {
  const actualHash = crypto.createHash('sha256').update(data).digest('hex');
  return actualHash.toLowerCase() === expectedHash.toLowerCase();
}

/**
 * Extract tar.gz archive
 */
async function extractTarGz(data, destDir) {
  return new Promise((resolve, reject) => {
    const gunzip = zlib.createGunzip();
    const extract = tar.extract({ cwd: destDir });
    
    extract.on('finish', resolve);
    extract.on('error', reject);
    gunzip.on('error', reject);
    
    gunzip.pipe(extract);
    gunzip.end(data);
  });
}

/**
 * Extract zip archive (Windows)
 */
async function extractZip(data, destDir, binaryName) {
  const tempZip = path.join(destDir, 'temp.zip');
  fs.writeFileSync(tempZip, data);
  
  try {
    // Use PowerShell to extract on Windows
    execSync(`powershell -Command "Expand-Archive -Path '${tempZip}' -DestinationPath '${destDir}' -Force"`, {
      stdio: 'pipe'
    });
  } finally {
    fs.unlinkSync(tempZip);
  }
}

/**
 * Main installation function
 */
async function install() {
  console.log(`Installing DX JavaScript Runtime v${PACKAGE_VERSION}...`);
  
  const binaryInfo = getBinaryInfo();
  const baseUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${PACKAGE_VERSION}`;
  const archiveUrl = `${baseUrl}/${binaryInfo.name}.${binaryInfo.ext}`;
  const checksumUrl = `${archiveUrl}.sha256`;
  
  console.log(`Platform: ${process.platform}-${process.arch}`);
  console.log(`Downloading from: ${archiveUrl}`);
  
  // Download checksum file
  let expectedChecksum;
  try {
    const checksumData = await download(checksumUrl);
    expectedChecksum = checksumData.toString().trim().split(/\s+/)[0];
    console.log(`Expected checksum: ${expectedChecksum}`);
  } catch (err) {
    console.warn(`Warning: Could not download checksum file: ${err.message}`);
    console.warn('Proceeding without checksum verification...');
  }
  
  // Download binary archive
  console.log('Downloading binary...');
  const archiveData = await download(archiveUrl);
  console.log(`Downloaded ${(archiveData.length / 1024 / 1024).toFixed(2)} MB`);
  
  // Verify checksum if available
  if (expectedChecksum) {
    console.log('Verifying checksum...');
    if (!verifyChecksum(archiveData, expectedChecksum)) {
      throw new Error(
        'Checksum verification failed!\n' +
        'The downloaded file may be corrupted or tampered with.\n' +
        'Please try again or download manually from GitHub releases.'
      );
    }
    console.log('Checksum verified ✓');
  }
  
  // Create bin directory
  const binDir = path.join(__dirname, '..', 'bin');
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  // Extract archive
  console.log('Extracting...');
  if (binaryInfo.ext === 'tar.gz') {
    await extractTarGz(archiveData, binDir);
  } else {
    await extractZip(archiveData, binDir, BINARY_NAME);
  }
  
  // Make binary executable (Unix)
  const binaryPath = path.join(binDir, BINARY_NAME);
  if (process.platform !== 'win32') {
    fs.chmodSync(binaryPath, 0o755);
  }
  
  // Verify binary exists
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`Binary not found after extraction: ${binaryPath}`);
  }
  
  console.log(`\n✓ DX JavaScript Runtime v${PACKAGE_VERSION} installed successfully!`);
  console.log(`  Binary location: ${binaryPath}`);
  console.log('\nRun "dx-js --help" to get started.');
}

// Run installation
install().catch((err) => {
  console.error(`\n✗ Installation failed: ${err.message}`);
  console.error('\nTroubleshooting:');
  console.error('1. Check your internet connection');
  console.error('2. Ensure you have write permissions to the installation directory');
  console.error('3. Try installing manually from: https://github.com/' + GITHUB_REPO + '/releases');
  process.exit(1);
});
