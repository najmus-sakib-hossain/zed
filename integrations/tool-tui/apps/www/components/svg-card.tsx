'use client';

import { useState, useEffect } from 'react';
import { useTheme } from 'next-themes';
import { motion } from 'framer-motion';
import {
  Copy,
  Download,
  Link as LinkIcon,
  Palette,
  Check,
  Sparkles,
  Baseline,
  Tag,
  MoreHorizontal,
  X,
  Heart,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { cn } from '@/lib/utils';
import { useToggleFavorite, useIsFavorite } from '@/hooks/use-favorites-query';
import { fetchAndOptimizeSvg } from '@/lib/svg-optimizer';
import { toast } from 'sonner';
import type { iSVG } from '@/types/svg';
import Link from 'next/link';
import { CopyModal } from '@/components/copy-modal';

// SVG Icon Component with theme support
function SvgIcon({ src, alt, category }: { src: string; alt: string; category?: string | string[] }) {
  const [svgContent, setSvgContent] = useState<string | null>(null);
  const { theme, systemTheme } = useTheme();
  
  const pack = typeof category === 'string' ? category : category?.[0];
  const currentTheme = theme === 'system' ? systemTheme : theme;
  
  useEffect(() => {
    // For SVGL icons, use img tag (direct static files)
    if (!pack || pack === 'Software') {
      setSvgContent(null);
      return;
    }

    // Load icon data directly from public/icons/*.json
    async function loadIconFromJSON() {
      try {
        // Extract icon name from src (format: /api/icons/pack/name or /api/icons/pack/name?v=3)
        const match = src.match(/\/api\/icons\/([^/]+)\/([^?]+)/);
        if (!match) {
          return;
        }
        
        const [, iconPack, iconName] = match;
        
        // Load JSON file
        const response = await fetch(`/icons/${iconPack}.json`);
        if (!response.ok) {
          console.error('Failed to load pack:', iconPack);
          return;
        }
        
        const data = await response.json();
        const iconData = data.icons?.[iconName];
        
        if (!iconData) {
          console.error('Icon not found:', iconName, 'in pack:', iconPack);
          return;
        }
        
        // Get dimensions - check multiple locations
        // 1. Root level width/height (ant-design uses this)
        // 2. info.height (most packs use this)
        // 3. Icon-specific width/height
        const rootWidth = data.width;
        const rootHeight = data.height;
        const packHeight = data.info?.height || rootHeight || 24;
        
        // For icons with custom width, calculate proper height
        // If icon has width but no height, use root height or 512 as default
        const iconWidth = iconData.width;
        const iconHeight = iconData.height;
        
        let width, height;
        
        if (iconWidth && !iconHeight) {
          // Icon has custom width but no height - use root height or default to 512
          width = iconWidth;
          height = rootHeight || 512;
        } else if (iconWidth && iconHeight) {
          // Icon has both dimensions
          width = iconWidth;
          height = iconHeight;
        } else {
          // Use pack defaults
          width = rootWidth || packHeight;
          height = rootHeight || packHeight;
        }
        
        const viewBox = `0 0 ${width} ${height}`;
        
        // Build SVG
        let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}" fill="currentColor" width="40" height="40">${iconData.body}</svg>`;
        
        setSvgContent(svg);
      } catch (error) {
        console.error('Failed to load icon:', error);
      }
    }

    loadIconFromJSON();
  }, [src, pack]);

  // SVGL icons: use img tag with suppressHydrationWarning for theme-dependent images
  if (!pack || pack === 'Software') {
    return (
      <img
        src={src}
        alt={alt}
        title={alt}
        height="40"
        loading="eager"
        decoding="async"
        className="mb-4 mt-1.5 h-10 select-none pointer-events-none"
        suppressHydrationWarning
        style={{ contentVisibility: 'auto' }}
      />
    );
  }

  // Other icons: render as inline SVG
  if (svgContent) {
    return (
      <div
        className="mb-4 mt-1.5 w-10 h-10 flex items-center justify-center select-none pointer-events-none"
        dangerouslySetInnerHTML={{ __html: svgContent }}
      />
    );
  }

  return null;
}

interface SvgCardProps {
  svg: iSVG;
}

export function SvgCard({ svg }: SvgCardProps) {
  const [copied, setCopied] = useState(false);
  const [showWordmark, setShowWordmark] = useState(false);
  const [moreTagsOpen, setMoreTagsOpen] = useState(false);
  const [copyModalOpen, setCopyModalOpen] = useState(false);
  const [localIsFav, setLocalIsFav] = useState(false);
  const [clickTimeout, setClickTimeout] = useState<NodeJS.Timeout | null>(null);
  const { theme, systemTheme } = useTheme();
  const toggleFavorite = useToggleFavorite();
  const { data: isFav = false } = useIsFavorite(svg.id);

  // Sync local state with server state
  useEffect(() => {
    setLocalIsFav(isFav);
  }, [isFav]);

  const handleCardClick = () => {
    if (clickTimeout) {
      // Double click detected
      clearTimeout(clickTimeout);
      setClickTimeout(null);
      handleToggleFavorite();
    } else {
      // Single click - wait to see if double click follows
      const timeout = setTimeout(async () => {
        // Copy SVG to clipboard
        try {
          const optimizeSvgs = localStorage.getItem('optimizeSvgs') === 'true';
          if (optimizeSvgs) {
            const optimizedSvg = await fetchAndOptimizeSvg(imageSrc);
            await navigator.clipboard.writeText(optimizedSvg);
            toast.success('Optimized SVG copied to clipboard!');
          } else {
            await navigator.clipboard.writeText(imageSrc);
            toast.success('SVG URL copied to clipboard!');
          }
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        } catch (error) {
          await navigator.clipboard.writeText(imageSrc);
          toast.success('SVG URL copied to clipboard!');
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        }
        setClickTimeout(null);
      }, 250);
      setClickTimeout(timeout);
    }
  };

  const currentTheme = theme === 'system' ? systemTheme : theme;
  const isDark = currentTheme === 'dark';

  const maxVisibleCategories = 1;
  const categories = Array.isArray(svg.category) ? svg.category : [svg.category];
  const visibleCategories = categories.slice(0, maxVisibleCategories);
  const hiddenCategories = categories.slice(maxVisibleCategories);

  const getImageUrl = (route: string | { light: string; dark: string }) => {
    if (typeof route === 'string') return route;
    return isDark ? route.dark : route.light;
  };

  const currentRoute = showWordmark && svg.wordmark ? svg.wordmark : svg.route;
  const imageUrl = getImageUrl(currentRoute);
  
  // Add cache buster for proper loading
  const imageSrc = `${imageUrl}?v=3`;

  const handleCopy = async () => {
    setCopyModalOpen(true);
  };

  const handleDownload = async () => {
    try {
      const optimizedSvg = await fetchAndOptimizeSvg(imageSrc);
      const blob = new Blob([optimizedSvg], { type: 'image/svg+xml' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `${svg.title.toLowerCase().replace(/\s+/g, '-')}.svg`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
      toast.success('Optimized SVG downloaded!');
    } catch (error) {
      const link = document.createElement('a');
      link.href = imageSrc;
      link.download = `${svg.title.toLowerCase().replace(/\s+/g, '-')}.svg`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      toast.success('Downloaded!');
    }
  };

  const handleToggleFavorite = async () => {
    // Optimistic update - instant UI feedback
    const newState = !localIsFav;
    setLocalIsFav(newState);
    toast.success(newState ? 'Added to favorites' : 'Removed from favorites');
    
    // Trigger mutation
    toggleFavorite.mutate(svg, {
      onError: () => {
        // Revert on error
        setLocalIsFav(!newState);
        toast.error('Failed to update favorites');
      }
    });
  };

  return (
    <div
      className="group flex flex-col items-center justify-center px-3.5 py-3 rounded-md border hover:bg-input/30 transition-colors cursor-pointer select-none"
      onClick={handleCardClick}
    >
      {/* Top Actions */}
      <div className="flex w-full items-center justify-end space-x-3 pb-0.5">
        {svg.brandUrl && (
          <a
            href={svg.brandUrl}
            title="Brand Assets"
            target="_blank"
            rel="noopener noreferrer"
            onClick={(e) => e.stopPropagation()}
            className="text-muted-foreground hover:text-foreground transition-colors opacity-0 group-hover:opacity-100"
          >
            <Palette className="h-4 w-4" strokeWidth={1.8} />
          </a>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={(e) => {
            e.stopPropagation();
            handleToggleFavorite();
          }}
          title={localIsFav ? 'Remove from favorites' : 'Add to favorites'}
          className={cn(
            "h-auto w-auto p-0 hover:bg-transparent transition-opacity",
            localIsFav ? "opacity-100" : "opacity-0 group-hover:opacity-100"
          )}
        >
          <div className="flex items-center justify-center">
            <Heart
              className={cn(
                'h-4 w-4 transition-all',
                localIsFav ? 'fill-red-500 text-red-500' : 'text-muted-foreground hover:text-foreground'
              )}
              strokeWidth={1.8}
            />
          </div>
        </Button>
      </div>

      {/* Image */}
      <div className="mb-4 mt-1.5 h-10 flex items-center justify-center">
        <SvgIcon src={imageSrc} alt={svg.title} category={svg.category} />
      </div>

      {/* Title & Categories */}
      <div className="mb-3 flex flex-col items-center justify-center space-y-1 w-full px-1">
        <p className="truncate text-center text-base font-medium w-full select-none" title={svg.title}>
          {svg.title}
        </p>
        <div className="flex items-center justify-center space-x-1">
          {visibleCategories.map((cat) => (
            <Link key={cat} href={`/directory/${cat.toLowerCase()}`} onClick={(e) => e.stopPropagation()}>
              <Badge
                variant="outline"
                className="cursor-pointer font-mono hover:border-foreground/40 select-none"
              >
                {cat}
              </Badge>
            </Link>
          ))}
          {hiddenCategories.length > 0 && (
            <Popover open={moreTagsOpen} onOpenChange={setMoreTagsOpen}>
              <PopoverTrigger asChild onClick={(e) => e.stopPropagation()}>
                <Badge
                  variant="outline"
                  className="cursor-pointer font-mono hover:border-foreground/40 select-none"
                >
                  {moreTagsOpen ? (
                    <X className="h-3 w-3" strokeWidth={1.5} />
                  ) : (
                    <MoreHorizontal className="h-3 w-3" strokeWidth={1.5} />
                  )}
                </Badge>
              </PopoverTrigger>
              <PopoverContent className="w-auto">
                <p className="font-medium mb-2 select-none">More tags</p>
                <div className="flex flex-col space-y-2">
                  {hiddenCategories.map((cat) => (
                    <Link key={cat} href={`/directory/${cat.toLowerCase()}`}>
                      <Button variant="outline" className="w-full justify-start select-none">
                        <Tag className="h-4 w-4 mr-2" strokeWidth={1.5} />
                        {cat}
                      </Button>
                    </Link>
                  ))}
                </div>
              </PopoverContent>
            </Popover>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center space-x-0.5" onClick={(e) => e.stopPropagation()}>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleCopy}
          title="Show copy options"
          className="hover:bg-accent"
        >
          {copied ? (
            <Check className="h-4 w-4 text-green-500" strokeWidth={1.8} />
          ) : (
            <Copy className="h-4 w-4" strokeWidth={1.8} />
          )}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleDownload}
          title="Download SVG"
          className="hover:bg-accent"
        >
          <Download className="h-4 w-4" strokeWidth={1.8} />
        </Button>
        <Button variant="ghost" size="icon" asChild title="Website" className="hover:bg-accent">
          <a href={svg.url} target="_blank" rel="noopener noreferrer">
            <LinkIcon className="h-4 w-4" strokeWidth={1.8} />
          </a>
        </Button>
        {svg.wordmark && (
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setShowWordmark(!showWordmark)}
            title={showWordmark ? 'Show logo' : 'Show wordmark'}
            className="hover:bg-accent"
          >
            {showWordmark ? (
              <Sparkles className="h-4 w-4" strokeWidth={1.8} />
            ) : (
              <Baseline className="h-4 w-4" strokeWidth={1.8} />
            )}
          </Button>
        )}
      </div>

      <CopyModal
        open={copyModalOpen}
        onOpenChange={setCopyModalOpen}
        svgUrl={imageSrc}
        iconName={svg.title}
      />
    </div>
  );
}
