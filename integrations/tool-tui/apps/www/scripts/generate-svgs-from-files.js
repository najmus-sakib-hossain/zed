import { readdirSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const svglDir = join(__dirname, '../public/svgl');
const files = readdirSync(svglDir).filter(f => f.endsWith('.svg'));

// Group files by base name (without theme suffixes)
const iconMap = new Map();

files.forEach(file => {
  const name = file.replace('.svg', '');
  
  // Detect theme variants (both hyphen and underscore formats)
  if (name.endsWith('_light') || name.endsWith('-light')) {
    const base = name.replace(/[_-]light$/, '');
    if (!iconMap.has(base)) iconMap.set(base, {});
    iconMap.get(base).light = `/svgl/${file}`;
  } else if (name.endsWith('_dark') || name.endsWith('-dark')) {
    const base = name.replace(/[_-]dark$/, '');
    if (!iconMap.has(base)) iconMap.set(base, {});
    iconMap.get(base).dark = `/svgl/${file}`;
  } else if (name.endsWith('_wordmark') || name.endsWith('-wordmark')) {
    const base = name.replace(/[_-]wordmark$/, '');
    if (!iconMap.has(base)) iconMap.set(base, {});
    iconMap.get(base).wordmark = `/svgl/${file}`;
  } else if (name.includes('wordmark')) {
    // Handle wordmark variants with themes
    const withoutWordmark = name.replace(/[_-]wordmark/, '');
    if (withoutWordmark.endsWith('_light') || withoutWordmark.endsWith('-light')) {
      const base = withoutWordmark.replace(/[_-]light$/, '');
      if (!iconMap.has(base)) iconMap.set(base, {});
      if (!iconMap.get(base).wordmark) iconMap.get(base).wordmark = {};
      iconMap.get(base).wordmark.light = `/svgl/${file}`;
    } else if (withoutWordmark.endsWith('_dark') || withoutWordmark.endsWith('-dark')) {
      const base = withoutWordmark.replace(/[_-]dark$/, '');
      if (!iconMap.has(base)) iconMap.set(base, {});
      if (!iconMap.get(base).wordmark) iconMap.get(base).wordmark = {};
      iconMap.get(base).wordmark.dark = `/svgl/${file}`;
    } else {
      const base = name.replace(/[_-]wordmark.*$/, '');
      if (!iconMap.has(base)) iconMap.set(base, {});
      iconMap.get(base).wordmark = `/svgl/${file}`;
    }
  } else {
    // Base icon
    if (!iconMap.has(name)) iconMap.set(name, {});
    iconMap.get(name).route = `/svgl/${file}`;
  }
});

// Convert to array format
const svgs = [];
let id = 1;

for (const [name, paths] of iconMap.entries()) {
  const icon = { 
    id: id++, 
    title: formatTitle(name), 
    category: 'Software',
    url: `https://${name.toLowerCase().replace(/[_\s]/g, '')}.com`
  };
  
  // Handle route (can be string or object with light/dark) - use actual file paths
  if (paths.route) {
    icon.route = paths.route;
  } else if (paths.light && paths.dark) {
    icon.route = { 
      light: paths.light, 
      dark: paths.dark 
    };
  } else if (paths.light) {
    icon.route = paths.light;
  } else if (paths.dark) {
    icon.route = paths.dark;
  }
  
  // Handle wordmark
  if (paths.wordmark) {
    icon.wordmark = paths.wordmark;
  }
  
  // Only add if we have at least a route
  if (icon.route) {
    svgs.push(icon);
  }
}

// Sort by title
svgs.sort((a, b) => a.title.localeCompare(b.title));

// Reassign IDs after sorting
svgs.forEach((svg, index) => svg.id = index + 1);

// Generate TypeScript file
const output = `import type { iSVG } from '@/types/svg';

export const svgs: iSVG[] = ${JSON.stringify(svgs, null, 2)};
`;

const outputPath = join(__dirname, '../data/svgs.ts');
writeFileSync(outputPath, output, 'utf-8');

console.log(`✓ Generated ${svgs.length} icons from public/svgl/`);

function formatTitle(name) {
  return name
    .split(/[_-]/)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
