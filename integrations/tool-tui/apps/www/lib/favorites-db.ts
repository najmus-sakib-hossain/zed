'use client';

import { getDb } from './db';
import type { iSVG } from '@/types/svg';

export async function getFavorites(): Promise<iSVG[]> {
  const db = await getDb();
  const result = await db.query<{
    svg_id: number;
    title: string;
    category: string;
    route: string;
    wordmark: string | null;
    brand_url: string | null;
    url: string;
  }>('SELECT * FROM favorites ORDER BY created_at DESC');

  return result.rows.map((row) => ({
    id: row.svg_id,
    title: row.title,
    category: JSON.parse(row.category),
    route: JSON.parse(row.route),
    wordmark: row.wordmark ? JSON.parse(row.wordmark) : undefined,
    brandUrl: row.brand_url || undefined,
    url: row.url,
  }));
}

export async function addFavorite(svg: iSVG): Promise<void> {
  const db = await getDb();
  await db.query(
    `INSERT INTO favorites (svg_id, title, category, route, wordmark, brand_url, url)
     VALUES ($1, $2, $3, $4, $5, $6, $7)
     ON CONFLICT (svg_id) DO NOTHING`,
    [
      svg.id,
      svg.title,
      JSON.stringify(svg.category),
      JSON.stringify(svg.route),
      svg.wordmark ? JSON.stringify(svg.wordmark) : null,
      svg.brandUrl || null,
      svg.url,
    ]
  );
}

export async function removeFavorite(svgId: number): Promise<void> {
  const db = await getDb();
  await db.query('DELETE FROM favorites WHERE svg_id = $1', [svgId]);
}

export async function isFavorite(svgId: number): Promise<boolean> {
  const db = await getDb();
  const result = await db.query<{ count: string }>(
    'SELECT COUNT(*) as count FROM favorites WHERE svg_id = $1',
    [svgId]
  );
  return parseInt(result.rows[0].count) > 0;
}

export async function clearAllFavorites(): Promise<void> {
  const db = await getDb();
  await db.query('DELETE FROM favorites');
}

export async function getFavoritesCount(): Promise<number> {
  const db = await getDb();
  const result = await db.query<{ count: string }>(
    'SELECT COUNT(*) as count FROM favorites'
  );
  return parseInt(result.rows[0].count);
}
