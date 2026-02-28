'use client';

import { useState } from 'react';
import { Check, Copy, Download } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { loadIconSVG, type IconEntry } from '@/lib/icon-loader';
import { toast } from 'sonner';

interface IconCardProps {
  icon: IconEntry;
}

export function IconCard({ icon }: IconCardProps) {
  const [copied, setCopied] = useState(false);
  const [loading, setLoading] = useState(false);
  
  const handleCopy = async () => {
    try {
      setLoading(true);
      const svg = await loadIconSVG(icon.name, icon.pack);
      await navigator.clipboard.writeText(svg);
      setCopied(true);
      toast.success('SVG copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      toast.error('Failed to copy SVG');
      console.error(error);
    } finally {
      setLoading(false);
    }
  };
  
  const handleDownload = async () => {
    try {
      setLoading(true);
      const svg = await loadIconSVG(icon.name, icon.pack);
      const blob = new Blob([svg], { type: 'image/svg+xml' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${icon.name}.svg`;
      a.click();
      URL.revokeObjectURL(url);
      toast.success('SVG downloaded');
    } catch (error) {
      toast.error('Failed to download SVG');
      console.error(error);
    } finally {
      setLoading(false);
    }
  };
  
  return (
    <div className="group relative p-4 border rounded-lg hover:shadow-lg transition-all bg-card">
      <div className="w-full aspect-square flex items-center justify-center mb-2">
        <img
          src={`/api/icons/${icon.pack}/${icon.name}`}
          alt={icon.name}
          className="w-16 h-16 object-contain"
          loading="lazy"
        />
      </div>
      
      <p className="text-sm text-center truncate font-medium">{icon.name}</p>
      <p className="text-xs text-center text-muted-foreground">{icon.pack}</p>
      
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
        <Button
          size="icon"
          variant="secondary"
          className="h-8 w-8"
          onClick={handleCopy}
          disabled={loading}
        >
          {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
        </Button>
        <Button
          size="icon"
          variant="secondary"
          className="h-8 w-8"
          onClick={handleDownload}
          disabled={loading}
        >
          <Download className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
