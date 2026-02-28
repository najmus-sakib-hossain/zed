import { readFileSync, writeFileSync, unlinkSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Read the original svgs.ts from the Svelte app
const svgsPath = join(__dirname, '../../svgl/src/data/svgs.ts');
let svgsContent = readFileSync(svgsPath, 'utf-8');

// Remove import and type annotation, keep export
svgsContent = svgsContent
  .replace(/import type \{ iSVG \} from ".*";\s*\n/, '')
  .replace(/export const svgs: iSVG\[\] = /, 'export const svgs = ');

// Write to a temporary .mjs file
const tempPath = join(__dirname, 'temp-svgs.mjs');
writeFileSync(tempPath, svgsContent, 'utf-8');

// Import the data
const { svgs } = await import('./temp-svgs.mjs');

// Clean up temp file
unlinkSync(tempPath);

// Add IDs
const svgsWithIds = svgs.map((svg, index) => ({
  id: index + 1,
  ...svg,
}));

// Generate TypeScript file
const output = `import type { iSVG } from '@/types/svg';

export const svgs: iSVG[] = ${JSON.stringify(svgsWithIds, null, 2)};
`;

// Write to data file
const outputPath = join(__dirname, '../data/svgs.ts');
writeFileSync(outputPath, output, 'utf-8');

console.log(`✓ Generated ${svgsWithIds.length} logos with real URLs`);
