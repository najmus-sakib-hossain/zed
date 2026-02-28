'use client';

import { useState, useMemo, useRef, useEffect } from 'react';
import { FolderHeart, Trash } from 'lucide-react';
import Fuse from 'fuse.js';
import { useFavoritesQuery, useClearFavorites } from '@/hooks/use-favorites-query';
import { SearchInput } from '@/components/search-input';
import { SvgCard } from '@/components/svg-card';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';

export default function FavoritesPage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [sorted, setSorted] = useState(false);
  const [displayCount, setDisplayCount] = useState(30);
  const { data: favorites = [], isLoading } = useFavoritesQuery();
  const clearFavorites = useClearFavorites();
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);

  const fuse = useMemo(
    () =>
      new Fuse(favorites, {
        keys: ['title', 'category'],
        threshold: 0.3,
      }),
    [favorites]
  );

  const filteredFavorites = useMemo(() => {
    let result = searchTerm.trim() ? fuse.search(searchTerm).map((r) => r.item) : favorites;

    if (sorted) {
      result = [...result].sort((a, b) => a.title.localeCompare(b.title));
    } else {
      result = [...result].sort((a, b) => b.title.localeCompare(a.title));
    }

    return result;
  }, [searchTerm, favorites, fuse, sorted]);

  const displayFavorites = useMemo(() => {
    return filteredFavorites.slice(0, displayCount);
  }, [filteredFavorites, displayCount]);

  const hasMore = displayCount < filteredFavorites.length;

  // Infinite scroll observer
  useEffect(() => {
    const currentRef = loadMoreRef.current;
    if (!currentRef || !hasMore) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          setDisplayCount((prev) => prev + 30);
        }
      },
      { threshold: 0.5, rootMargin: '200px' }
    );

    observer.observe(currentRef);

    return () => {
      if (currentRef) {
        observer.unobserve(currentRef);
      }
      observer.disconnect();
    };
  }, [hasMore, displayCount, filteredFavorites.length]);

  // Reset display count when search changes
  useEffect(() => {
    setDisplayCount(30);
  }, [searchTerm, sorted]);

  const handleClearAll = () => {
    clearFavorites.mutate();
  };

  if (isLoading) {
    return (
      <div className="h-[calc(100vh-6rem)] bg-background rounded-md">
        <ScrollArea className="h-full rounded-md border">
          <div className="flex items-center justify-center min-h-[60vh]">
            <div className="flex flex-col items-center gap-3">
              <div className="animate-spin h-8 w-8 border-4 border-primary border-t-transparent rounded-full" />
              <p className="text-muted-foreground">Loading favorites...</p>
            </div>
          </div>
        </ScrollArea>
      </div>
    );
  }

  return (
    <div>
      <div className="h-[calc(100vh-6rem)] bg-background rounded-md">
        <ScrollArea ref={scrollAreaRef} className="h-full rounded-md border">
          <div className="p-2 px-0 pt-0 min-h-[calc(100vh-6rem)] flex flex-col">
            <div className="flex items-center">
              <div className="flex-1">
                <SearchInput
                  value={searchTerm}
                  onChange={setSearchTerm}
                  placeholder="Search favorites..."
                  totalIcons={favorites.length}
                  filteredCount={filteredFavorites.length}
                  sorted={sorted}
                  onSortToggle={() => setSorted(!sorted)}
                  onClearSearch={() => setSearchTerm('')}
                  showSortButton={filteredFavorites.length > 0}
                />
              </div>
              {favorites.length > 0 && (
                <Button
                  variant="outline"
                  onClick={handleClearAll}
                  size="sm"
                  disabled={clearFavorites.isPending}
                  className="shrink-0 mr-2 h-10"
                >
                 Clear All
                </Button>
              )}
            </div>

            {displayFavorites.length > 0 ? (
              <div className="px-2">
                <div className="mt-0 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-2">
                  {displayFavorites.map((svg) => (
                    <div key={svg.id}>
                      <SvgCard svg={svg} />
                    </div>
                  ))}
                </div>

                {hasMore && (
                  <div ref={loadMoreRef} className="mt-6 flex justify-center py-4">
                    <div className="animate-pulse text-muted-foreground">
                      Loading more favorites...
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="flex-1 flex items-center justify-center px-2">
                <div className="flex flex-col items-center gap-4 text-center">
                  {searchTerm ? (
                    <>
                      <p className="text-muted-foreground">
                        No favorites found matching &quot;{searchTerm}&quot;
                      </p>
                    </>
                  ) : (
                    <>
                      <FolderHeart className="h-12 w-12 text-muted-foreground" strokeWidth={1} />
                      <div>
                        <h2 className="text-xl font-semibold mb-2">No favorites yet</h2>
                        <p className="text-muted-foreground">
                          Double-click any icon to add it to your favorites
                        </p>
                      </div>
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
