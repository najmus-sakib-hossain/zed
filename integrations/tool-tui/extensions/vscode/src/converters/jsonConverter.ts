/**
 * JSON to DxDocument Converter
 * 
 * Converts JSON objects and arrays to DxDocument structure.
 * 
 * Requirements: 1.1
 */

import {
    DxDocument,
    DxSection,
    DxValue,
    createDocument,
    createSection,
    strValue,
    numValue,
    boolValue,
    nullValue,
    arrValue,
    toFieldDefs,
} from '../llmParser';

// No abbreviations in new format - use keys as-is
const compressKey = (key: string) => key;
const compressSectionName = (name: string) => name;

// ============================================================================
// Types
// ============================================================================

export interface ConversionResult {
    success: boolean;
    document?: DxDocument;
    error?: string;
}

// ============================================================================
// Value Conversion
// ============================================================================

/**
 * Convert a JSON value to DxValue
 */
export function jsonValueToDx(value: unknown): DxValue {
    if (value === null || value === undefined) {
        return nullValue();
    }

    if (typeof value === 'string') {
        return strValue(value);
    }

    if (typeof value === 'number') {
        return numValue(value);
    }

    if (typeof value === 'boolean') {
        return boolValue(value);
    }

    if (Array.isArray(value)) {
        const items = value.map(v => jsonValueToDx(v));
        return arrValue(items);
    }

    if (typeof value === 'object') {
        // Nested object - convert to string representation
        return strValue(JSON.stringify(value));
    }

    return strValue(String(value));
}

// ============================================================================
// Main Converter
// ============================================================================

/**
 * Convert JSON content to DxDocument
 * 
 * - Top-level object keys become context values
 * - Arrays of objects become sections
 * - Nested objects are flattened or stringified
 */
export function convertJsonToDocument(content: string): ConversionResult {
    try {
        const parsed = JSON.parse(content);
        const doc = createDocument();

        if (Array.isArray(parsed)) {
            // Root array - create a single section
            const section = convertArrayToSection('data', parsed);
            if (section) {
                doc.sections.set(section.id, section);
            }
        } else if (typeof parsed === 'object' && parsed !== null) {
            // Root object - process each key
            for (const [key, value] of Object.entries(parsed)) {
                const compressedKey = compressKey(key.toLowerCase());

                if (Array.isArray(value) && value.length > 0 && typeof value[0] === 'object') {
                    // Array of objects - create section
                    const section = convertArrayToSection(key, value);
                    if (section) {
                        doc.sections.set(section.id, section);
                    }
                } else {
                    // Scalar or simple array - add to context
                    doc.context.set(compressedKey, jsonValueToDx(value));
                }
            }
        }

        return { success: true, document: doc };
    } catch (error) {
        return {
            success: false,
            error: `JSON parse error: ${error instanceof Error ? error.message : String(error)}`,
        };
    }
}

/**
 * Convert an array of objects to a DxSection
 */
function convertArrayToSection(name: string, items: unknown[]): DxSection | null {
    if (items.length === 0) {
        return null;
    }

    // Get all unique keys from all objects
    const allKeys = new Set<string>();
    for (const item of items) {
        if (typeof item === 'object' && item !== null) {
            for (const key of Object.keys(item)) {
                allKeys.add(key);
            }
        }
    }

    if (allKeys.size === 0) {
        return null;
    }

    const schema = Array.from(allKeys).map(k => compressKey(k.toLowerCase()));
    const sectionId = compressSectionName(name.toLowerCase());
    const section = createSection(sectionId, toFieldDefs(schema));

    // Convert each item to a row
    for (const item of items) {
        if (typeof item === 'object' && item !== null) {
            const row: DxValue[] = [];
            for (const key of allKeys) {
                const value = (item as Record<string, unknown>)[key];
                row.push(jsonValueToDx(value));
            }
            section.rows.push(row);
        }
    }

    return section;
}
