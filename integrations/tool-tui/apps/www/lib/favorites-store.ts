'use client';

import type { iSVG } from '@/types/svg';
import { svgs } from '@/data/svgs';

const STORAGE_KEY = 'svgl_favorites';

class FavoritesStore {
  private listeners: Set<() => void> = new Set();

  private validateFavorites(favorites: iSVG[]): iSVG[] {
    return favorites.filter((favorite) =>
      svgs.some(
        (svg) =>
          svg.title === favorite.title &&
          JSON.stringify(svg.route) === JSON.stringify(favorite.route)
      )
    );
  }

  private loadFavorites(): iSVG[] {
    if (typeof window === 'undefined') return [];
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const storedFavorites: iSVG[] = JSON.parse(stored);
        const validatedFavorites = this.validateFavorites(storedFavorites);
        if (validatedFavorites.length !== storedFavorites.length) {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(validatedFavorites));
        }
        return validatedFavorites;
      }
      return [];
    } catch (error) {
      console.error('Error loading favorites:', error);
      return [];
    }
  }

  private saveFavorites(favorites: iSVG[]) {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(favorites));
      this.notifyListeners();
    } catch (error) {
      console.error('Error saving favorites:', error);
    }
  }

  private notifyListeners() {
    this.listeners.forEach((listener) => listener());
  }

  subscribe(listener: () => void) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  getFavorites(): iSVG[] {
    return this.loadFavorites();
  }

  isFavorite(item: iSVG): boolean {
    const favorites = this.loadFavorites();
    return favorites.some(
      (fav) =>
        fav.title === item.title &&
        JSON.stringify(fav.route) === JSON.stringify(item.route)
    );
  }

  toggleFavorite(item: iSVG) {
    const favorites = this.loadFavorites();
    const exists = favorites.some(
      (fav) =>
        fav.title === item.title &&
        JSON.stringify(fav.route) === JSON.stringify(item.route)
    );

    const newFavorites = exists
      ? favorites.filter(
          (fav) =>
            !(
              fav.title === item.title &&
              JSON.stringify(fav.route) === JSON.stringify(item.route)
            )
        )
      : [...favorites, item];

    this.saveFavorites(newFavorites);
  }

  clearFavorites() {
    this.saveFavorites([]);
  }

  getCount(): number {
    return this.loadFavorites().length;
  }
}

export const favoritesStore = new FavoritesStore();
