import { readdir } from 'fs/promises';
import { NextResponse } from 'next/server';
import { join } from 'path';

export async function GET() {
  try {
    const libraryPath = join(process.cwd(), 'public', 'svgl');
    const files = await readdir(libraryPath);
    
    // Filter SVG files and remove .svg extension
    const allIcons = files
      .filter(file => file.endsWith('.svg'))
      .map(file => file.replace('.svg', ''));
    
    // Remove theme variant suffixes and deduplicate
    const iconMap = new Map<string, { hasLight: boolean; hasDark: boolean; hasBase: boolean }>();
    
    for (const icon of allIcons) {
      if (icon.endsWith('_light') || icon.endsWith('-light')) {
        const base = icon.replace(/[_-]light$/, '');
        const entry = iconMap.get(base) || { hasLight: false, hasDark: false, hasBase: false };
        entry.hasLight = true;
        iconMap.set(base, entry);
      } else if (icon.endsWith('_dark') || icon.endsWith('-dark')) {
        const base = icon.replace(/[_-]dark$/, '');
        const entry = iconMap.get(base) || { hasLight: false, hasDark: false, hasBase: false };
        entry.hasDark = true;
        iconMap.set(base, entry);
      } else {
        // Standalone icon without theme variants
        const entry = iconMap.get(icon) || { hasLight: false, hasDark: false, hasBase: false };
        entry.hasBase = true;
        iconMap.set(icon, entry);
      }
    }
    
    // Only include icons that have a base file OR both light and dark variants
    const icons = Array.from(iconMap.entries())
      .filter(([_, variants]) => variants.hasBase || (variants.hasLight && variants.hasDark))
      .map(([name]) => name);
    
    return NextResponse.json(icons, {
      headers: {
        'Cache-Control': 'public, max-age=3600',
      },
    });
  } catch (error) {
    console.error('Failed to list SVGL icons:', error);
    return new NextResponse('Internal Server Error', { status: 500 });
  }
}
