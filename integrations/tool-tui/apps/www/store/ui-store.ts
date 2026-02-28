import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface UIState {
  searchTerm: string;
  sorted: boolean;
  showAll: boolean;
  viewMode: 'grid' | 'list';
  setSearchTerm: (term: string) => void;
  setSorted: (sorted: boolean) => void;
  setShowAll: (showAll: boolean) => void;
  setViewMode: (mode: 'grid' | 'list') => void;
  clearSearch: () => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      searchTerm: '',
      sorted: false,
      showAll: false,
      viewMode: 'grid',
      setSearchTerm: (term) => set({ searchTerm: term, showAll: false }),
      setSorted: (sorted) => set({ sorted, showAll: false }),
      setShowAll: (showAll) => set({ showAll }),
      setViewMode: (mode) => set({ viewMode: mode }),
      clearSearch: () => set({ searchTerm: '', showAll: false }),
    }),
    {
      name: 'dx-ui-storage',
      partialize: (state) => ({ sorted: state.sorted, viewMode: state.viewMode }),
    }
  )
);
