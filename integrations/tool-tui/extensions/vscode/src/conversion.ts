/**
 * LLM ↔ Human Format Conversion
 * 
 * LLM Format: Compact format optimized for token efficiency
 * Human Format: TOML-like with [sections], aligned = at column 28
 * 
 * LLM Format patterns:
 * 1. Root scalars: key=value key2=value2
 * 2. Inline objects: section:count[key=value key2[count]=item1 item2]
 * 3. Key-value pairs: section:count@=[key value key value]
 * 4. Simple arrays: section:count=item1 item2 item3
 * 5. Tabular data: section:count(schema)@suffixes[row1;row2]
 * 
 * Suffix patterns in tabular data:
 * - Suffixes are positional and apply to columns in order
 * - @value = prefix (prepend to column value)
 * - @@value = suffix (append to column value)
 */

interface SectionData {
    scalars: Map<string, string>;
    arrays: Map<string, string[]>;
    tabular?: { schema: string[]; suffix?: string; rows: string[][] };
}

const ALIGN_COL = 28;

/**
 * Convert underscore to space in value (for human display)
 * Preserves underscores in paths, URLs, and identifiers
 */
function humanizeValue(v: string): string {
    if (!v) return v;
    // Don't convert underscores in paths, URLs, or identifiers with special chars
    if (v.startsWith('@/') || v.startsWith('@') || v.startsWith('http') || v.includes('://')) {
        return v;
    }
    return v.replace(/_/g, ' ');
}

/**
 * Format a key-value line with aligned =
 */
function formatKV(key: string, value: string): string {
    const padding = ' '.repeat(Math.max(1, ALIGN_COL - key.length));
    return `${key}${padding}= ${value}`;
}

/**
 * Parse inline object content: key=value key2[count]=item1 item2
 */
function parseInlineContent(content: string): { scalars: Map<string, string>; arrays: Map<string, string[]> } {
    const scalars = new Map<string, string>();
    const arrays = new Map<string, string[]>();
    let pos = 0;
    
    while (pos < content.length) {
        // Skip whitespace
        while (pos < content.length && content[pos] === ' ') pos++;
        if (pos >= content.length) break;
        
        // Read key (may include array count like key[2])
        const keyStart = pos;
        while (pos < content.length && content[pos] !== '=' && content[pos] !== ' ') {
            pos++;
        }
        const keyPart = content.substring(keyStart, pos);
        if (!keyPart) break;
        
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
        
        // Expect =
        if (pos >= content.length || content[pos] !== '=') {
            break;
        }
        pos++; // skip =
        
        // Read value(s)
        if (isArray) {
            const values: string[] = [];
            for (let i = 0; i < arrayCount && pos < content.length; i++) {
                const valueStart = pos;
                while (pos < content.length && content[pos] !== ' ') {
                    pos++;
                }
                values.push(content.substring(valueStart, pos));
                if (pos < content.length && content[pos] === ' ') pos++;
            }
            arrays.set(key, values);
        } else {
            const valueStart = pos;
            // Read until next key=value pattern or end
            while (pos < content.length) {
                const remaining = content.substring(pos);
                const nextKeyMatch = remaining.match(/^\s+([a-zA-Z_][a-zA-Z0-9_-]*(?:\[\d+\])?)=/);
                if (nextKeyMatch) {
                    break;
                }
                pos++;
            }
            const value = content.substring(valueStart, pos).trim();
            scalars.set(key, value);
        }
    }
    
    return { scalars, arrays };
}

/**
 * Parse key-value pairs format: key value key value ...
 */
function parseKVPairs(content: string): Map<string, string> {
    const result = new Map<string, string>();
    const parts = content.trim().split(/\s+/);
    for (let i = 0; i < parts.length - 1; i += 2) {
        result.set(parts[i], parts[i + 1] || '');
    }
    return result;
}

/**
 * Parse multiple key=value on same line
 */
function parseMultipleScalars(line: string): Map<string, string> {
    const result = new Map<string, string>();
    const regex = /([a-zA-Z_][a-zA-Z0-9_.-]*)=([^\s]+)/g;
    let match;
    while ((match = regex.exec(line)) !== null) {
        result.set(match[1], match[2]);
    }
    return result;
}

/**
 * Parse suffix patterns for tabular data
 * 
 * Suffix patterns indicate how to expand compressed values back to full form.
 * 
 * Examples:
 * - "@ORD- @2025-01- @@ex.com" for (id customer email items total status date)
 *   - @ORD- → prefix for id column
 *   - @2025-01- → prefix for date column
 *   - @@ex.com → suffix for email column
 * 
 * - "@_Trail @4." for (id name dist elev companion sunny rating)
 *   - @_Trail → suffix for name column (value + "_Trail" → "Blue_Lake_Trail")
 *   - @4. → prefix for rating column ("4." + value → "4.5")
 */
interface SuffixTransform {
    colIndex: number;
    type: 'prefix' | 'suffix';
    value: string;
}

function parseSuffixPatterns(suffixStr: string | undefined, schema: string[]): SuffixTransform[] {
    const transforms: SuffixTransform[] = [];
    if (!suffixStr) return transforms;
    
    // Parse patterns - split by space, each starts with @ or @@
    const patterns = suffixStr.trim().split(/\s+/).filter(p => p.startsWith('@'));
    
    for (const pattern of patterns) {
        const isDoubleSuffix = pattern.startsWith('@@');
        const value = isDoubleSuffix ? pattern.substring(2) : pattern.substring(1);
        
        // Determine which column and whether it's prefix or suffix
        let colIndex = -1;
        let type: 'prefix' | 'suffix' = 'prefix';
        
        // @@ always means suffix (append to end)
        if (isDoubleSuffix) {
            type = 'suffix';
            // Match to column based on value content
            if (value.includes('.com') || value.includes('.org') || value.includes('.net')) {
                colIndex = schema.findIndex(c => c.toLowerCase() === 'email');
            }
        } else {
            // Single @ - determine based on pattern content
            if (/^[A-Z]+-$/.test(value)) {
                // ID prefix like "ORD-"
                colIndex = schema.findIndex(c => c.toLowerCase() === 'id');
                type = 'prefix';
            } else if (/^\d{4}-\d{2}-$/.test(value)) {
                // Date prefix like "2025-01-"
                colIndex = schema.findIndex(c => c.toLowerCase() === 'date');
                type = 'prefix';
            } else if (/^\d{4}-\d{2}-\d{2}T$/.test(value)) {
                // Timestamp prefix like "2025-01-15T"
                colIndex = schema.findIndex(c => c.toLowerCase() === 'timestamp');
                type = 'prefix';
            } else if (value.startsWith('/')) {
                // API endpoint prefix like "/api/"
                colIndex = schema.findIndex(c => c.toLowerCase() === 'endpoint');
                type = 'prefix';
            } else if (value.includes('Trail') || value.startsWith('_')) {
                // Name suffix like "_Trail" - this is SUFFIX not prefix!
                colIndex = schema.findIndex(c => c.toLowerCase() === 'name');
                type = 'suffix';
            } else if (/^\d+\.$/.test(value)) {
                // Rating prefix like "4."
                colIndex = schema.findIndex(c => c.toLowerCase() === 'rating');
                type = 'prefix';
            }
        }
        
        if (colIndex >= 0) {
            transforms.push({ colIndex, type, value });
        }
    }
    
    return transforms;
}

/**
 * Apply suffix transformations to a row value
 */
function applyTransforms(val: string, colIndex: number, transforms: SuffixTransform[]): string {
    let result = val;
    for (const t of transforms) {
        if (t.colIndex === colIndex) {
            if (t.type === 'prefix') {
                result = t.value + result;
            } else {
                result = result + t.value;
            }
        }
    }
    return result;
}

/**
 * Convert LLM format to Human format
 */
export function llmToHumanFallback(llmContent: string): string {
    // First, handle multi-line tabular data by joining lines between [ and ]
    const normalizedContent = llmContent.replace(/\[[\r\n]+([\s\S]*?)[\r\n]+\]/g, (match, content) => {
        // Replace newlines with semicolons for row separation
        const rows = content.trim().split(/[\r\n]+/).filter((r: string) => r.trim());
        return '[' + rows.join(';') + ']';
    });
    
    const lines = normalizedContent.trim().split('\n');
    const rootScalars = new Map<string, string>();
    const sections = new Map<string, SectionData>();
    
    // Get or create section data
    const getSection = (name: string): SectionData => {
        if (!sections.has(name)) {
            sections.set(name, { scalars: new Map(), arrays: new Map() });
        }
        return sections.get(name)!;
    };
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        
        // Parse: section:count@=[key value key value ...] (key-value pairs without =)
        const kvPairsMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.]*):(\d+)@=\[(.+)\]$/);
        if (kvPairsMatch) {
            const sectionName = kvPairsMatch[1];
            const content = kvPairsMatch[3];
            const section = getSection(sectionName);
            const pairs = parseKVPairs(content);
            for (const [k, v] of pairs) {
                section.scalars.set(k, v);
            }
            continue;
        }
        
        // Parse: section:count[content] (inline object) - must NOT have (schema)
        // But CAN have nested arrays like key[count]=...
        const inlineObjMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.]*):(\d+)\[(.+)\]$/);
        if (inlineObjMatch && !trimmed.match(/:\d+\([^)]+\)/)) {
            const sectionName = inlineObjMatch[1];
            const content = inlineObjMatch[3];
            const parsed = parseInlineContent(content);
            const section = getSection(sectionName);
            for (const [k, v] of parsed.scalars) {
                section.scalars.set(k, v);
            }
            for (const [k, v] of parsed.arrays) {
                section.arrays.set(k, v);
            }
            continue;
        }
        
        // Parse: section:count(schema)@suffix...[rows] (tabular with suffix patterns)
        const tabularMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.]*):(\d+)\(([^)]+)\)((?:@[^\[]*)?)\[(.+)\]$/);
        if (tabularMatch) {
            const sectionName = tabularMatch[1];
            const schemaStr = tabularMatch[3];
            const suffixPattern = tabularMatch[4] || '';
            const rowsStr = tabularMatch[5];
            const schema = schemaStr.split(/\s+/).filter(s => s);
            
            // Determine row separator
            let rows: string[];
            if (rowsStr.includes(';')) {
                rows = rowsStr.split(';').map(r => r.trim()).filter(r => r);
            } else if (rowsStr.includes(', ')) {
                rows = rowsStr.split(', ').map(r => r.trim()).filter(r => r);
            } else if (rowsStr.includes(',')) {
                rows = rowsStr.split(',').map(r => r.trim()).filter(r => r);
            } else if (rowsStr.includes(':') && !rowsStr.includes('://')) {
                // Colon separator for logs - split by colon followed by timestamp pattern
                rows = rowsStr.split(/:(?=\d{2}:\d{2}:\d{2})/).map(r => r.trim()).filter(r => r);
            } else {
                rows = [rowsStr];
            }
            
            const section = getSection(sectionName);
            const parsedRows: string[][] = [];
            
            for (const row of rows) {
                const values = row.split(/\s+/).filter(v => v);
                if (values.length > 0) {
                    parsedRows.push(values);
                }
            }
            
            section.tabular = {
                schema,
                suffix: suffixPattern.trim() || undefined,
                rows: parsedRows
            };
            continue;
        }
        
        // Parse: section:count=item1 item2 item3 (simple array without brackets)
        const simpleArrayMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.]*):(\d+)=(.+)$/);
        if (simpleArrayMatch) {
            const sectionName = simpleArrayMatch[1];
            const items = simpleArrayMatch[3].split(/\s+/).map(s => s.trim()).filter(s => s.length > 0);
            const section = getSection(sectionName);
            section.arrays.set('items', items);
            continue;
        }
        
        // Parse: key[count]=item1 item2 item3 (root array)
        const arrayMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.-]*)\[(\d+)\]=(.*)$/);
        if (arrayMatch) {
            const key = arrayMatch[1];
            const items = arrayMatch[3].split(/\s+/).map(s => s.trim()).filter(s => s.length > 0);
            
            const dotIndex = key.indexOf('.');
            if (dotIndex !== -1) {
                const sectionName = key.substring(0, dotIndex);
                const localKey = key.substring(dotIndex + 1);
                const section = getSection(sectionName);
                section.arrays.set(localKey, items);
            } else {
                const section = getSection(key);
                section.arrays.set('items', items);
            }
            continue;
        }
        
        // Parse multiple key=value on same line
        if (trimmed.includes('=') && trimmed.includes(' ') && !trimmed.includes('[')) {
            const multiMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.-]*)=\S+(\s+[a-zA-Z_][a-zA-Z0-9_.-]*)=/);
            if (multiMatch) {
                const pairs = parseMultipleScalars(trimmed);
                for (const [key, value] of pairs) {
                    const dotIndex = key.indexOf('.');
                    if (dotIndex !== -1) {
                        const sectionName = key.substring(0, dotIndex);
                        const localKey = key.substring(dotIndex + 1);
                        const section = getSection(sectionName);
                        section.scalars.set(localKey, value);
                    } else {
                        rootScalars.set(key, value);
                    }
                }
                continue;
            }
        }
        
        // Parse: key=value (single scalar)
        const scalarMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_.-]*)=(.+)$/);
        if (scalarMatch) {
            const key = scalarMatch[1];
            const value = scalarMatch[2];
            
            const dotIndex = key.indexOf('.');
            if (dotIndex !== -1) {
                const sectionName = key.substring(0, dotIndex);
                const localKey = key.substring(dotIndex + 1);
                const section = getSection(sectionName);
                section.scalars.set(localKey, value);
            } else {
                rootScalars.set(key, value);
            }
            continue;
        }
    }
    
    const output: string[] = [];
    
    // Output root scalars first
    for (const [key, value] of rootScalars) {
        output.push(formatKV(key, humanizeValue(value)));
    }
    
    // Output sections
    for (const [sectionName, sectionData] of sections) {
        if (output.length > 0) output.push('');
        
        if (sectionData.tabular) {
            const { schema, suffix, rows } = sectionData.tabular;
            const transforms = parseSuffixPatterns(suffix, schema);
            
            // Output each row as [section:index] format
            for (let rowIdx = 0; rowIdx < rows.length; rowIdx++) {
                const rowValues = rows[rowIdx];
                if (rowIdx > 0) output.push('');
                output.push(`[${sectionName}:${rowIdx + 1}]`);
                for (let i = 0; i < schema.length; i++) {
                    const col = schema[i];
                    let val = rowValues[i] || '';
                    // Apply suffix/prefix transformations
                    val = applyTransforms(val, i, transforms);
                    output.push(formatKV(col, humanizeValue(val)));
                }
            }
        } else {
            output.push(`[${sectionName}]`);
            
            // Scalars first
            for (const [key, value] of sectionData.scalars) {
                output.push(formatKV(key, humanizeValue(value)));
            }
            
            // Then arrays
            for (const [key, items] of sectionData.arrays) {
                output.push(`${key}:`);
                for (const item of items) {
                    output.push(`- ${humanizeValue(item)}`);
                }
            }
        }
    }
    
    return output.join('\n');
}
