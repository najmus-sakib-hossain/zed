'use client';

import { useEffect, useRef, useState } from 'react';
import { Search, Command, X } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command';
import { cn } from '@/lib/utils';

interface SearchInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  totalIcons?: number;
  filteredCount?: number;
  sorted?: boolean;
  onSortToggle?: () => void;
  onClearSearch?: () => void;
  showSortButton?: boolean;
  selectedPack?: string;
  onPackChange?: (pack: string) => void;
  availablePacks?: string[];
}

export function SearchInput({ 
  value, 
  onChange, 
  placeholder = 'Search...',
  totalIcons,
  filteredCount,
  sorted,
  onSortToggle,
  onClearSearch,
  showSortButton = true,
  selectedPack = 'svgl',
  onPackChange,
  availablePacks = []
}: SearchInputProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [open, setOpen] = useState(false);

  // Build icon sets from available packs
  const iconSets = [
    { value: 'svgl', label: 'SVGL Icons' },
    ...availablePacks.map(pack => ({
      value: pack,
      label: `${pack.charAt(0).toUpperCase() + pack.slice(1)} Icons`
    })),
    { value: 'all', label: 'All Icon Sets' },
  ];

  useEffect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  }, []);

  const iconCount = value ? filteredCount : totalIcons;
  const iconText = value ? 'found' : 'icons';
  const selectedLabel = iconSets.find(set => set.value === selectedPack)?.label || 'All Icon Sets';

  return (
    <div className="px-2 sticky top-0 z-10 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 flex items-center py-2">
      <div className="flex items-center gap-2 w-full">
        <div className="relative flex-1">
          <Search
            className={cn(
              'pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 h-5 w-5 transition-colors',
              value ? 'text-foreground' : 'text-muted-foreground'
            )}
          />
          <Input
            ref={inputRef}
            type="text"
            placeholder={placeholder}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className={cn(
              "px-10 text-base h-auto py-2",
              value ? "pr-32" : "pr-24"
            )}
          />
          <div className="absolute top-1/2 right-2 flex -translate-y-1/2 items-center gap-1.5">
            {value && onClearSearch && (
              <Button
                variant="ghost"
                size="icon"
                onClick={onClearSearch}
                className="h-6 w-6 hover:bg-secondary"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            )}
            {totalIcons !== undefined && (
              <span className="text-xs text-muted-foreground font-mono whitespace-nowrap px-1.5 py-0.5 rounded bg-secondary/50">
                {iconCount} {iconText}
              </span>
            )}
            {!value && (
              <div className="flex items-center space-x-1 rounded-md px-1.5 py-0.5 text-xs text-muted-foreground border">
                <Command className="h-3 w-3" />
                <span className="select-none">K</span>
              </div>
            )}
          </div>
        </div>
        
        {onPackChange && (
          <>
            <Button
            className="h-10"
              variant="outline"
              onClick={() => setOpen(true)}
            >
              <span className="truncate">{selectedLabel}</span>
            </Button>
            
            <CommandDialog open={open} onOpenChange={setOpen}>
              <CommandInput placeholder="Search icon sets..." />
              <CommandList>
                <CommandEmpty>No icon set found.</CommandEmpty>
                <CommandGroup>
                  {iconSets.map((set) => (
                    <CommandItem
                      key={set.value}
                      value={set.value}
                      onSelect={(currentValue) => {
                        onPackChange(currentValue);
                        setOpen(false);
                      }}
                    >
                      {set.label}
                    </CommandItem>
                  ))}
                </CommandGroup>
              </CommandList>
            </CommandDialog>
          </>
        )}
        
        {showSortButton && filteredCount !== 0 && (
          <Button
            variant="outline"
            size="sm"
            onClick={onSortToggle}
            className="px-3 h-10 shrink-0"
          >
            {sorted ? 'DSC' : 'ASC'}
          </Button>
        )}
      </div>
    </div>
  );
}
