/**
 * TOML to DxDocument Converter
 * 
 * Converts TOML content to DxDocument structure.
 * Uses simple parsing without external dependencies.
 * 
 * Requirements: 1.3
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
// Simple TOML Parser
// ============================================================================

/**
 * Parse a TOML value
 */
function parseTomlValue(value: string): unknown {
    const trimmed = value.trim();

    // Empty
    if (trimmed === '') {
        return '';
    }

    // Boolean
    if (trimmed === 'true') {
        return true;
    }
    if (trimmed === 'false') {
        return false;
    }

    // Quoted string
    if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
        return trimmed.slice(1, -1);
    }

    // Array [a, b, c]
    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
        const inner = trimmed.slice(1, -1);
        if (!inner.trim()) {
            return [];
        }
        return inner.split(',').map(item => parseTomlValue(item.trim()));
    }

    // Number
    const num = parseFloat(trimmed);
    if (!isNaN(num) && /^-?\d+(\.\d+)?$/.test(trimmed)) {
        return num;
    }

    // Plain string (unquoted)
    return trimmed;
}

/**
 * Simple TOML parser
 */
export function parseSimpleToml(content: string): Record<string, unknown> {
    const lines = content.split('\n');
    const result: Record<string, unknown> = {};
    let currentSection: Record<string, unknown> = result;
    let currentSectionName = '';

    for (const line of lines) {
        const trimmed = line.trim();

        // Skip empty lines and comments
        if (!trimmed || trimmed.startsWith('#')) {
            continue;
        }

        // Section header [section]
        const sectionMatch = trimmed.match(/^\[([a-zA-Z0-9_.-]+)\]$/);
        if (sectionMatch) {
            currentSectionName = sectionMatch[1];
            // Handle nested sections like [section.subsection]
            const parts = currentSectionName.split('.');
            currentSection = result;
            for (const part of parts) {
                if (!currentSection[part]) {
                    currentSection[part] = {};
                }
                currentSection = currentSection[part] as Record<string, unknown>;
            }
            continue;
        }

        // Array of tables [[section]]
        const arrayTableMatch = trimmed.match(/^\[\[([a-zA-Z0-9_.-]+)\]\]$/);
        if (arrayTableMatch) {
            currentSectionName = arrayTableMatch[1];
            const parts = currentSectionName.split('.');
            let target = result;
            for (let i = 0; i < parts.length - 1; i++) {
                if (!target[parts[i]]) {
                    target[parts[i]] = {};
                }
                target = target[parts[i]] as Record<string, unknown>;
            }
            const lastPart = parts[parts.length - 1];
            if (!target[lastPart]) {
                target[lastPart] = [];
            }
            const newObj: Record<string, unknown> = {};
            (target[lastPart] as unknown[]).push(newObj);
            currentSection = newObj;
            continue;
        }

        // Key = value
        const eqIndex = trimmed.indexOf('=');
        if (eqIndex > 0) {
            const key = trimmed.slice(0, eqIndex).trim();
            const valueStr = trimmed.slice(eqIndex + 1).trim();
            currentSection[key] = parseTomlValue(valueStr);
        }
    }

    return result;
}

// ============================================================================
// Main Converter
// ============================================================================

/**
 * Convert TOML content to DxDocument
 */
export function convertTomlToDocument(content: string): ConversionResult {
    try {
        const parsed = parseSimpleToml(content);
        const doc = createDocument();

        for (const [key, value] of Object.entries(parsed)) {
            const compressedKey = compressKey(key.toLowerCase());

            if (Array.isArray(value) && value.length > 0 && typeof value[0] === 'object') {
                // Array of tables - create section
                const section = convertArrayToSection(key, value);
                if (section) {
                    doc.sections.set(section.id, section);
                }
            } else if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
                // Nested table - create section with single row
                const section = convertObjectToSection(key, value as Record<string, unknown>);
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
            error: `TOML parse error: ${error instanceof Error ? error.message : String(error)}`,
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

/**
 * Convert a single object to a DxSection with one row
 */
function convertObjectToSection(name: string, obj: Record<string, unknown>): DxSection | null {
    const keys = Object.keys(obj);
    if (keys.length === 0) {
        return null;
    }

    const schema = keys.map(k => compressKey(k.toLowerCase()));
    const sectionId = compressSectionName(name.toLowerCase());
    const section = createSection(sectionId, toFieldDefs(schema));

    const row: DxValue[] = keys.map(k => jsonValueToDx(obj[k]));
    section.rows.push(row);

    return section;
}
