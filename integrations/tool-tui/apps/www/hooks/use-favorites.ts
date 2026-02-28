'use client';

import { useState, useEffect } from 'react';
import { favoritesStore } from '@/lib/favorites-store';
import type { iSVG } from '@/types/svg';

export function useFavorites() {
  const [favorites, setFavorites] = useState<iSVG[]>([]);

  useEffect(() => {
    setFavorites(favoritesStore.getFavorites());
    const unsubscribe = favoritesStore.subscribe(() => {
      setFavorites(favoritesStore.getFavorites());
    });
    return unsubscribe;
  }, []);

  return {
    favorites,
    isFavorite: (svg: iSVG) => favoritesStore.isFavorite(svg),
    toggleFavorite: (svg: iSVG) => favoritesStore.toggleFavorite(svg),
    clearFavorites: () => favoritesStore.clearFavorites(),
    count: favorites.length,
  };
}
