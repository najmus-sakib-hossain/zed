'use client';

import { useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { IconCard } from './icon-card';
import type { IconEntry } from '@/lib/icon-loader';

interface IconGridProps {
  icons: IconEntry[];
  columns?: number;
}

export function IconGrid({ icons, columns = 6 }: IconGridProps) {
  const parentRef = useRef<HTMLDivElement>(null);
  
  // Calculate rows
  const rows = Math.ceil(icons.length / columns);
  
  const virtualizer = useVirtualizer({
    count: rows,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 140, // Row height
    overscan: 5,
  });
  
  return (
    <div ref={parentRef} className="h-[calc(100vh-200px)] overflow-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const startIdx = virtualRow.index * columns;
          const rowIcons = icons.slice(startIdx, startIdx + columns);
          
          return (
            <div
              key={virtualRow.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              className="grid gap-4"
              style={{
                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
              }}
            >
              {rowIcons.map((icon) => (
                <IconCard key={`${icon.pack}-${icon.name}`} icon={icon} />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
