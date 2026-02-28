/**
 * Human Format Parser for DX Serializer VS Code Extension
 * 
 * Parses the TOML/INI-like human format back to DxDocument:
 * - Root scalars (key = value)
 * - [section] headers
 * - [parent.child] nested sections
 * - Arrays as key: followed by - item lines
 * 
 * Also provides serialization back to LLM format.
 */

import {
    DxDocument,
    DxValue,
    createDocument,
    createSection,
    strValue,
    numValue,
    boolValue,
    nullValue,
    arrValue,
    toFieldDefs,
    getFieldName,
} from './llmParser';

// ============================================================================
// Types
// ============================================================================

export interface ParseErrorInfo {
    message: string;
    line: number;
    column: number;
    snippet?: string;
    hint?: string;
}

export interface HumanParseResult {
    success: boolean;
    document?: DxDocument;
    error?: ParseErrorInfo;
}

// Extended document with section order tracking
export interface DxDocumentWithOrder extends DxDocument {
    sectionOrder: string[];
}

// ============================================================================
// Value Parsing
// ============================================================================

function parseValue(raw: string): DxValue {
    const trimmed = raw.trim();
    
    if (trimmed === '' || trimmed === 'none') return nullValue();
    if (trimmed.toLowerCase() === 'true') return boolValue(true);
    if (trimmed.toLowerCase() === 'false') return boolValue(false);
    
    // Handle quoted strings
    if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
        (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
        return strValue(trimmed.slice(1, -1));
    }
    
    // Try number
    const num = parseFloat(trimmed);
    if (!isNaN(num) && isFinite(num) && /^-?\d+(\.\d+)?$/.test(trimmed)) {
        return numValue(num);
    }
    
    return strValue(trimmed);
}

// ============================================================================
// Line Parsing
// ============================================================================

function parseSectionHeader(line: string): string | null {
    const trimmed = line.trim();
    const match = trimmed.match(/^\[([a-zA-Z0-9_.]+)\]$/);
    if (!match) return null;
    return match[1].toLowerCase();
}

function parseKeyValueLine(line: string): [string, string] | null {
    const trimmed = line.trim();
    
    if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('//')) return null;
    if (trimmed.startsWith('[')) return null;
    if (trimmed.startsWith('-')) return null;
    
    const eqIndex = trimmed.indexOf('=');
    if (eqIndex === -1) {
        // Check for array header (key:)
        if (trimmed.endsWith(':')) {
            return [trimmed.slice(0, -1).trim(), ''];
        }
        return null;
    }
    
    const key = trimmed.substring(0, eqIndex).trim();
    const value = trimmed.substring(eqIndex + 1).trim();
    
    if (!key) return null;
    
    return [key, value];
}

function parseArrayItem(line: string): string | null {
    const trimmed = line.trim();
    if (!trimmed.startsWith('-')) return null;
    return trimmed.substring(1).trim();
}

// ============================================================================
// Main Parser
// ============================================================================

export function parseHuman(input: string): HumanParseResult {
    const doc = createDocument() as DxDocumentWithOrder;
    doc.sectionOrder = [];
    
    const lines = input.split('\n');
    let currentSection: string | null = null;
    let currentArrayKey: string | null = null;
    let currentArrayItems: string[] = [];
    let lineNum = 0;

    // Track section data
    const sectionData: Map<string, Map<string, DxValue | string[]>> = new Map();

    function flushArray() {
        if (currentArrayKey && currentArrayItems.length > 0) {
            if (currentSection) {
                if (!sectionData.has(currentSection)) {
                    sectionData.set(currentSection, new Map());
                }
                sectionData.get(currentSection)!.set(currentArrayKey, currentArrayItems);
            }
            currentArrayKey = null;
            currentArrayItems = [];
        }
    }

    for (const line of lines) {
        lineNum++;
        const trimmed = line.trim();
        
        if (!trimmed) continue;
        if (trimmed.startsWith('//') || trimmed.startsWith('#')) continue;

        // Check for array item
        const arrayItem = parseArrayItem(trimmed);
        if (arrayItem !== null) {
            if (currentArrayKey) {
                currentArrayItems.push(arrayItem);
            }
            continue;
        }

        // Flush any pending array
        flushArray();

        // Check for section header
        const sectionName = parseSectionHeader(trimmed);
        if (sectionName !== null) {
            currentSection = sectionName;
            if (!doc.sectionOrder.includes(sectionName)) {
                doc.sectionOrder.push(sectionName);
            }
            if (!sectionData.has(sectionName)) {
                sectionData.set(sectionName, new Map());
            }
            continue;
        }

        // Check for key-value or array header
        const kv = parseKeyValueLine(trimmed);
        if (kv) {
            const [key, value] = kv;
            
            // Array header (key:)
            if (value === '' && trimmed.endsWith(':')) {
                currentArrayKey = key;
                currentArrayItems = [];
                continue;
            }

            // Regular key-value
            if (currentSection === null) {
                // Root scalar
                doc.context.set(key, parseValue(value));
            } else {
                // Section scalar
                if (!sectionData.has(currentSection)) {
                    sectionData.set(currentSection, new Map());
                }
                sectionData.get(currentSection)!.set(key, parseValue(value));
            }
        }
    }

    // Flush final array
    flushArray();

    // Convert section data to DxSections
    for (const sectionName of doc.sectionOrder) {
        const data = sectionData.get(sectionName);
        if (!data || data.size === 0) continue;

        const schema: string[] = [];
        const row: DxValue[] = [];

        for (const [key, value] of data) {
            schema.push(key);
            if (Array.isArray(value)) {
                const items = value.map(v => parseValue(v));
                row.push(arrValue(items));
            } else {
                row.push(value as DxValue);
            }
        }

        const section = createSection(sectionName, toFieldDefs(schema));
        section.rows.push(row);
        doc.sections.set(sectionName, section);
    }

    return { success: true, document: doc };
}

// ============================================================================
// LLM Serializer
// ============================================================================

function serializeValue(value: DxValue): string {
    switch (value.type) {
        case 'string':
            // Replace spaces with underscores for LLM format
            return String(value.value).replace(/ /g, '_');
        case 'number':
            return String(value.value);
        case 'bool':
            return value.value ? 'true' : 'false';
        case 'null':
            return 'none';
        case 'array':
            const items = value.value as DxValue[];
            return items.map(v => serializeValue(v)).join(' ');
        default:
            return String(value.value);
    }
}

/**
 * Serialize DxDocument to LLM format
 */
export function serializeToLlm(doc: DxDocument): string {
    const lines: string[] = [];

    // Root scalars
    for (const [key, value] of doc.context) {
        lines.push(`${key}=${serializeValue(value)}`);
    }

    // Sections
    const sectionOrder = (doc as DxDocumentWithOrder).sectionOrder || Array.from(doc.sections.keys());

    for (const sectionId of sectionOrder) {
        const section = doc.sections.get(sectionId);
        if (!section || section.rows.length === 0) continue;

        const schema = section.schema;
        const row = section.rows[0];

        // Check if tabular (multiple rows)
        if (section.rows.length > 1) {
            const schemaStr = schema.map(f => getFieldName(f)).join(' ');
            const rowsStr = section.rows.map(r => 
                r.map(v => serializeValue(v)).join(' ')
            ).join(';');
            lines.push(`${sectionId}:${section.rows.length}(${schemaStr})[${rowsStr}]`);
            continue;
        }

        // Single row - inline object format
        const props: string[] = [];
        let propCount = 0;

        for (let i = 0; i < schema.length && i < row.length; i++) {
            const key = getFieldName(schema[i]);
            const value = row[i];
            propCount++;

            if (value.type === 'array') {
                const items = value.value as DxValue[];
                const itemsStr = items.map(v => serializeValue(v)).join(' ');
                props.push(`${key}[${items.length}]=${itemsStr}`);
            } else {
                props.push(`${key}=${serializeValue(value)}`);
            }
        }

        lines.push(`${sectionId}:${propCount}[${props.join(' ')}]`);
    }

    return lines.join('\n');
}

