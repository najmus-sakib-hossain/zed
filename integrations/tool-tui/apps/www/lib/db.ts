'use client';

import { PGlite } from '@electric-sql/pglite';

let db: PGlite | null = null;

export async function getDb() {
  if (db) return db;

  if (typeof window === 'undefined') {
    throw new Error('PGlite can only be used in the browser');
  }

  db = new PGlite('idb://svgl-db');

  await db.exec(`
    CREATE TABLE IF NOT EXISTS favorites (
      id SERIAL PRIMARY KEY,
      svg_id INTEGER NOT NULL,
      title TEXT NOT NULL,
      category TEXT NOT NULL,
      route TEXT NOT NULL,
      wordmark TEXT,
      brand_url TEXT,
      url TEXT NOT NULL,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(svg_id)
    );

    CREATE INDEX IF NOT EXISTS idx_favorites_svg_id ON favorites(svg_id);
    CREATE INDEX IF NOT EXISTS idx_favorites_created_at ON favorites(created_at DESC);
  `);

  return db;
}

export async function clearDb() {
  const database = await getDb();
  await database.exec('DROP TABLE IF EXISTS favorites CASCADE;');
  db = null;
}
