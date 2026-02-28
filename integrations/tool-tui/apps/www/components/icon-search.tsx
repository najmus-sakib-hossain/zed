'use client';

import { useState, useCallback, useEffect } from 'react';
import { Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { useDebounce } from '@/hooks/use-debounce';

interface IconSearchProps {
  onSearch: (query: string) => void;
  placeholder?: string;
}

export function IconSearch({ onSearch, placeholder = 'Search icons...' }: IconSearchProps) {
  const [value, setValue] = useState('');
  const debouncedValue = useDebounce(value, 150);
  
  // Trigger search when debounced value changes
  useEffect(() => {
    onSearch(debouncedValue);
  }, [debouncedValue, onSearch]);
  
  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(e.target.value);
  }, []);
  
  return (
    <div className="relative w-full max-w-2xl">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
      <Input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder={placeholder}
        className="pl-10 h-12 text-lg"
      />
    </div>
  );
}
