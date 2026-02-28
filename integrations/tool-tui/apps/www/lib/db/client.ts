'use client';

import { PGlite } from '@electric-sql/pglite';
import { drizzle } from 'drizzle-orm/pglite';
import * as schema from './schema';

let db: ReturnType<typeof drizzle> | null = null;
let pg: PGlite | null = null;
let initPromise: Promise<ReturnType<typeof drizzle>> | null = null;

export async function getDb() {
  if (db) return db;
  
  // Prevent concurrent initialization
  if (initPromise) return initPromise;

  initPromise = (async () => {
    // Initialize PGlite with IndexedDB persistence
    pg = new PGlite('idb://dx-icons-db');
    
    // Wait for PGlite to be ready
    await pg.waitReady;
    
    // Create tables if they don't exist
    await pg.exec(`
      CREATE TABLE IF NOT EXISTS icon_packs (
        name TEXT PRIMARY KEY,
        icon_count INTEGER NOT NULL,
        icons TEXT NOT NULL,
        last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
      );

      CREATE TABLE IF NOT EXISTS icon_cache (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        expires_at TIMESTAMP
      );

      CREATE TABLE IF NOT EXISTS favorites (
        svg_id INTEGER PRIMARY KEY,
        title TEXT NOT NULL,
        category TEXT NOT NULL,
        route TEXT NOT NULL,
        wordmark TEXT,
        brand_url TEXT,
        url TEXT NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
      );
    `);

    db = drizzle(pg, { schema });
    return db;
  })();

  return initPromise;
}

export async function clearCache() {
  const database = await getDb();
  await database.delete(schema.iconPacks);
  await database.delete(schema.iconCache);
}
