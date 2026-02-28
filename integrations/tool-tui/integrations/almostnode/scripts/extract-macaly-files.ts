/**
 * Extract files from the cloned macaly-web repository
 * Outputs a JSON file that can be loaded by the browser demo
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Try the real macaly-web location first, then fall back to temp
const MACALY_PATH = fs.existsSync('/Users/petrbrzek/Desktop/dev/macaly-web')
  ? '/Users/petrbrzek/Desktop/dev/macaly-web'
  : path.join(__dirname, '../temp/macaly-web');
const OUTPUT_PATH = path.join(__dirname, '../temp/macaly-files.json');

// Text file extensions (read as UTF-8)
const TEXT_EXTENSIONS = [
  '.ts', '.tsx', '.js', '.jsx', '.json', '.css', '.md', '.mdx',
  '.svg', '.ico', '.txt', '.mjs', '.cjs', '.html', '.xml'
];

// Binary file extensions (read as base64)
const BINARY_EXTENSIONS = [
  '.png', '.jpg', '.jpeg', '.gif', '.webp', '.avif', '.bmp',
  '.woff', '.woff2', '.ttf', '.eot', '.otf',
  '.mp3', '.mp4', '.wav', '.ogg', '.webm',
  '.pdf', '.zip'
];

// All included extensions
const INCLUDE_EXTENSIONS = [...TEXT_EXTENSIONS, ...BINARY_EXTENSIONS];

// Directories to skip
const SKIP_DIRS = [
  'node_modules', '.git', '.next', 'dist', '.turbo', '.vercel',
  '.claude', '.macaly', '.vscode'
];

// Files to skip
const SKIP_FILES = [
  'pnpm-lock.yaml', 'package-lock.json', 'yarn.lock'
];

interface FileMap {
  [path: string]: string;
}

function shouldIncludeFile(filePath: string): boolean {
  const ext = path.extname(filePath);
  const basename = path.basename(filePath);

  if (SKIP_FILES.includes(basename)) return false;
  if (!INCLUDE_EXTENSIONS.includes(ext)) return false;

  return true;
}

function shouldIncludeDir(dirName: string): boolean {
  return !SKIP_DIRS.includes(dirName);
}

function readFilesRecursively(dir: string, basePath: string = ''): FileMap {
  const files: FileMap = {};

  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    const relativePath = basePath ? `${basePath}/${entry.name}` : entry.name;

    if (entry.isDirectory()) {
      if (shouldIncludeDir(entry.name)) {
        const subFiles = readFilesRecursively(fullPath, relativePath);
        Object.assign(files, subFiles);
      }
    } else if (entry.isFile()) {
      if (shouldIncludeFile(entry.name)) {
        try {
          const ext = path.extname(entry.name);
          if (BINARY_EXTENSIONS.includes(ext)) {
            // Read binary files as base64 with a prefix marker
            const buffer = fs.readFileSync(fullPath);
            files['/' + relativePath] = 'base64:' + buffer.toString('base64');
          } else {
            // Read text files as UTF-8
            const content = fs.readFileSync(fullPath, 'utf-8');
            files['/' + relativePath] = content;
          }
        } catch (e) {
          console.error(`Error reading ${fullPath}:`, e);
        }
      }
    }
  }

  return files;
}

function main() {
  console.log('Extracting files from macaly-web...');
  console.log(`Source: ${MACALY_PATH}`);
  console.log(`Output: ${OUTPUT_PATH}`);

  if (!fs.existsSync(MACALY_PATH)) {
    console.error('Error: macaly-web not found. Clone it first with:');
    console.error('git clone git@github.com:langtail/macaly-web.git temp/macaly-web');
    process.exit(1);
  }

  const files = readFilesRecursively(MACALY_PATH);
  const fileCount = Object.keys(files).length;

  console.log(`Found ${fileCount} files`);

  // Write JSON
  fs.writeFileSync(OUTPUT_PATH, JSON.stringify(files, null, 2));

  // Print summary
  const totalSize = Object.values(files).reduce((sum, content) => sum + content.length, 0);
  console.log(`Total size: ${(totalSize / 1024).toFixed(1)} KB`);
  console.log(`Output written to: ${OUTPUT_PATH}`);

  // Print file tree summary
  console.log('\nTop-level directories:');
  const topDirs = new Set<string>();
  for (const filePath of Object.keys(files)) {
    const parts = filePath.split('/').filter(Boolean);
    if (parts.length > 0) {
      topDirs.add(parts[0]);
    }
  }
  for (const dir of Array.from(topDirs).sort()) {
    const count = Object.keys(files).filter(f => f.startsWith('/' + dir + '/')).length;
    console.log(`  ${dir}/ (${count} files)`);
  }
}

main();
