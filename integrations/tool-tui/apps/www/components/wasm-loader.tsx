'use client';

// WASM search feature commented out - using PGlite + client-side filtering instead
// import { useEffect, useState } from 'react';
// import { initWasmSearch, loadIconsIntoWasm, getTotalIcons } from '@/lib/wasm-icon-search';
// import { loadAllIconData } from '@/lib/icon-data-loader';

export function WasmLoader() {
  // WASM loading disabled
  return null;
  
  /* COMMENTED OUT - WASM SEARCH
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [progress, setProgress] = useState('');
  
  useEffect(() => {
    let mounted = true;
    
    async function init() {
      try {
        setProgress('Initializing WASM...');
        await initWasmSearch();
        
        setProgress('Loading icon data...');
        const icons = await loadAllIconData();
        
        setProgress(`Loading ${icons.length} icons into search engine...`);
        await loadIconsIntoWasm(icons);
        
        if (mounted) {
          const total = getTotalIcons();
          setProgress(`Ready! ${total.toLocaleString()} icons loaded`);
          setStatus('ready');
        }
      } catch (error) {
        console.error('WASM initialization failed:', error);
        if (mounted) {
          setStatus('error');
          setProgress('Failed to initialize search');
        }
      }
    }
    
    init();
    
    return () => {
      mounted = false;
    };
  }, []);
  
  if (status === 'ready') return null;
  
  return (
    <div className="fixed bottom-4 right-4 bg-black/80 text-white px-4 py-2 rounded-lg text-sm">
      {status === 'loading' && (
        <div className="flex items-center gap-2">
          <div className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
          <span>{progress}</span>
        </div>
      )}
      {status === 'error' && (
        <div className="text-red-400">{progress}</div>
      )}
    </div>
  );
  */
}
