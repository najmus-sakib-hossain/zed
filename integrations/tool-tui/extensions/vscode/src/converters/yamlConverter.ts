/**
 * YAML to DxDocument Converter
 * 
 * Converts YAML content to DxDocument structure.
 * Uses simple parsing without external dependencies.
 * 
 * Requirements: 1.2
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
import { ConversionResult, jsonValueToDx } from './jsonConverter';

// No abbreviations in new format - use keys as-is
const compressKey = (key: string) => key;
const compressSectionName = (name: string) => name;

// ============================================================================
// Simple YAML Parser
// ============================================================================

interface YamlParseState {
    currentIndent: number;
    result: Record<string, unknown>;
    stack: Array<{ indent: number; obj: Record<string, unknown>; key: string }>;
}

/**
 * Parse a simple YAML value
 */
function parseYamlValue(value: string): unknown {
    const trimmed = value.trim();

    // Empty or null
    if (trimmed === '' || trimmed === '~' || trimmed === 'null') {
        return null;
    }

    // Boolean
    if (trimmed === 'true' || trimmed === 'yes' || trimmed === 'on') {
        return true;
    }
    if (trimmed === 'false' || trimmed === 'no' || trimmed === 'off') {
        return false;
    }

    // Quoted string
    if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
        return trimmed.slice(1, -1);
    }

    // Number
    const num = parseFloat(trimmed);
    if (!isNaN(num) && /^-?\d+(\.\d+)?$/.test(trimmed)) {
        return num;
    }

    // Inline array [a, b, c]
    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
        const inner = trimmed.slice(1, -1);
        return inner.split(',').map(item => parseYamlValue(item.trim()));
    }

    // Plain string
    return trimmed;
}

/**
 * Get indentation level of a line
 */
function getIndent(line: string): number {
    const match = line.match(/^(\s*)/);
    return match ? match[1].length : 0;
}

/**
 * Simple YAML parser for flat and nested structures
 */
export function parseSimpleYaml(content: string): Record<string, unknown> {
    const lines = content.split('\n');
    const result: Record<string, unknown> = {};
    let currentObj = result;
    let currentKey = '';
    let pendingListKey = '';
    const stack: Array<{ indent: number; obj: Record<string, unknown>; key: string }> = [];
    let listItems: unknown[] = [];
    let inList = false;

    for (const line of lines) {
        // Skip empty lines and comments
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#') || trimmed === '---') {
            continue;
        }

        const indent = getIndent(line);

        // List item (- value or -value)
        if (/^-\s*/.test(trimmed)) {
            const value = parseYamlValue(trimmed.replace(/^-\s*/, ''));
            if (!inList) {
                inList = true;
                listItems = [];
            }
            listItems.push(value);
            continue;
        }

        // End of list - save it to the pending key
        if (inList && pendingListKey) {
            // Find the right object to assign to
            if (stack.length > 0) {
                const parent = stack[stack.length - 1];
                parent.obj[pendingListKey] = listItems;
            } else {
                result[pendingListKey] = listItems;
            }
            inList = false;
            listItems = [];
            pendingListKey = '';
        }

        // Key: value pattern
        const colonIndex = trimmed.indexOf(':');
        if (colonIndex > 0) {
            const key = trimmed.slice(0, colonIndex).trim();
            const valueStr = trimmed.slice(colonIndex + 1).trim();

            // Pop stack if we're at a lower indent
            while (stack.length > 0 && indent <= stack[stack.length - 1].indent) {
                stack.pop();
                currentObj = stack.length > 0 ? stack[stack.length - 1].obj : result;
            }

            if (valueStr) {
                // Key with value
                currentObj[key] = parseYamlValue(valueStr);
            } else {
                // Key without value - could be list or nested object
                // We'll set pendingListKey and wait to see what comes next
                pendingListKey = key;
                stack.push({ indent, obj: currentObj, key });
            }

            currentKey = key;
        }
    }

    // Handle trailing list
    if (inList && pendingListKey) {
        if (stack.length > 0) {
            const parent = stack[stack.length - 1];
            parent.obj[pendingListKey] = listItems;
        } else {
            result[pendingListKey] = listItems;
        }
    }

    return result;
}

// ============================================================================
// Main Converter
// ============================================================================

/**
 * Convert YAML content to DxDocument
 */
export function convertYamlToDocument(content: string): ConversionResult {
    try {
        const parsed = parseSimpleYaml(content);
        const doc = createDocument();

        for (const [key, value] of Object.entries(parsed)) {
            const compressedKey = compressKey(key.toLowerCase());

            if (Array.isArray(value) && value.length > 0 && typeof value[0] === 'object') {
                // Array of objects - create section
                const section = convertArrayToSection(key, value as unknown[]);
                if (section) {
                    doc.sections.set(section.id, section);
                }
            } else {
                // Scalar or simple array - add to context
                doc.context.set(compressedKey, jsonValueToDx(value));
            }
        }

        return { success: true, document: doc };
    } catch (error) {
        return {
            success: false,
            error: `YAML parse error: ${error instanceof Error ? error.message : String(error)}`,
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
