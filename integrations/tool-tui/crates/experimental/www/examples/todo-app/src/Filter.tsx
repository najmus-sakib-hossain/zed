import { FilterType } from './App';

interface FilterProps {
    current: FilterType;
    onChange: (filter: FilterType) => void;
}

export function Filter({ current, onChange }: FilterProps) {
    const filters: FilterType[] = ['all', 'active', 'completed'];

    return (
        <ul class="filters" role="tablist">
            {filters.map(filter => (
                <li key={filter}>
                    <button
                        class={current === filter ? 'selected' : ''}
                        onClick={() => onChange(filter)}
                        role="tab"
                        aria-selected={current === filter}
                    >
                        {filter.charAt(0).toUpperCase() + filter.slice(1)}
                    </button>
                </li>
            ))}
        </ul>
    );
}
