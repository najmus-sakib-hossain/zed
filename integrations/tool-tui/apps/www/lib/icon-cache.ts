'use client';

import { getDb } from './db/client';
import { iconPacks, iconCache } from './db/schema';
import { eq } from 'drizzle-orm';

interface IconPackData {
  name: string;
  iconCount: number;
  icons: string[];
}

// Cache icon pack data in PGlite
export async function cacheIconPack(packName: string, icons: string[]) {
  try {
    const db = await getDb();
    
    await db.insert(iconPacks)
      .values({
        name: packName,
        iconCount: icons.length,
        icons: JSON.stringify(icons),
        lastUpdated: new Date(),
      })
      .onConflictDoUpdate({
        target: iconPacks.name,
        set: {
          iconCount: icons.length,
          icons: JSON.stringify(icons),
          lastUpdated: new Date(),
        },
      });
  } catch (error) {
    console.error(`Failed to cache ${packName}:`, error);
  }
}

// Get cached icon pack data
export async function getCachedIconPack(packName: string): Promise<IconPackData | null> {
  try {
    const db = await getDb();
    const result = await db.select()
      .from(iconPacks)
      .where(eq(iconPacks.name, packName))
      .limit(1);
    
    if (result.length === 0) return null;
    
    const pack = result[0];
    return {
      name: pack.name,
      iconCount: pack.iconCount,
      icons: JSON.parse(pack.icons),
    };
  } catch (error) {
    console.error(`Failed to get cached ${packName}:`, error);
    return null;
  }
}

// Get all cached icon packs
export async function getAllCachedPacks(): Promise<IconPackData[]> {
  try {
    const db = await getDb();
    const results = await db.select().from(iconPacks);
    
    return results.map(pack => ({
      name: pack.name,
      iconCount: pack.iconCount,
      icons: JSON.parse(pack.icons),
    }));
  } catch (error) {
    console.error('Failed to get all cached packs:', error);
    return [];
  }
}

// Check if all packs are cached
export async function areAllPacksCached(): Promise<boolean> {
  try {
    const db = await getDb();
    const results = await db.select().from(iconPacks);
    // We expect 228 packs
    return results.length >= 228;
  } catch (error) {
    return false;
  }
}

// Generic cache functions
export async function setCache(key: string, value: any, expiresInMs?: number) {
  try {
    const db = await getDb();
    const expiresAt = expiresInMs ? new Date(Date.now() + expiresInMs) : null;
    
    await db.insert(iconCache)
      .values({
        key,
        value: JSON.stringify(value),
        expiresAt,
      })
      .onConflictDoUpdate({
        target: iconCache.key,
        set: {
          value: JSON.stringify(value),
          expiresAt,
        },
      });
  } catch (error) {
    console.error(`Failed to set cache for ${key}:`, error);
  }
}

export async function getCache<T>(key: string): Promise<T | null> {
  try {
    const db = await getDb();
    const result = await db.select()
      .from(iconCache)
      .where(eq(iconCache.key, key))
      .limit(1);
    
    if (result.length === 0) return null;
    
    const cached = result[0];
    
    // Check expiration
    if (cached.expiresAt && cached.expiresAt < new Date()) {
      await db.delete(iconCache).where(eq(iconCache.key, key));
      return null;
    }
    
    return JSON.parse(cached.value) as T;
  } catch (error) {
    console.error(`Failed to get cache for ${key}:`, error);
    return null;
  }
}
