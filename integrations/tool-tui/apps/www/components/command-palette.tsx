'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { Command } from 'cmdk';
import { Search, Home, Heart, Github } from 'lucide-react';
import { svgs } from '@/data/svgs';
import { useUIStore } from '@/store/ui-store';

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const router = useRouter();
  const setSearchTerm = useUIStore((state) => state.setSearchTerm);

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((open) => !open);
      }
    };

    document.addEventListener('keydown', down);
    return () => document.removeEventListener('keydown', down);
  }, []);

  const handleSelect = (value: string) => {
    if (value.startsWith('search:')) {
      const term = value.replace('search:', '');
      setSearchTerm(term);
      router.push('/');
    } else if (value === 'home') {
      router.push('/');
    } else if (value === 'favorites') {
      router.push('/favorites');
    } else if (value === 'github') {
      window.open('https://github.com/dx-rs/dx', '_blank');
    }
    setOpen(false);
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/50" onClick={() => setOpen(false)}>
      <div className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-lg">
        <Command className="rounded-lg border shadow-md bg-background">
          <Command.Input placeholder="Search icons or navigate..." className="border-b px-3 py-3" />
          <Command.List className="max-h-[300px] overflow-y-auto p-2">
            <Command.Empty className="py-6 text-center text-sm text-muted-foreground">
              No results found.
            </Command.Empty>
            <Command.Group heading="Navigation">
              <Command.Item onSelect={() => handleSelect('home')} className="flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded hover:bg-accent">
                <Home className="h-4 w-4" />
                <span>Home</span>
              </Command.Item>
              <Command.Item onSelect={() => handleSelect('favorites')} className="flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded hover:bg-accent">
                <Heart className="h-4 w-4" />
                <span>Favorites</span>
              </Command.Item>
              <Command.Item onSelect={() => handleSelect('github')} className="flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded hover:bg-accent">
                <Github className="h-4 w-4" />
                <span>GitHub</span>
              </Command.Item>
            </Command.Group>
            <Command.Group heading="Icons">
              {svgs.slice(0, 10).map((svg) => (
                <Command.Item
                  key={svg.id}
                  onSelect={() => handleSelect(`search:${svg.title}`)}
                  className="flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded hover:bg-accent"
                >
                  <Search className="h-4 w-4" />
                  <span>{svg.title}</span>
                </Command.Item>
              ))}
            </Command.Group>
          </Command.List>
        </Command>
      </div>
    </div>
  );
}
