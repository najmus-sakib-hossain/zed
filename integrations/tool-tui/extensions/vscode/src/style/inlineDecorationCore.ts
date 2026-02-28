/**
 * Inline Decoration Core - Pure Functions
 * 
 * This module contains pure functions for inline decoration logic
 * that do not depend on VS Code APIs. This allows for testing outside of VS Code.
 * 
 * **Validates: Requirements 5.1, 5.4, 5.6**
 */

/**
 * Group Registry - stores mapping from grouped classnames to atomic classnames
 */
export interface GroupRegistry {
    /** Map from grouped classname (e.g., 'dxg-a1b2c') to atomic classnames */
    groups: Map<string, string[]>;
    /** Reverse map for lookup */
    atomicToGroup: Map<string, string>;
}

/**
 * Decoration style configuration
 */
export interface DecorationStyle {
    /** Text color for expanded classnames */
    color: string;
    /** Font size relative to editor */
    fontSize: string;
    /** Opacity */
    opacity: string;
    /** Font style */
    fontStyle: string;
}

/**
 * Default decoration style for expanded classnames
 */
export const DEFAULT_DECORATION_STYLE: DecorationStyle = {
    color: '#888888',
    fontSize: '0.9em',
    opacity: '0.8',
    fontStyle: 'italic'
};

/**
 * Pattern to match grouped classnames (dxg-* prefix)
 * **Validates: Requirements 5.4**
 */
export const GROUPED_CLASSNAME_PATTERN = /\bdxg-[a-zA-Z0-9]+\b/g;

/**
 * Check if a classname is a grouped classname
 * **Validates: Requirements 5.4**
 */
export function isGroupedClass(classname: string): boolean {
    return classname.startsWith('dxg-');
}

/**
 * Expand a grouped classname using a registry
 * **Validates: Requirements 5.1, 5.4**
 */
export function expandGroupedClass(groupedClass: string, groups: Map<string, string[]>): string[] {
    return groups.get(groupedClass) || [];
}

/**
 * Create an empty group registry
 */
export function createGroupRegistry(): GroupRegistry {
    return {
        groups: new Map(),
        atomicToGroup: new Map()
    };
}

/**
 * Load groups into a registry
 */
export function loadGroupsIntoRegistry(registry: GroupRegistry, groups: Map<string, string[]>): void {
    registry.groups = groups;

    // Build reverse map
    registry.atomicToGroup.clear();
    for (const [groupedClass, atomicClasses] of groups) {
        for (const atomicClass of atomicClasses) {
            registry.atomicToGroup.set(atomicClass, groupedClass);
        }
    }
}

/**
 * Add a group to the registry
 */
export function addGroupToRegistry(registry: GroupRegistry, groupedClass: string, atomicClasses: string[]): void {
    registry.groups.set(groupedClass, atomicClasses);
    for (const atomicClass of atomicClasses) {
        registry.atomicToGroup.set(atomicClass, groupedClass);
    }
}

/**
 * Parse group definitions from dx-style config or output
 */
export function parseGroupDefinitions(content: string): Map<string, string[]> {
    const groups = new Map<string, string[]>();

    // Pattern to match group definitions in various formats
    // Format 1: dxg-abc123: flex items-center justify-between
    // Format 2: "dxg-abc123": ["flex", "items-center", "justify-between"]

    // Try JSON format first
    try {
        const data = JSON.parse(content);
        if (data.groups) {
            for (const [key, value] of Object.entries(data.groups)) {
                if (Array.isArray(value)) {
                    groups.set(key, value as string[]);
                }
            }
            return groups;
        }
    } catch {
        // Not JSON, try other formats
    }

    // Try simple key: value format
    const linePattern = /^(dxg-[a-zA-Z0-9]+):\s*(.+)$/gm;
    let match: RegExpExecArray | null;

    while ((match = linePattern.exec(content)) !== null) {
        const groupedClass = match[1];
        const atomicClasses = match[2].trim().split(/\s+/);
        groups.set(groupedClass, atomicClasses);
    }

    return groups;
}

/**
 * Find all grouped classnames in a line of text
 */
export function findGroupedClassnamesInLine(lineText: string): Array<{ classname: string; start: number; end: number }> {
    const results: Array<{ classname: string; start: number; end: number }> = [];

    // Reset the regex
    GROUPED_CLASSNAME_PATTERN.lastIndex = 0;

    let match: RegExpExecArray | null;
    while ((match = GROUPED_CLASSNAME_PATTERN.exec(lineText)) !== null) {
        results.push({
            classname: match[0],
            start: match.index,
            end: match.index + match[0].length
        });
    }

    return results;
}

/**
 * Format expanded classnames for display
 */
export function formatExpandedClassnames(atomicClasses: string[]): string {
    if (atomicClasses.length === 0) {
        return '';
    }
    return ` → ${atomicClasses.join(' ')}`;
}
