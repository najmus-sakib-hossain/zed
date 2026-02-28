'use client';

import { useMemo, useRef, useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import Fuse from 'fuse.js';
import { svgs as svglIcons } from '@/data/svgs';
import type { iSVG } from '@/types/svg';
import { SearchInput } from '@/components/search-input';
import { SvgCard } from '@/components/svg-card';
import { useUIStore } from '@/store/ui-store';
import { ScrollArea } from '@/components/ui/scroll-area';
import { loadAllIcons, getAvailablePacks } from '@/lib/dx-icon-adapter';
import { ICON_PACK_COUNTS } from '@/lib/icon-pack-counts';

export default function Home() {
  const { searchTerm, sorted, setSearchTerm, setSorted } = useUIStore();
  const [displayCount, setDisplayCount] = useState(30);
  const [selectedPack, setSelectedPack] = useState('svgl');
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);

  // Cache available packs
  const { data: availablePacks = [] } = useQuery({
    queryKey: ['availablePacks'],
    queryFn: getAvailablePacks,
    staleTime: Infinity,
    gcTime: Infinity,
    initialData: Object.keys(ICON_PACK_COUNTS).sort(), // Use static data immediately
  });

  // Cache all icons with persistent storage
  const { data: allIconsData = [], isLoading, isFetching } = useQuery({
    queryKey: ['allIcons'],
    queryFn: loadAllIcons,
    staleTime: Infinity,
    gcTime: Infinity,
    enabled: selectedPack !== 'svgl',
    placeholderData: (previousData) => previousData,
  });

  // Listen for pack selection from sidebar
  useEffect(() => {
    const handlePackSelection = (event: CustomEvent<string>) => {
      setSelectedPack(event.detail);
    };
    
    window.addEventListener('selectIconPack', handlePackSelection as EventListener);
    
    // Check URL for pack parameter on mount
    const urlParams = new URLSearchParams(window.location.search);
    const packParam = urlParams.get('pack');
    if (packParam) {
      setSelectedPack(packParam);
      // Clean up URL
      window.history.replaceState({}, '', '/');
    }
    
    return () => {
      window.removeEventListener('selectIconPack', handlePackSelection as EventListener);
    };
  }, []);

  // Get icons based on selected pack
  const allIcons = useMemo(() => {
    if (selectedPack === 'svgl') return svglIcons;
    if (selectedPack === 'all') return [...svglIcons, ...allIconsData];
    
    return allIconsData.filter((icon) => {
      const category = typeof icon.category === 'string' ? icon.category : icon.category[0];
      return category === selectedPack;
    });
  }, [selectedPack, allIconsData]);

  const fuse = useMemo(
    () =>
      new Fuse(allIcons, {
        keys: ['title', 'category'],
        threshold: 0.3,
      }),
    [allIcons]
  );

  const filteredSvgs = useMemo(() => {
    let result = searchTerm.trim() ? fuse.search(searchTerm).map((r) => r.item) : allIcons;
    
    if (sorted) {
      result = [...result].sort((a, b) => a.title.localeCompare(b.title));
    } else {
      result = [...result].sort((a, b) => b.title.localeCompare(a.title));
    }
    
    return result;
  }, [searchTerm, fuse, allIcons, sorted]);

  const displaySvgs = useMemo(() => {
    return filteredSvgs.slice(0, displayCount);
  }, [filteredSvgs, displayCount]);

  const hasMore = displayCount < filteredSvgs.length;

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
  }, [hasMore, displayCount, filteredSvgs.length]);

  // Reset display count when search or pack changes
  useEffect(() => {
    setDisplayCount(30);
  }, [searchTerm, sorted, selectedPack]);

  return (
    <div>
      <div className="h-[calc(100vh-6rem)] bg-background rounded-md">
        <ScrollArea ref={scrollAreaRef} className="h-full rounded-md border">
          <div className="p-2 px-0 pt-0 min-h-[calc(100vh-6rem)] flex flex-col">
            <SearchInput 
              value={searchTerm} 
              onChange={setSearchTerm} 
              placeholder="Search icons..."
              totalIcons={allIcons.length}
              filteredCount={filteredSvgs.length}
              sorted={sorted}
              onSortToggle={() => setSorted(!sorted)}
              onClearSearch={() => setSearchTerm('')}
              showSortButton={filteredSvgs.length > 0}
              selectedPack={selectedPack}
              onPackChange={setSelectedPack}
              availablePacks={availablePacks}
            />
            <div className="flex-1 px-2 flex flex-col">
              {(selectedPack !== 'svgl' && allIconsData.length === 0 && (isLoading || isFetching)) ? (
                <div className="flex items-center justify-center flex-1">
                  <div className="flex flex-col items-center gap-3">
                    <div className="animate-spin h-8 w-8 border-4 border-primary border-t-transparent rounded-full" />
                    <p className="text-muted-foreground">Loading icons...</p>
                  </div>
                </div>
              ) : displaySvgs.length > 0 ? (
                <>
                  <div key={selectedPack} className="mt-0 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-2">
                    {displaySvgs.map((svg) => (
                      <div key={svg.id}>
                        <SvgCard svg={svg} />
                      </div>
                    ))}
                  </div>

                  {hasMore && (
                    <div ref={loadMoreRef} className="mt-6 flex justify-center py-4">
                      <div className="animate-pulse text-muted-foreground">
                        Loading more icons...
                      </div>
                    </div>
                  )}
                </>
              ) : (
                <div className="flex items-center justify-center flex-1">
                  <div className="text-center">
                    <p className="text-muted-foreground">
                      No icons found matching &quot;{searchTerm}&quot;
                    </p>
                  </div>
                </div>
              )}
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
