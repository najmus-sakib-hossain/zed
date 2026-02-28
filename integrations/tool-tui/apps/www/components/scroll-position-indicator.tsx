'use client';

import { useEffect, useState } from 'react';
import { motion, AnimatePresence, useMotionValue, useSpring } from 'framer-motion';

interface ScrollPositionIndicatorProps {
  scrollAreaRef: React.RefObject<HTMLDivElement | null>;
  totalItems: number;
  itemsPerRow: number;
}

export function ScrollPositionIndicator({
  scrollAreaRef,
  totalItems,
  itemsPerRow,
}: ScrollPositionIndicatorProps) {
  const [scrollInfo, setScrollInfo] = useState({
    percentage: 0,
    currentRow: 0,
    totalRows: 0,
    isScrolling: false,
  });

  const topPosition = useMotionValue(60);
  const smoothTop = useSpring(topPosition, { stiffness: 400, damping: 30 });

  useEffect(() => {
    const viewport = scrollAreaRef.current?.querySelector('[data-slot="scroll-area-viewport"]');
    if (!viewport) return;

    let scrollTimeout: NodeJS.Timeout;
    const totalRows = Math.ceil(totalItems / itemsPerRow);

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = viewport;
      const maxScroll = scrollHeight - clientHeight;
      const percentage = maxScroll > 0 ? (scrollTop / maxScroll) * 100 : 0;
      const currentRow = Math.floor((scrollTop / scrollHeight) * totalRows) + 1;
      
      // Calculate position to follow scroll thumb accurately
      // Account for the indicator height and padding
      const indicatorHeight = 100; // Approximate height of indicator
      const availableHeight = clientHeight - indicatorHeight - 40; // Padding top and bottom
      const calculatedTop = maxScroll > 0 ? (scrollTop / maxScroll) * availableHeight + 20 : 20;
      
      topPosition.set(calculatedTop);

      setScrollInfo({
        percentage: Math.min(100, Math.max(0, percentage)),
        currentRow: Math.min(totalRows, Math.max(1, currentRow)),
        totalRows,
        isScrolling: true,
      });

      clearTimeout(scrollTimeout);
      scrollTimeout = setTimeout(() => {
        setScrollInfo((prev) => ({ ...prev, isScrolling: false }));
      }, 1500);
    };

    viewport.addEventListener('scroll', handleScroll);
    handleScroll();

    return () => {
      viewport.removeEventListener('scroll', handleScroll);
      clearTimeout(scrollTimeout);
    };
  }, [scrollAreaRef, totalItems, itemsPerRow, topPosition]);

  return (
    <AnimatePresence>
      {scrollInfo.isScrolling && (
        <motion.div
          initial={{ opacity: 0, x: -30, scale: 0.8 }}
          animate={{ 
            opacity: 1, 
            x: 0, 
            scale: 1,
          }}
          exit={{ opacity: 0, x: -30, scale: 0.8 }}
          transition={{ 
            type: "spring", 
            stiffness: 400, 
            damping: 25,
            bounce: 0.5
          }}
          style={{ top: smoothTop as any }}
          className="absolute right-[25px] z-40 pointer-events-none"
        >
          <div className="backdrop-blur-md bg-secondary/95 supports-[backdrop-filter]:bg-secondary/90 border-2 border-secondary rounded-xl px-5 py-3.5 shadow-2xl min-w-[140px]">
            <div className="text-sm font-mono space-y-1.5">
              {/* <div className="text-secondary-foreground font-bold text-base">
                Row {scrollInfo.currentRow} / {scrollInfo.totalRows}
              </div> */}
              <div className="text-xs text-secondary-foreground/80 font-semibold">
                {scrollInfo.percentage.toFixed(1)}% scrolled
              </div>
              <div className="h-1.5 bg-secondary-foreground/20 rounded-full overflow-hidden mt-2">
                <motion.div
                  className="h-full bg-secondary-foreground/70 rounded-full"
                  initial={{ width: 0 }}
                  animate={{ width: `${scrollInfo.percentage}%` }}
                  transition={{ duration: 0.3 }}
                />
              </div>
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
