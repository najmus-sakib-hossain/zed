/**
 * LLM Format Parser for DX Serializer VS Code Extension
 * 
 * Parses the token-efficient dx-serializer LLM format:
 * - Root scalars: name=value (underscores for spaces)
 * - Inline objects: section:count[key=value key2=value2]
 * - Nested arrays: key[count]=item1 item2 item3
 * - Tabular data: name:count(schema)[row1;row2]
 * 
 * Example LLM format:
 *   name=dx
 *   version=0.0.1
 *   workspace:1[paths[2]=@/www @/backend]
 *   editors:2[default=neovim items[7]=neovim zed vscode]
 *   dependencies:2(name version)[dx-package-1 0.0.1;dx-package-2 0.0.1]
 */

// ============================================================================
// Types and Interfaces
// ============================================================================

/**
 * Value types in DX LLM format
 */
export type DxValueType = 'string' | 'number' | 'bool' | 'null' | 'array' | 'object' | 'table' | 'ref';

/**
 * A value in the DX document
 */
export interface DxValue {
    type: DxValueType;
    value: string | number | boolean | null | DxValue[] | Map<string, DxValue>;
    refKey?: string;
}

/**
 * Field definition for tabular schemas
 */
export interface FieldDef {
    name: string;
    nested?: FieldDef[];
    isArray?: boolean;
}

/**
 * A data section with schema and rows
 */
export interface DxSection {
    id: string;
    schema: FieldDef[];
    rows: DxValue[][];
}

/**
 * The complete DX document representation
 */
export interface DxDocument {
    context: Map<string, DxValue>;
    objects: Map<string, Map<string, DxValue>>;
    sections: Map<string, DxSection>;
    arrays: Map<string, DxValue[]>;
    refs: Map<string, string>;
    sectionOrder?: string[];
}

/**
 * Parse error with location information
 */
export interface ParseError {
    message: string;
    line: number;
    column: number;
    hint?: string;
}

/**
 * Parse result
 */
export interface ParseResult {
    success: boolean;
    document?: DxDocument;
    error?: ParseError;
}

// ============================================================================
// Value Constructors
// ============================================================================

export function strValue(s: string): DxValue {
    return { type: 'string', value: s };
}

export function numValue(n: number): DxValue {
    return { type: 'number', value: n };
}

export function boolValue(b: boolean): DxValue {
    return { type: 'bool', value: b };
}

export function nullValue(): DxValue {
    return { type: 'null', value: null };
}

export function arrValue(items: DxValue[]): DxValue {
    return { type: 'array', value: items };
}

export function objValue(obj: Map<string, DxValue>): DxValue {
    return { type: 'object', value: obj };
}

export function refValue(key: string): DxValue {
    return { type: 'string', value: `^${key}` };
}

// ============================================================================
// Document Constructor
// ============================================================================

export function createDocument(): DxDocument {
    return {
        context: new Map(),
        objects: new Map(),
        sections: new Map(),
        arrays: new Map(),
        refs: new Map(),
        sectionOrder: [],
    };
}

export function createSection(id: string, schema: FieldDef[]): DxSection {
    return {
        id,
        schema,
        rows: [],
    };
}

/**
 * Helper to get field name from FieldDef
 */
export function getFieldName(field: FieldDef | string): string {
    if (typeof field === 'string') {
        return field;
    }
    return field.name;
}

/**
 * Helper to convert string array to FieldDef array
 */
export function toFieldDefs(schema: (string | FieldDef)[]): FieldDef[] {
    return schema.map(f => typeof f === 'string' ? { name: f } : f);
}

// ============================================================================
// Main Parser
// ============================================================================

/**
 * Parse LLM format content into a DxDocument
 */
export function parseLlm(input: string): ParseResult {
    const doc = createDocument();
    const lines = input.split('\n');

    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;

        try {
            parseLine(trimmed, doc);
        } catch (error) {
            return {
                success: false,
                error: {
                    message: error instanceof Error ? error.message : String(error),
                    line: 0,
                    column: 0,
                },
            };
        }
    }

    return { success: true, document: doc };
}

/**
 * Parse a single line of LLM format
 */
function parseLine(line: string, doc: DxDocument): void {
    // Check for inline object: section:count[...]
    const inlineObjMatch = line.match(/^([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)?):(\d+)\[(.+)\]$/);
    if (inlineObjMatch) {
        const [, sectionName, , content] = inlineObjMatch;
        parseInlineObject(sectionName, content, doc);
        return;
    }

    // Check for tabular data: name:count(schema)[rows]
    const tabularMatch = line.match(/^([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)?):(\d+)\(([^)]+)\)\[(.+)\]$/);
    if (tabularMatch) {
        const [, sectionName, , schemaStr, rowsStr] = tabularMatch;
        parseTabularData(sectionName, schemaStr, rowsStr, doc);
        return;
    }

    // Check for simple key=value (root scalar)
    const kvMatch = line.match(/^([a-zA-Z_][a-zA-Z0-9_]*)=(.+)$/);
    if (kvMatch) {
        const [, key, value] = kvMatch;
        doc.context.set(key, parseScalarValue(value));
        return;
    }

    // Unknown format - skip silently
}

/**
 * Parse an inline object like: workspace:1[paths[2]=@/www @/backend]
 */
function parseInlineObject(sectionName: string, content: string, doc: DxDocument): void {
    const properties = parseObjectContent(content);
    
    const sectionId = sectionName;
    const schema: string[] = [];
    const row: DxValue[] = [];

    for (const [key, prop] of properties) {
        schema.push(key);
        if (prop.isArray) {
            const items = (prop.value as string[]).map(v => parseScalarValue(v));
            row.push(arrValue(items));
        } else {
            row.push(parseScalarValue(prop.value as string));
        }
    }

    const section = createSection(sectionId, toFieldDefs(schema));
    section.rows.push(row);
    doc.sections.set(sectionId, section);
    
    if (doc.sectionOrder && !doc.sectionOrder.includes(sectionId)) {
        doc.sectionOrder.push(sectionId);
    }
}

// Parsed property type
interface ParsedProperty {
    name: string;
    value: string | string[];
    isArray: boolean;
    count?: number;
}

/**
 * Parse the content inside [...] of an inline object
 * Auto-detects separator: comma (legacy) or space (new format)
 */
function parseObjectContent(content: string): Map<string, ParsedProperty> {
    const properties = new Map<string, ParsedProperty>();
    
    // Detect separator by scanning for comma at depth 0
    let separator = ' '; // default to space
    let depth = 0;
    let afterValue = false;
    for (let i = 0; i < content.length; i++) {
        const ch = content[i];
        if (ch === '[' || ch === '(') depth++;
        else if (ch === ']' || ch === ')') depth--;
        else if (ch === '=') afterValue = true;
        else if (ch === ',' && depth === 0 && afterValue) {
            separator = ',';
            break;
        }
    }
    
    let pos = 0;

    while (pos < content.length) {
        // Skip whitespace
        while (pos < content.length && (content[pos] === ' ' || content[pos] === '\t')) pos++;
        if (pos >= content.length) break;

        // Read key (may include array count like key[2])
        const keyStart = pos;
        while (pos < content.length && content[pos] !== '=' && content[pos] !== ' ' && content[pos] !== ',') {
            pos++;
        }
        const keyPart = content.substring(keyStart, pos);

        // Check if key has array count: key[count]
        const arrayMatch = keyPart.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\[(\d+)\]$/);
        let key: string;
        let isArray = false;
        let arrayCount = 0;

        if (arrayMatch) {
            key = arrayMatch[1];
            isArray = true;
            arrayCount = parseInt(arrayMatch[2], 10);
        } else {
            key = keyPart;
        }

        // Skip whitespace before =
        while (pos < content.length && content[pos] === ' ') pos++;

        // Expect =
        if (pos >= content.length || content[pos] !== '=') {
            break;
        }
        pos++; // skip =

        // Skip whitespace after =
        while (pos < content.length && content[pos] === ' ') pos++;

        // Read value(s)
        if (isArray) {
            const values: string[] = [];
            // Detect array item separator (comma or space)
            const arrayItemSep = separator; // Use same separator as object
            
            for (let i = 0; i < arrayCount && pos < content.length; i++) {
                const valueStart = pos;
                while (pos < content.length && content[pos] !== arrayItemSep && content[pos] !== ' ') {
                    pos++;
                }
                values.push(content.substring(valueStart, pos));
                // Skip separator
                if (pos < content.length && (content[pos] === arrayItemSep || content[pos] === ' ')) {
                    pos++;
                    while (pos < content.length && content[pos] === ' ') pos++;
                }
            }
            properties.set(key, { name: key, value: values, isArray: true, count: arrayCount });
        } else {
            const valueStart = pos;
            let depth = 0;
            while (pos < content.length) {
                const ch = content[pos];
                if (ch === '[' || ch === '(') depth++;
                else if (ch === ']' || ch === ')') depth--;
                else if (depth === 0 && ch === separator) break;
                else if (depth === 0 && separator === ' ') {
                    // For space separator, check if next part is a key
                    const remaining = content.substring(pos);
                    const nextKeyMatch = remaining.match(/^\s+([a-zA-Z_][a-zA-Z0-9_-]*(?:\[\d+\])?)=/);
                    if (nextKeyMatch) break;
                }
                pos++;
            }
            const value = content.substring(valueStart, pos).trim();
            properties.set(key, { name: key, value, isArray: false });
        }
        
        // Skip separator if present
        while (pos < content.length && (content[pos] === separator || content[pos] === ' ')) pos++;
    }

    return properties;
}

/**
 * Parse tabular data like: dependencies:2(name version)[dx-package-1 0.0.1;dx-package-2 0.0.1]
 * Supports multiple row separators: comma, semicolon, colon, newline
 */
function parseTabularData(sectionName: string, schemaStr: string, rowsStr: string, doc: DxDocument): void {
    // Detect schema separator: comma or space
    const schema = schemaStr.includes(',') 
        ? schemaStr.split(',').map(s => s.trim()).filter(s => s)
        : schemaStr.split(/\s+/).filter(s => s);
    
    const section = createSection(sectionName, toFieldDefs(schema));

    // Detect row separator by checking which appears first at depth 0
    let rowSeparator = ';'; // default
    let depth = 0;
    for (let i = 0; i < rowsStr.length; i++) {
        const ch = rowsStr[i];
        if (ch === '[' || ch === '(') depth++;
        else if (ch === ']' || ch === ')') depth--;
        else if (depth === 0) {
            if (ch === ',') { rowSeparator = ','; break; }
            if (ch === ';') { rowSeparator = ';'; break; }
            if (ch === ':') { rowSeparator = ':'; break; }
            if (ch === '\n') { rowSeparator = '\n'; break; }
        }
    }

    const rows = rowsStr.split(rowSeparator);
    for (const rowStr of rows) {
        const trimmedRow = rowStr.trim();
        if (!trimmedRow) continue;

        const values = trimmedRow.split(/\s+/);
        const row: DxValue[] = values.map(v => parseScalarValue(v));

        while (row.length < schema.length) {
            row.push(strValue(''));
        }

        section.rows.push(row);
    }

    doc.sections.set(sectionName, section);
    
    if (doc.sectionOrder && !doc.sectionOrder.includes(sectionName)) {
        doc.sectionOrder.push(sectionName);
    }
}

/**
 * Parse a scalar value (handles underscores as spaces for name-like patterns, booleans, numbers)
 */
function parseScalarValue(raw: string): DxValue {
    const trimmed = raw.trim();

    if (trimmed === 'true') return boolValue(true);
    if (trimmed === 'false') return boolValue(false);
    if (trimmed === 'null') return nullValue();

    const num = parseFloat(trimmed);
    if (!isNaN(num) && isFinite(num) && /^-?\d+(\.\d+)?$/.test(trimmed)) {
        return numValue(num);
    }

    // Smart underscore conversion: Only convert patterns that look like human names
    // Requirements:
    // 1. Contains underscore
    // 2. Each part is Title Case (first char uppercase, rest lowercase)
    // 3. Each part has at least 3 characters
    // 4. Each part is all letters (no numbers or special chars)
    // Examples:
    //   James_Smith -> James Smith (converted)
    //   HTTP_Request -> HTTP_Request (kept as-is)
    //   my_variable -> my_variable (kept as-is)
    if (trimmed.includes('_')) {
        const parts = trimmed.split('_');
        if (parts.length >= 2) {
            const looksLikeName = parts.every(part => {
                if (part.length < 3) return false;
                const firstChar = part[0];
                const restChars = part.slice(1);
                // First char uppercase, rest lowercase, all letters
                return /^[A-Z]$/.test(firstChar) && 
                       /^[a-z]+$/.test(restChars);
            });
            if (looksLikeName) {
                return strValue(trimmed.replace(/_/g, ' '));
            }
        }
    }

    return strValue(trimmed);
}

