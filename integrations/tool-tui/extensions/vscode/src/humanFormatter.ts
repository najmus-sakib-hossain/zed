/**
 * Human Format Formatter for DX Serializer VS Code Extension
 * 
 * Converts DxDocument to clean TOML/INI-like human-readable format:
 * - Root scalars first (key = value, padded to column 28)
 * - [section] headers for grouped data
 * - [parent.child] for nested sections
 * - Arrays as key: followed by - item lines
 * - Scalars before arrays within sections
 * - Blank line between sections
 * 
 * Based on HUMAN_FORMAT.md specification.
 */

import { DxDocument, DxValue, DxSection, getFieldName } from './llmParser';

// ============================================================================
// Configuration
// ============================================================================

const KEY_PADDING = 28; // Align = to column 28

// ============================================================================
// Value Formatting
// ============================================================================

/**
 * Format a DxValue to string for display
 */
function formatValue(value: DxValue): string {
    switch (value.type) {
        case 'string':
            return String(value.value);
        case 'number':
            return String(value.value);
        case 'bool':
            return value.value ? 'true' : 'false';
        case 'null':
            return 'none';
        case 'array':
            return '';
        default:
            return String(value.value);
    }
}

/**
 * Check if a value is an array
 */
function isArrayValue(value: DxValue): boolean {
    return value.type === 'array';
}

/**
 * Get array items from a DxValue
 */
function getArrayItems(value: DxValue): string[] {
    if (value.type !== 'array') return [];
    const items = value.value as DxValue[];
    return items.map(item => formatValue(item));
}

/**
 * Format a key with padding to align = at column 28
 */
function formatKeyValue(key: string, value: string): string {
    const padding = Math.max(1, KEY_PADDING - key.length);
    return `${key}${' '.repeat(padding)}= ${value}`;
}

// ============================================================================
// Section Formatting
// ============================================================================

/**
 * Format root scalars (context values that are not arrays)
 */
function formatRootScalars(doc: DxDocument): string[] {
    const lines: string[] = [];

    for (const [key, value] of doc.context) {
        if (!isArrayValue(value)) {
            lines.push(formatKeyValue(key, formatValue(value)));
        }
    }

    return lines;
}

/**
 * Format a section with its properties
 */
function formatSection(sectionId: string, section: DxSection): string[] {
    const lines: string[] = [];

    lines.push(`[${sectionId}]`);

    if (section.rows.length === 0) {
        return lines;
    }

    const schema = section.schema;
    const row = section.rows[0];

    // Separate scalars and arrays
    const scalars: Array<{ key: string; value: string }> = [];
    const arrays: Array<{ key: string; items: string[] }> = [];

    for (let i = 0; i < schema.length && i < row.length; i++) {
        const key = getFieldName(schema[i]);
        const value = row[i];

        if (isArrayValue(value)) {
            arrays.push({ key, items: getArrayItems(value) });
        } else {
            scalars.push({ key, value: formatValue(value) });
        }
    }

    // Output scalars first
    for (const { key, value } of scalars) {
        lines.push(formatKeyValue(key, value));
    }

    // Output arrays (key: followed by - item lines)
    for (const { key, items } of arrays) {
        lines.push(`${key}:`);
        for (const item of items) {
            lines.push(`- ${item}`);
        }
    }

    return lines;
}

/**
 * Format tabular section (like dependencies with name/version columns)
 */
function formatTabularSection(sectionId: string, section: DxSection): string[] {
    const lines: string[] = [];

    lines.push(`[${sectionId}]`);

    for (const row of section.rows) {
        if (row.length >= 2) {
            const key = formatValue(row[0]);
            const value = formatValue(row[1]);
            lines.push(formatKeyValue(key, value));
        }
    }

    return lines;
}

// ============================================================================
// Document Formatting
// ============================================================================

/**
 * Check if a section is tabular (has multiple rows with name/version schema)
 */
function isTabularSection(section: DxSection): boolean {
    if (section.rows.length <= 1) return false;
    const schemaNames = section.schema.map(f => getFieldName(f).toLowerCase());
    return schemaNames.includes('name') && schemaNames.includes('version');
}

/**
 * Format a complete DxDocument to Human Format
 */
export function formatDocument(doc: DxDocument): string {
    const sections: string[] = [];

    // 1. Root scalars first
    const rootScalars = formatRootScalars(doc);
    if (rootScalars.length > 0) {
        sections.push(rootScalars.join('\n'));
    }

    // 2. Sections in order
    const sectionOrder = doc.sectionOrder || Array.from(doc.sections.keys());

    for (const sectionId of sectionOrder) {
        const section = doc.sections.get(sectionId);
        if (!section) continue;

        let sectionLines: string[];

        if (isTabularSection(section)) {
            sectionLines = formatTabularSection(sectionId, section);
        } else {
            sectionLines = formatSection(sectionId, section);
        }

        if (sectionLines.length > 0) {
            sections.push(sectionLines.join('\n'));
        }
    }

    return sections.join('\n\n');
}

