// Adapter to convert iconify icons to existing SVG format
import { loadAllIconData, getAvailableIconPacks, type IconifyIcon } from './icon-data-loader';
import type { iSVG } from '@/types/svg';

let iconCache: iSVG[] | null = null;

// WASM search commented out - using PGlite + client-side filtering instead
// import init, { search_icons } from 'dx-icons';
// let wasmInitialized = false;

// Get list of all packs
export async function getAvailablePacks(): Promise<string[]> {
  return await getAvailableIconPacks();
}

// Convert iconify icon to SVG format
function iconToSVG(icon: IconifyIcon, id: number): iSVG {
  return {
    id,
    title: icon.name,
    category: icon.pack,
    route: `/api/icons/${icon.pack}/${icon.name}`,
    url: `https://iconify.design/icon-sets/${icon.pack}/${icon.name}`,
  };
}

// Get total icon count
export function getTotalIconCount(): number {
  return iconCache?.length || 0;
}

// Load all icons and convert to SVG format
export async function loadAllIcons(): Promise<iSVG[]> {
  if (iconCache) return iconCache;
  
  const iconData = await loadAllIconData();
  iconCache = iconData.map((icon, index) => iconToSVG(icon, index + 1));
  
  return iconCache;
}

// Search icons using simple filter
export async function searchDXIcons(query: string, packFilter?: string): Promise<iSVG[]> {
  const allIcons = await loadAllIcons();
  const lowerQuery = query.toLowerCase();
  
  return allIcons.filter(icon => {
    const matchesQuery = icon.title.toLowerCase().includes(lowerQuery);
    const matchesPack = !packFilter || icon.category === packFilter;
    return matchesQuery && matchesPack;
  });
}
