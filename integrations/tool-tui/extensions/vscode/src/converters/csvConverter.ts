/**
 * CSV to DxDocument Converter
 * 
 * Converts CSV content to DxDocument structure.
 * First row is treated as headers (schema).
 * 
 * Requirements: 1.4
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
    toFieldDefs,
} from '../llmParser';
import { ConversionResult } from './jsonConverter';

// No abbreviations in new format - use keys as-is
const compressKey = (key: string) => key;
const compressSectionName = (name: string) => name;

// ============================================================================
// CSV Parser
// ============================================================================

/**
 * Parse a CSV value (handle quoted values)
 */
function parseCsvValue(value: string): string {
    const trimmed = value.trim();

    // Quoted value
    if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
        return trimmed.slice(1, -1).replace(/""/g, '"');
    }

    return trimmed;
}

/**
 * Parse a CSV line into cells (handles quoted values with commas)
 */
function parseCsvLine(line: string): string[] {
    const cells: string[] = [];
    let current = '';
    let inQuotes = false;

    for (let i = 0; i < line.length; i++) {
        const char = line[i];

        if (char === '"') {
            if (inQuotes && line[i + 1] === '"') {
                // Escaped quote
                current += '"';
                i++;
            } else {
                inQuotes = !inQuotes;
            }
        } else if (char === ',' && !inQuotes) {
            cells.push(parseCsvValue(current));
            current = '';
        } else {
            current += char;
        }
    }

    cells.push(parseCsvValue(current));
    return cells;
}

/**
 * Convert a cell value to DxValue
 */
function cellToDxValue(cell: string): DxValue {
    const trimmed = cell.trim();

    // Empty
    if (trimmed === '' || trimmed === '-') {
        return nullValue();
    }

    // Boolean
    if (trimmed.toLowerCase() === 'true') {
        return boolValue(true);
    }
    if (trimmed.toLowerCase() === 'false') {
        return boolValue(false);
    }

    // Number
    const num = parseFloat(trimmed);
    if (!isNaN(num) && /^-?\d+(\.\d+)?$/.test(trimmed)) {
        return numValue(num);
    }

    // String
    return strValue(trimmed);
}

// ============================================================================
// Main Converter
// ============================================================================

/**
 * Convert CSV content to DxDocument
 * 
 * - First row is treated as headers (schema)
 * - Creates a single section named "data"
 */
export function convertCsvToDocument(content: string, sectionName: string = 'data'): ConversionResult {
    try {
        const lines = content.split('\n').filter(l => l.trim());

        if (lines.length < 1) {
            return {
                success: false,
                error: 'CSV must have at least a header row',
            };
        }

        // Parse header row
        const headers = parseCsvLine(lines[0]);
        const schema = headers.map(h => compressKey(h.toLowerCase()));

        // Create section
        const sectionId = compressSectionName(sectionName.toLowerCase());
        const section = createSection(sectionId, toFieldDefs(schema));

        // Parse data rows
        for (let i = 1; i < lines.length; i++) {
            const cells = parseCsvLine(lines[i]);
            const row: DxValue[] = [];

            for (let j = 0; j < schema.length; j++) {
                const cell = cells[j] || '';
                row.push(cellToDxValue(cell));
            }

            section.rows.push(row);
        }

        const doc = createDocument();
        doc.sections.set(sectionId, section);

        return { success: true, document: doc };
    } catch (error) {
        return {
            success: false,
            error: `CSV parse error: ${error instanceof Error ? error.message : String(error)}`,
        };
    }
}
