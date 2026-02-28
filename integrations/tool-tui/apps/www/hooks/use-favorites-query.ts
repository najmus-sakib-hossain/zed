'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getFavorites,
  addFavorite,
  removeFavorite,
  clearAllFavorites,
  getFavoritesCount,
  isFavorite,
} from '@/lib/favorites-db';
import type { iSVG } from '@/types/svg';

export function useFavoritesQuery() {
  return useQuery({
    queryKey: ['favorites'],
    queryFn: getFavorites,
    staleTime: 1000 * 60 * 5, // 5 minutes
  });
}

export function useFavoritesCount() {
  return useQuery({
    queryKey: ['favorites', 'count'],
    queryFn: getFavoritesCount,
    staleTime: 1000 * 60 * 5,
  });
}

export function useIsFavorite(svgId: number | undefined) {
  return useQuery({
    queryKey: ['favorites', 'check', svgId],
    queryFn: () => (svgId ? isFavorite(svgId) : false),
    enabled: !!svgId,
    staleTime: 1000 * 60 * 5,
  });
}

export function useAddFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: addFavorite,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
    },
  });
}

export function useRemoveFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: removeFavorite,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
    },
  });
}

export function useClearFavorites() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clearAllFavorites,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
    },
  });
}

export function useToggleFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (svg: iSVG) => {
      if (!svg.id) return { id: svg.id, isFav: false };
      const isFav = await isFavorite(svg.id);
      if (isFav) {
        await removeFavorite(svg.id);
      } else {
        await addFavorite(svg);
      }
      return { id: svg.id, isFav: !isFav };
    },
    onSuccess: () => {
      // Invalidate all favorites queries to refetch
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
    },
  });
}
