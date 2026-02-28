import { pgTable, text, integer, timestamp } from 'drizzle-orm/pg-core';

export const iconPacks = pgTable('icon_packs', {
  name: text('name').primaryKey(),
  iconCount: integer('icon_count').notNull(),
  icons: text('icons').notNull(), // JSON string of icon names
  lastUpdated: timestamp('last_updated').defaultNow().notNull(),
});

export const iconCache = pgTable('icon_cache', {
  key: text('key').primaryKey(),
  value: text('value').notNull(),
  expiresAt: timestamp('expires_at'),
});
