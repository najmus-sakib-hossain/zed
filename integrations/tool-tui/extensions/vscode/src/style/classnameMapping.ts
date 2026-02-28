/**
 * Classname to CSS Property Mapping
 * 
 * This module maps dx-style classname patterns to their corresponding CSS properties.
 * Used by the hover provider to show MDN documentation for utility classes.
 * 
 * **Validates: Requirements 2.1**
 */

import { getMDNPropertyInfo, MDNPropertyInfo } from './mdnData';

/**
 * Pattern mapping entry.
 * Patterns can be exact matches or use wildcards (*).
 */
interface PatternMapping {
    /** The classname pattern (e.g., "flex", "p-*", "bg-*") */
    pattern: string;
    /** The CSS property this pattern maps to */
    property: string;
}

/**
 * Classname pattern to CSS property mappings.
 * 
 * Patterns are checked in order, so more specific patterns should come first.
 * Use "*" as a wildcard to match any suffix.
 */
const patternMappings: PatternMapping[] = [
    // Display
    { pattern: 'flex', property: 'display' },
    { pattern: 'inline-flex', property: 'display' },
    { pattern: 'grid', property: 'display' },
    { pattern: 'inline-grid', property: 'display' },
    { pattern: 'block', property: 'display' },
    { pattern: 'inline-block', property: 'display' },
    { pattern: 'inline', property: 'display' },
    { pattern: 'hidden', property: 'display' },
    { pattern: 'contents', property: 'display' },

    // Flex direction
    { pattern: 'flex-row', property: 'flex-direction' },
    { pattern: 'flex-row-reverse', property: 'flex-direction' },
    { pattern: 'flex-col', property: 'flex-direction' },
    { pattern: 'flex-col-reverse', property: 'flex-direction' },

    // Justify content
    { pattern: 'justify-start', property: 'justify-content' },
    { pattern: 'justify-end', property: 'justify-content' },
    { pattern: 'justify-center', property: 'justify-content' },
    { pattern: 'justify-between', property: 'justify-content' },
    { pattern: 'justify-around', property: 'justify-content' },
    { pattern: 'justify-evenly', property: 'justify-content' },

    // Align items
    { pattern: 'items-start', property: 'align-items' },
    { pattern: 'items-end', property: 'align-items' },
    { pattern: 'items-center', property: 'align-items' },
    { pattern: 'items-baseline', property: 'align-items' },
    { pattern: 'items-stretch', property: 'align-items' },

    // Grid
    { pattern: 'grid-cols-*', property: 'grid-cols' },
    { pattern: 'grid-rows-*', property: 'grid-template-rows' },
    { pattern: 'col-span-*', property: 'grid-column' },
    { pattern: 'row-span-*', property: 'grid-row' },

    // Gap
    { pattern: 'gap-*', property: 'gap' },
    { pattern: 'gap-x-*', property: 'column-gap' },
    { pattern: 'gap-y-*', property: 'row-gap' },

    // Padding
    { pattern: 'p-*', property: 'padding' },
    { pattern: 'px-*', property: 'padding' },
    { pattern: 'py-*', property: 'padding' },
    { pattern: 'pt-*', property: 'padding-top' },
    { pattern: 'pr-*', property: 'padding-right' },
    { pattern: 'pb-*', property: 'padding-bottom' },
    { pattern: 'pl-*', property: 'padding-left' },

    // Margin
    { pattern: 'm-*', property: 'margin' },
    { pattern: 'mx-*', property: 'margin' },
    { pattern: 'my-*', property: 'margin' },
    { pattern: 'mt-*', property: 'margin-top' },
    { pattern: 'mr-*', property: 'margin-right' },
    { pattern: 'mb-*', property: 'margin-bottom' },
    { pattern: 'ml-*', property: 'margin-left' },
    { pattern: '-m-*', property: 'margin' },
    { pattern: '-mx-*', property: 'margin' },
    { pattern: '-my-*', property: 'margin' },
    { pattern: '-mt-*', property: 'margin-top' },
    { pattern: '-mr-*', property: 'margin-right' },
    { pattern: '-mb-*', property: 'margin-bottom' },
    { pattern: '-ml-*', property: 'margin-left' },

    // Width
    { pattern: 'w-*', property: 'width' },
    { pattern: 'min-w-*', property: 'min-width' },
    { pattern: 'max-w-*', property: 'max-width' },

    // Height
    { pattern: 'h-*', property: 'height' },
    { pattern: 'min-h-*', property: 'min-height' },
    { pattern: 'max-h-*', property: 'max-height' },

    // Typography
    { pattern: 'text-xs', property: 'font-size' },
    { pattern: 'text-sm', property: 'font-size' },
    { pattern: 'text-base', property: 'font-size' },
    { pattern: 'text-lg', property: 'font-size' },
    { pattern: 'text-xl', property: 'font-size' },
    { pattern: 'text-2xl', property: 'font-size' },
    { pattern: 'text-3xl', property: 'font-size' },
    { pattern: 'text-4xl', property: 'font-size' },
    { pattern: 'text-5xl', property: 'font-size' },
    { pattern: 'text-6xl', property: 'font-size' },
    { pattern: 'font-thin', property: 'font-weight' },
    { pattern: 'font-extralight', property: 'font-weight' },
    { pattern: 'font-light', property: 'font-weight' },
    { pattern: 'font-normal', property: 'font-weight' },
    { pattern: 'font-medium', property: 'font-weight' },
    { pattern: 'font-semibold', property: 'font-weight' },
    { pattern: 'font-bold', property: 'font-weight' },
    { pattern: 'font-extrabold', property: 'font-weight' },
    { pattern: 'font-black', property: 'font-weight' },
    { pattern: 'text-left', property: 'text-align' },
    { pattern: 'text-center', property: 'text-align' },
    { pattern: 'text-right', property: 'text-align' },
    { pattern: 'text-justify', property: 'text-align' },
    { pattern: 'leading-*', property: 'line-height' },

    // Colors (text)
    { pattern: 'text-*', property: 'color' },

    // Background colors
    { pattern: 'bg-*', property: 'background-color' },

    // Border
    { pattern: 'border', property: 'border' },
    { pattern: 'border-*', property: 'border' },
    { pattern: 'rounded', property: 'border-radius' },
    { pattern: 'rounded-*', property: 'border-radius' },

    // Effects
    { pattern: 'shadow', property: 'box-shadow' },
    { pattern: 'shadow-*', property: 'box-shadow' },
    { pattern: 'opacity-*', property: 'opacity' },

    // Transforms
    { pattern: 'transform', property: 'transform' },
    { pattern: 'rotate-*', property: 'transform' },
    { pattern: 'scale-*', property: 'transform' },
    { pattern: 'translate-*', property: 'transform' },
    { pattern: 'skew-*', property: 'transform' },

    // Transitions
    { pattern: 'transition', property: 'transition' },
    { pattern: 'transition-*', property: 'transition' },
    { pattern: 'duration-*', property: 'transition-duration' },
    { pattern: 'ease-*', property: 'transition-timing-function' },
    { pattern: 'delay-*', property: 'transition-delay' },

    // Position
    { pattern: 'static', property: 'position' },
    { pattern: 'fixed', property: 'position' },
    { pattern: 'absolute', property: 'position' },
    { pattern: 'relative', property: 'position' },
    { pattern: 'sticky', property: 'position' },
    { pattern: 'top-*', property: 'top' },
    { pattern: 'right-*', property: 'right' },
    { pattern: 'bottom-*', property: 'bottom' },
    { pattern: 'left-*', property: 'left' },
    { pattern: 'inset-*', property: 'inset' },
    { pattern: 'z-*', property: 'z-index' },

    // Overflow
    { pattern: 'overflow-auto', property: 'overflow' },
    { pattern: 'overflow-hidden', property: 'overflow' },
    { pattern: 'overflow-visible', property: 'overflow' },
    { pattern: 'overflow-scroll', property: 'overflow' },
    { pattern: 'overflow-x-*', property: 'overflow-x' },
    { pattern: 'overflow-y-*', property: 'overflow-y' },
];

/**
 * Check if a classname matches a pattern.
 * 
 * @param classname - The classname to check
 * @param pattern - The pattern to match against (supports * wildcard)
 * @returns True if the classname matches the pattern
 */
function matchesPattern(classname: string, pattern: string): boolean {
    if (pattern.endsWith('*')) {
        const prefix = pattern.slice(0, -1);
        return classname.startsWith(prefix);
    }
    return classname === pattern;
}

/**
 * Get the CSS property for a dx-style classname.
 * 
 * @param classname - The dx-style classname (e.g., "flex", "p-4", "bg-blue-500")
 * @returns The CSS property name, or undefined if not found
 */
export function getPropertyForClassname(classname: string): string | undefined {
    // Remove any variant prefixes (hover:, focus:, etc.)
    const baseClassname = classname.includes(':')
        ? classname.split(':').pop()!
        : classname;

    for (const mapping of patternMappings) {
        if (matchesPattern(baseClassname, mapping.pattern)) {
            return mapping.property;
        }
    }

    return undefined;
}

/**
 * Get MDN property info for a dx-style classname.
 * 
 * @param classname - The dx-style classname
 * @returns The MDN property info, or undefined if not found
 */
export function getMDNInfoForClassname(classname: string): MDNPropertyInfo | undefined {
    const property = getPropertyForClassname(classname);
    if (!property) {
        return undefined;
    }
    return getMDNPropertyInfo(property);
}
