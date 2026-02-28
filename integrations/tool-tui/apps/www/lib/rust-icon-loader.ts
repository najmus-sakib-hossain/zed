// Fast icon loader using 396MB JSON data from public/icons/
// Integrates with Rust search engine for solar and lucide icons

export interface RustIconMetadata {
  name: string;
  pack: string;
  category?: string;
  tags?: string[];
}

export interface RustIconSearchResult extends RustIconMetadata {
  score: number;
  matchType: 'exact' | 'prefix' | 'substring' | 'fuzzy';
}

// Cache for loaded icon packs
const packCache = new Map<string, any>();
const iconMetadataCache = new Map<string, RustIconMetadata[]>();

// Load icon pack JSON
async function loadIconPack(pack: string): Promise<any> {
  if (packCache.has(pack)) {
    return packCache.get(pack);
  }

  try {
    const response = await fetch(`/icons/${pack}.json`);
    if (!response.ok) {
      throw new Error(`Failed to load ${pack}: ${response.statusText}`);
    }
    const data = await response.json();
    packCache.set(pack, data);
    return data;
  } catch (error) {
    console.error(`Error loading ${pack}:`, error);
    return null;
  }
}

// Extract metadata from icon pack
function extractMetadata(pack: string, data: any): RustIconMetadata[] {
  const metadata: RustIconMetadata[] = [];
  
  if (!data || !data.icons) {
    return metadata;
  }

  // Handle different JSON structures
  const icons = data.icons;
  
  if (Array.isArray(icons)) {
    // Array format
    for (const icon of icons) {
      if (typeof icon === 'string') {
        metadata.push({ name: icon, pack });
      } else if (icon.name) {
        metadata.push({
          name: icon.name,
          pack,
          category: icon.category,
          tags: icon.tags,
        });
      }
    }
  } else if (typeof icons === 'object') {
    // Object format: { iconName: { body: "...", ... }, ... }
    for (const [name, iconData] of Object.entries(icons)) {
      metadata.push({
        name,
        pack,
        category: