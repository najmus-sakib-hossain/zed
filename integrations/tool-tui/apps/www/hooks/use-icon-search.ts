'use client';

import { useQuery } from '@tanstack/react-query';
import { searchIcons, fuzzySearchIcons, type IconEntry, type FuzzyMatch } from '@/lib/icon-loader';

export function useIconSearch(query: string, pack?: string, fuzzy = false) {
  return useQuery({
    queryKey: ['icons', query, pack, fuzzy],
    queryFn: async () => {
      if (fuzzy) {
        return fuzzySearchIcons(query, pack);
      }
      const result = await searchIcons(query, pack);
      return result.icons;
    },
    enabled: query.length > 0,
    staleTime: 5 * 60 * 1000, // 5min cache
  });
}

export function useIconSearchWithTiming(query: string, pack?: string) {
  return useQuery({
    queryKey: ['icons-timed', query, pack],
    queryFn: () => searchIcons(query, pack),
    enabled: query.length > 0,
    staleTime: 5 * 60 * 1000,
  });
}
