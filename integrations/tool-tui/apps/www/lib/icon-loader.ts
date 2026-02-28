// Icon loader with WASM search for solar/lucide
import { initWasmSearch, loadIconsIntoWasm, searchIconsWasm, WasmSearchResult } from './wasm-icon-search';
import { loadAllIconData } from './icon-data-loader';

export interface IconEntry {
  name: string;
  pack: string;
}

export interface IconSearchResult {
  icons: IconEntry[];
  searchTime: number;
}

let wasmReady = false;

// Parse .llm text format to extract icon names
async function parseLLMFile(pack: string): Promise<string[]> {
  try {
    // Read from .llm text format (not .machine binary!)
    const response = await fetch(`/serializer/${pack}.llm?v=2`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const text = await response.text();
    const icons: string[] = [];
    const lines = text.split('\n');
    
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      
      // Match: icons.name(...)
      const match = trimmed.match(/^icons\.([a-zA-Z0-9_-]+)\(/);
      if (match) {
        icons.push(match[1]);
      }
    }
    
    return icons;
  } catch (error) {
    console.error(`Failed to parse ${pack}:`, error);
    return [];
  }
}

// Load SVGL icons from public/library directory
async function loadSVGLIcons(): Promise<string[]> {
  try {
    const response = await fetch('/api/svgl-list');
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    return response.json();
  } catch (error) {
    console.error('Failed to load SVGL icons:', error);
    return [];
  }
}

// Global icon index (loaded once)
let globalIndex: IconEntry[] | null = null;
let indexLoadPromise: Promise<IconEntry[]> | null = null;

// Initialize WASM search engine
async function ensureWasmReady(): Promise<void> {
  if (wasmReady) return;
  
  try {
    await initWasmSearch();
    const iconData = await loadAllIconData();
    await loadIconsIntoWasm(iconData);
    wasmReady = true;
  } catch (error) {
    console.error('WASM initialization failed:', error);
  }
}

export async function loadIconIndex(): Promise<IconEntry[]> {
  if (globalIndex) {
    return globalIndex;
  }
  
  if (indexLoadPromise) {
    return indexLoadPromise;
  }
  
  indexLoadPromise = (async () => {
    const packs = ['dx', 'lucide', 'solar'];
    const entries: IconEntry[] = [];
    
    // Load icons from .llm files
    await Promise.all(
      packs.map(async (pack) => {
        try {
          const icons = await parseLLMFile(pack);
          for (const name of icons) {
            entries.push({ name, pack });
          }
        } catch (e) {
          console.warn(`Failed to load ${pack}:`, e);
        }
      })
    );
    
    // Load SVGL icons from public/library directory
    try {
      const svglIcons = await loadSVGLIcons();
      for (const name of svglIcons) {
        entries.push({ name, pack: 'svgl' });
      }
    } catch (e) {
      console.warn('Failed to load SVGL icons:', e);
    }
    
    // Sort for better cache locality
    entries.sort((a, b) => a.name.localeCompare(b.name));
    
    globalIndex = entries;
    return entries;
  })();
  
  return indexLoadPromise;
}

// Fast search using WASM for all packs except svgl
export async function searchIcons(
  query: string,
  packFilter?: string
): Promise<IconSearchResult> {
  const start = performance.now();
  
  // Use WASM for all packs except svgl
  if (packFilter !== 'svgl' && packFilter !== 'dx') {
    try {
      await ensureWasmReady();
      const results = await searchIconsWasm(query, 1000);
      
      const icons: IconEntry[] = results
        .filter(r => !packFilter || r.pack === packFilter)
        .map(r => ({ name: r.name, pack: r.pack }));
      
      const searchTime = performance.now() - start;
      return { icons, searchTime };
    } catch (error) {
      console.error('WASM search failed, falling back:', error);
    }
  }
  
  // Fallback to original search for svgl and dx
  const index = await loadIconIndex();
  const queryLower = query.toLowerCase();
  
  const icons = index.filter((entry) => {
    if (packFilter && entry.pack !== packFilter) {
      return false;
    }
    return entry.name.toLowerCase().includes(queryLower);
  });
  
  const searchTime = performance.now() - start;
  
  return { icons, searchTime };
}

// Load SVG body from .llm file
export async function loadIconSVG(
  name: string,
  pack: string
): Promise<string> {
  const response = await fetch(`/api/icons/${pack}/${name}`);
  if (!response.ok) {
    throw new Error(`Failed to load icon: ${response.statusText}`);
  }
  return response.text();
}

// Fuzzy search with scoring
export interface FuzzyMatch extends IconEntry {
  score: number;
}

export async function fuzzySearchIcons(
  query: string,
  packFilter?: string
): Promise<FuzzyMatch[]> {
  // Use WASM for all packs except svgl/dx (already has scoring)
  if (packFilter !== 'svgl' && packFilter !== 'dx') {
    try {
      await ensureWasmReady();
      const results = await searchIconsWasm(query, 1000);
      
      return results
        .filter(r => !packFilter || r.pack === packFilter)
        .map(r => ({
          name: r.name,
          pack: r.pack,
          score: r.score,
        }));
    } catch (error) {
      console.error('WASM fuzzy search failed:', error);
    }
  }
  
  // Fallback for svgl/dx
  const { icons } = await searchIcons(query, packFilter);
  
  const matches: FuzzyMatch[] = icons.map((icon) => {
    const nameLower = icon.name.toLowerCase();
    const queryLower = query.toLowerCase();
    
    let score = 0;
    
    if (nameLower === queryLower) {
      score = 1000;
    } else if (nameLower.startsWith(queryLower)) {
      score = 500;
    } else if (nameLower.includes(queryLower)) {
      score = 100;
    }
    
    score -= icon.name.length;
    
    return { ...icon, score };
  });
  
  matches.sort((a, b) => b.score - a.score);
  
  return matches;
}
