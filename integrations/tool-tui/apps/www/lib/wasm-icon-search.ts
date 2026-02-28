// WASM-powered icon search using Rust
import init, { IconSearch } from './wasm/dx_icon_search_wasm';

let wasmInitialized = false;
let searchEngine: IconSearch | null = null;
let loadingPromise: Promise<void> | null = null;

export interface WasmIcon {
  name: string;
  pack: string;
  svg?: string;
}

export interface WasmSearchResult {
  name: string;
  pack: string;
  score: number;
}

export async function initWasmSearch(): Promise<void> {
  if (wasmInitialized) return;
  
  try {
    await init();
    searchEngine = new IconSearch();
    wasmInitialized = true;
  } catch (error) {
    console.error('Failed to initialize WASM:', error);
    throw error;
  }
}

export async function loadIconsIntoWasm(icons: WasmIcon[]): Promise<void> {
  // Prevent concurrent loading (React StrictMode calls effects twice)
  if (loadingPromise) {
    return loadingPromise;
  }
  
  loadingPromise = (async () => {
    if (!searchEngine) {
      await initWasmSearch();
    }
    
    // Validate first icon
    if (icons.length > 0) {
      const first = icons[0];
      if ('svg' in first) {
        throw new Error('Icons still contain SVG bodies! Clear browser cache and reload.');
      }
    }
    
    try {
      // Sanitize all icons at once
      const sanitized = icons.map(icon => ({
        name: icon.name.replace(/[\u0000-\u001F\u007F-\u009F\u2028\u2029]/g, ''),
        pack: icon.pack
      }));
      
      // Load ALL icons in ONE call (no batching to avoid RefCell borrow issues)
      const jsonData = JSON.stringify(sanitized);
      
      searchEngine!.loadIcons(jsonData);
    } catch (error) {
      console.error('Failed to load icons:', error);
      loadingPromise = null; // Reset on error
      throw error;
    }
  })();
  
  return loadingPromise;
}

export async function searchIconsWasm(
  query: string,
  limit: number = 100
): Promise<WasmSearchResult[]> {
  if (!searchEngine) {
    throw new Error('WASM search engine not initialized');
  }
  
  return searchEngine.search(query, limit);
}

export function getTotalIcons(): number {
  if (!searchEngine) return 0;
  return searchEngine.totalIcons();
}

export function clearCache(): void {
  if (searchEngine) {
    searchEngine.clearCache();
  }
}

export function getCacheSize(): number {
  if (!searchEngine) return 0;
  return searchEngine.cacheSize();
}
