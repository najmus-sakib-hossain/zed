/**
 * DxCore - WASM wrapper with TypeScript fallback
 * 
 * Provides transformation between dense (disk) and human (editor) formats
 * with validation support. Uses WASM for performance with a TypeScript
 * fallback for reliability.
 * 
 * The dense format is the LLM format.
 * The human format is the TOML-like display format.
 * 
 * Requirements: 1.1-1.9, 2.1-2.7, 3.1-3.5, 5.1-5.4, 8.1-8.4
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { parseLlm, DxDocument, DxValue, DxSection } from './llmParser';
import { formatDocument } from './humanFormatter';
import { parseHuman, serializeToLlm } from './humanParser';

/**
 * Result of a transformation operation
 */
export interface TransformResult {
    success: boolean;
    content: string;
    error?: string;
}

/**
 * Result of a validation operation
 */
export interface ValidationResult {
    success: boolean;
    error?: string;
    line?: number;
    column?: number;
    hint?: string;
}

/**
 * DxCore interface for transformation operations
 */
export interface DxCore {
    /** Transform dense format to human-readable format */
    toHuman(dense: string): TransformResult;

    /** Transform human-readable format to dense format */
    toDense(human: string): TransformResult;

    /** Validate content syntax */
    validate(content: string): ValidationResult;

    /** Check if content is complete enough to save */
    isSaveable(content: string): boolean;

    /** Whether this is using WASM or fallback */
    readonly isWasm: boolean;
}

/**
 * WASM module interface (matches wasm-bindgen output)
 */
interface WasmModule {
    default: (input?: Buffer | ArrayBuffer | Uint8Array) => Promise<void>;
    DxSerializer: new () => WasmSerializer;
}

interface WasmSerializer {
    toHuman(dense: string): WasmTransformResult;
    toDense(human: string): WasmTransformResult;
    validate(content: string): WasmValidationResult;
    isSaveable(content: string): boolean;
    maxInputSize(): number;
    maxRecursionDepth(): number;
    maxTableRows(): number;
}


interface WasmTransformResult {
    success: boolean;
    content: string;
    error?: string;
}

interface WasmValidationResult {
    success: boolean;
    error?: string;
    line?: number;
    column?: number;
    hint?: string;
}

/**
 * WASM-based DxCore implementation
 * 
 * NOTE: toHuman() uses TypeScript formatter instead of WASM because:
 * - WASM outputs OLD format with decorative comments and Unicode tables
 * - TypeScript outputs correct TOML-like Human Format V3
 * - WASM is still used for toDense() and validate() (battle-hardened security limits)
 */
class WasmDxCore implements DxCore {
    private serializer: WasmSerializer;
    readonly isWasm = true;
    private keyPadding: number;

    constructor(serializer: WasmSerializer, keyPadding: number = 20) {
        this.serializer = serializer;
        this.keyPadding = keyPadding;
    }

    toHuman(dense: string): TransformResult {
        // FIXED: Use TypeScript formatter instead of WASM
        // WASM outputs old format with Unicode tables and decorative comments
        // TypeScript outputs correct TOML-like Human Format V3
        try {
            const content = formatDx(dense, 2, this.keyPadding);
            return { success: true, content };
        } catch (error) {
            return {
                success: false,
                content: '',
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }

    toDense(human: string): TransformResult {
        // FIXED: Use TypeScript minifyDx instead of WASM
        // WASM uses old HumanParser that expects [config] headers
        // TypeScript uses parseHumanV3 that handles Human Format V3 (no [config] header)
        try {
            const content = minifyDx(human);
            return { success: true, content };
        } catch (error) {
            return {
                success: false,
                content: '',
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }

    validate(content: string): ValidationResult {
        const result = this.serializer.validate(content);
        return {
            success: result.success,
            error: result.error,
            line: result.line,
            column: result.column,
            hint: result.hint,
        };
    }

    isSaveable(content: string): boolean {
        return this.serializer.isSaveable(content);
    }
}

// ============================================================================
// TypeScript Fallback Implementation
// ============================================================================

/**
 * Apply smart quoting to a string value
 * 
 * - If string contains apostrophe ('), wrap in double quotes
 * - If string contains both ' and ", use double quotes with escaped "
 */
export function smartQuote(value: string): string {
    const hasSingle = value.includes("'");
    const hasDouble = value.includes('"');

    if (!hasSingle && !hasDouble) {
        // No quotes needed for simple strings without spaces/special chars
        if (!/[ #|^:]/.test(value)) {
            return value;
        }
        // Default to double quotes
        return `"${value}"`;
    }

    if (hasSingle && !hasDouble) {
        // Contains apostrophe - use double quotes
        return `"${value}"`;
    }

    if (hasDouble && !hasSingle) {
        // Contains double quotes - use single quotes
        return `'${value}'`;
    }

    // Contains both - use double quotes with escaped double quotes
    const escaped = value.replace(/"/g, '\\"');
    return `"${escaped}"`;
}


/**
 * Format DX content to human-readable format (TypeScript fallback)
 * 
 * Transforms LLM format like:
 *   name=dx
 *   version=0.0.1
 *   workspace:1[paths[2]=@/www @/backend]
 *   editors:2[default=neovim items[7]=neovim zed vscode cursor antigravity replit firebase-studio]
 * 
 * To Human format like:
 *   name                         = dx
 *   version                      = 0.0.1
 *   
 *   [workspace]
 *   paths:
 *   - @/www
 *   - @/backend
 *   
 *   [editors]
 *   default                      = neovim
 *   items:
 *   - neovim
 *   - zed
 *   - vscode
 * 
 * Requirements: 1.1-1.9, 2.1-2.7
 */
export function formatDx(dense: string, indentSize: number = 2, keyPadding: number = 28): string {
    if (!dense.trim()) {
        return '';
    }

    // Parse LLM format
    const parseResult = parseLlm(dense);

    if (!parseResult.success || !parseResult.document) {
        // If parsing fails, return the original content
        return dense;
    }

    // Check if document has content
    const doc = parseResult.document;
    if (doc.context.size > 0 || doc.sections.size > 0) {
        return formatDocument(doc);
    }

    return dense;
}

/**
 * Format a value for human-readable output (legacy helper)
 */
function formatValueLegacy(value: string): string {
    // Handle special values
    if (value === '1' || value === 'true') return 'true';
    if (value === '0' || value === 'false') return 'false';
    if (value === 'null' || value === '') return 'null';

    // Check if value needs quoting
    if (/[ #|^:\n\t]/.test(value) || value.includes("'") || value.includes('"')) {
        return smartQuote(value);
    }

    return value;
}


/**
 * Minify DX content to dense format (TypeScript fallback)
 * 
 * Transforms Human format like:
 *   name                 = Test
 *   count                = 42
 *   
 *   [d]                  = Id | Name | Enabled
 *   1                    = Alpha | true
 *   2                    = Beta | false
 * 
 * To LLM format like:
 *   #c:nm|Test;ct|42
 *   #d(id|nm|en)
 *   1|Alpha|+
 *   2|Beta|-
 * 
 * Requirements: 3.1-3.5
 */
export function minifyDx(human: string): string {
    if (!human.trim()) {
        return '';
    }

    // Check if content is already in LLM format
    const trimmed = human.trim();
    // LLM format starts with key=value (no spaces around =) or section:count[...]
    if (/^[a-zA-Z_][a-zA-Z0-9_]*=[^\s]/.test(trimmed) || 
        /^[a-zA-Z_][a-zA-Z0-9_]*:\d+[\[(]/.test(trimmed)) {
        // Already in LLM format - return as-is
        return human;
    }

    // Parse Human format
    const parseResult = parseHuman(human);

    if (!parseResult.success || !parseResult.document) {
        // If parsing fails, return the original content
        return human;
    }

    // Check if the parsed document has any content
    const doc = parseResult.document;
    if (doc.context.size === 0 && doc.sections.size === 0) {
        return human;
    }

    // Serialize to LLM format
    return serializeToLlm(parseResult.document);
}

/**
 * Parse a value from human format to dense format (legacy helper)
 */
function parseValueLegacy(value: string): string {
    // Handle quoted strings
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
        return value.slice(1, -1);
    }

    // Handle booleans
    if (value === 'true') return '1';
    if (value === 'false') return '0';
    if (value === 'null') return '';

    return value;
}


/**
 * Validate DX content syntax (TypeScript fallback)
 * 
 * Validates both LLM format and human format:
 * - LLM format: sigil syntax, reference definitions, schema/row consistency
 * - Human format: brackets, strings, general syntax
 * 
 * Requirements: 8.1-8.4
 */
export function validateDx(content: string): ValidationResult {
    if (!content.trim()) {
        return { success: true };
    }

    // Detect format type
    // Detect format: DSR uses key=value and [], legacy uses # sigils and | separators
    const isDsrFormat = /^[a-zA-Z_][a-zA-Z0-9_]*[=\[]/.test(content.trim()) ||
                        /^[a-zA-Z_][a-zA-Z0-9_]*:\d+[=(]/.test(content.trim());
    const isLegacyLlmFormat = content.trim().startsWith('#');
    const isLlmFormat = isDsrFormat || isLegacyLlmFormat || content.includes('|');

    if (isLlmFormat) {
        return validateLlmFormat(content);
    } else {
        return validateHumanFormat(content);
    }
}

/**
 * Validate LLM format content
 * 
 * Checks for:
 * - DSR format: key=value, name[key=value,...], name:count(schema)[data]
 * - Valid sigil syntax (#:, #<letter>) - legacy
 * - Root-level key|value pairs (legacy format)
 * - Legacy #c: context format (still supported)
 * - Reference definitions
 * - Schema/row consistency
 * 
 * Requirements: 8.1-8.4
 */
function validateLlmFormat(content: string): ValidationResult {
    const lines = content.split('\n');
    const definedRefs = new Set<string>();
    const usedRefs = new Set<string>();
    let currentSchema: string[] = [];
    let currentSectionId = '';
    let lineNum = 0;
    let inDsrTable = false;

    for (const line of lines) {
        lineNum++;
        const trimmed = line.trim();

        // Skip empty lines and comments
        if (!trimmed || trimmed.startsWith('//')) {
            continue;
        }

        // DSR format: name[key=value,key2=value2] (object)
        if (/^[a-zA-Z_][a-zA-Z0-9_]*\[/.test(trimmed) && trimmed.includes('=')) {
            // Validate bracket closure
            if (!trimmed.endsWith(']')) {
                return {
                    success: false,
                    error: `Unclosed bracket in object definition`,
                    line: lineNum,
                    column: trimmed.length,
                    hint: 'Objects should be in format name[key=value,key2=value2]',
                };
            }
            continue;
        }

        // DSR format: name:count(schema)[data] (table start)
        const tableMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_]*):(\d+)\(([^)]+)\)\[$/);
        if (tableMatch) {
            currentSectionId = tableMatch[1];
            currentSchema = tableMatch[3].split(',').map(col => col.trim()).filter(col => col);
            inDsrTable = true;
            if (currentSchema.length === 0) {
                return {
                    success: false,
                    error: `Empty schema in table header`,
                    line: lineNum,
                    column: 1,
                    hint: 'Schema must have at least one column',
                };
            }
            continue;
        }

        // DSR table end
        if (trimmed === ']' && inDsrTable) {
            inDsrTable = false;
            currentSchema = [];
            continue;
        }

        // DSR table row (comma-separated)
        if (inDsrTable && currentSchema.length > 0) {
            const values = trimmed.split(',');
            if (values.length !== currentSchema.length) {
                return {
                    success: false,
                    error: `Row has ${values.length} columns, expected ${currentSchema.length}`,
                    line: lineNum,
                    column: 1,
                    hint: `Schema for table '${currentSectionId}' has columns: ${currentSchema.join(', ')}`,
                };
            }
            continue;
        }

        // DSR format: name:count=item1,item2 (array)
        if (/^[a-zA-Z_][a-zA-Z0-9_]*:\d+=/.test(trimmed)) {
            continue;
        }

        // DSR format: key=value (simple pair, no spaces around =)
        if (/^[a-zA-Z_][a-zA-Z0-9_]*=[^\s]/.test(trimmed)) {
            continue;
        }

        // Legacy context section: #c:... (still supported)
        if (trimmed.startsWith('#c:')) {
            const content = trimmed.substring(3);
            // Validate context format
            if (content && !content.includes('|')) {
                return {
                    success: false,
                    error: `Invalid context format: missing pipe separator`,
                    line: lineNum,
                    column: 4,
                    hint: 'Context format should be #c:key|value;key|value',
                };
            }
            // Check for reference usage in context
            const refMatches = content.match(/\^([a-zA-Z0-9_]+)/g);
            if (refMatches) {
                for (const ref of refMatches) {
                    usedRefs.add(ref.substring(1));
                }
            }
            currentSchema = [];
            continue;
        }

        // Reference definition: #:...
        if (trimmed.startsWith('#:')) {
            const content = trimmed.substring(2);
            const pipeIdx = content.indexOf('|');
            if (pipeIdx === -1) {
                return {
                    success: false,
                    error: `Invalid reference definition: missing pipe separator`,
                    line: lineNum,
                    column: 3,
                    hint: 'Reference format should be #:key|value',
                };
            }
            const refKey = content.substring(0, pipeIdx).trim();
            if (!refKey) {
                return {
                    success: false,
                    error: `Invalid reference definition: empty key`,
                    line: lineNum,
                    column: 3,
                    hint: 'Reference key cannot be empty',
                };
            }
            definedRefs.add(refKey);
            currentSchema = [];
            continue;
        }

        // Data section header: #<letter>(schema) - legacy
        if (trimmed.startsWith('#') && trimmed.includes('(')) {
            const match = trimmed.match(/^#([a-zA-Z])\(([^)]*)\)$/);
            if (!match) {
                return {
                    success: false,
                    error: `Invalid section header: ${trimmed}`,
                    line: lineNum,
                    column: 1,
                    hint: 'Section headers should be in format #<letter>(col1|col2|col3)',
                };
            }
            currentSectionId = match[1];
            currentSchema = match[2].split('|').map(col => col.trim()).filter(col => col);
            if (currentSchema.length === 0) {
                return {
                    success: false,
                    error: `Empty schema in section header`,
                    line: lineNum,
                    column: 3,
                    hint: 'Schema must have at least one column',
                };
            }
            continue;
        }

        // Unknown sigil (but not root-level key|value)
        if (trimmed.startsWith('#')) {
            const sigil = trimmed.substring(0, 2);
            return {
                success: false,
                error: `Unknown sigil '${sigil}'`,
                line: lineNum,
                column: 1,
                hint: 'Valid sigils are #: (reference), #<letter>( (data section)',
            };
        }

        // Data row (if we have a schema) - legacy pipe format
        if (currentSchema.length > 0 && !inDsrTable) {
            const values = trimmed.split('|');
            if (values.length !== currentSchema.length) {
                return {
                    success: false,
                    error: `Row has ${values.length} columns, expected ${currentSchema.length}`,
                    line: lineNum,
                    column: 1,
                    hint: `Schema for section '${currentSectionId}' has columns: ${currentSchema.join(', ')}`,
                };
            }
            // Check for reference usage in row
            for (const value of values) {
                if (value.trim().startsWith('^')) {
                    usedRefs.add(value.trim().substring(1));
                }
            }
            continue;
        }

        // Root-level key|value pair (legacy format context)
        if (trimmed.includes('|') && !trimmed.startsWith('#')) {
            // Valid root-level context pair
            continue;
        }
    }

    // Check for unclosed DSR table
    if (inDsrTable) {
        return {
            success: false,
            error: `Unclosed table: missing closing bracket`,
            line: lineNum,
            column: 1,
            hint: 'Tables should end with ]',
        };
    }

    // Check for undefined references (warning, not error)
    // We don't fail on undefined refs as they may be intentional

    return { success: true };
}

/**
 * Validate human format content
 * 
 * Checks for:
 * - Unclosed brackets
 * - Unclosed strings
 * - Mismatched brackets
 */
function validateHumanFormat(content: string): ValidationResult {
    const bracketStack: Array<{ char: string; line: number; column: number }> = [];
    let inString = false;
    let stringChar = '"';
    let stringStart: { line: number; column: number } | null = null;

    const lines = content.split('\n');

    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
        const line = lines[lineIdx];
        const lineNum = lineIdx + 1;
        let col = 0;

        for (let i = 0; i < line.length; i++) {
            const ch = line[i];
            col = i + 1;

            // Handle escape sequences in strings
            if (inString && ch === '\\' && i + 1 < line.length) {
                i++; // Skip escaped character
                continue;
            }

            // Handle string boundaries
            if (!inString && (ch === '"' || ch === "'")) {
                inString = true;
                stringChar = ch;
                stringStart = { line: lineNum, column: col };
                continue;
            }

            if (inString && ch === stringChar) {
                inString = false;
                stringStart = null;
                continue;
            }

            // Skip bracket checking inside strings
            if (inString) {
                continue;
            }

            // Track brackets
            if (ch === '{' || ch === '[' || ch === '(') {
                bracketStack.push({ char: ch, line: lineNum, column: col });
            } else if (ch === '}' || ch === ']' || ch === ')') {
                const expected = ch === '}' ? '{' : ch === ']' ? '[' : '(';

                if (bracketStack.length === 0) {
                    return {
                        success: false,
                        error: `Unexpected closing bracket '${ch}'`,
                        line: lineNum,
                        column: col,
                        hint: `No matching opening bracket for '${ch}'`,
                    };
                }

                const open = bracketStack.pop()!;
                if (open.char !== expected) {
                    const expectedCloseForOpen = open.char === '{' ? '}' : open.char === '[' ? ']' : ')';
                    return {
                        success: false,
                        error: `Mismatched bracket: expected '${expectedCloseForOpen}' but found '${ch}'`,
                        line: lineNum,
                        column: col,
                        hint: `Opening '${open.char}' at line ${open.line}, column ${open.column} expects '${expectedCloseForOpen}'`,
                    };
                }
            }
        }
    }

    // Check for unclosed strings
    if (inString && stringStart) {
        return {
            success: false,
            error: `Unclosed string starting with '${stringChar}'`,
            line: stringStart.line,
            column: stringStart.column,
            hint: `Add a closing '${stringChar}' to complete the string`,
        };
    }

    // Check for unclosed brackets
    if (bracketStack.length > 0) {
        const open = bracketStack[bracketStack.length - 1];
        const expectedClose = open.char === '{' ? '}' : open.char === '[' ? ']' : ')';
        return {
            success: false,
            error: `Unclosed bracket '${open.char}'`,
            line: open.line,
            column: open.column,
            hint: `Add a closing '${expectedClose}' to match the opening '${open.char}'`,
        };
    }

    return { success: true };
}


/**
 * TypeScript fallback DxCore implementation
 */
class FallbackDxCore implements DxCore {
    readonly isWasm = false;
    private indentSize: number;
    private keyPadding: number;

    constructor(indentSize: number = 2, keyPadding: number = 20) {
        this.indentSize = indentSize;
        this.keyPadding = keyPadding;
    }

    toHuman(dense: string): TransformResult {
        try {
            const content = formatDx(dense, this.indentSize, this.keyPadding);
            return { success: true, content };
        } catch (error) {
            return {
                success: false,
                content: '',
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }

    toDense(human: string): TransformResult {
        try {
            const content = minifyDx(human);
            return { success: true, content };
        } catch (error) {
            return {
                success: false,
                content: '',
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }

    validate(content: string): ValidationResult {
        return validateDx(content);
    }

    isSaveable(content: string): boolean {
        return this.validate(content).success;
    }
}

// ============================================================================
// DxCore Loader
// ============================================================================

let cachedCore: DxCore | null = null;

/**
 * Load the DxCore, attempting WASM first with TypeScript fallback
 * 
 * The WASM module provides the battle-hardened Rust serializer with:
 * - Security limits (100 MB input, 1000 recursion depth, 10M table rows)
 * - 38 property-based tests for correctness
 * - Performance optimizations
 * 
 * @param extensionPath - Path to the extension directory
 * @param indentSize - Indent size for formatting (default: 2)
 * @param keyPadding - Minimum key padding width (default: 20)
 * @returns DxCore instance
 */
export async function loadDxCore(
    extensionPath: string,
    indentSize: number = 2,
    keyPadding: number = 20
): Promise<DxCore> {
    // Return cached instance if available
    if (cachedCore) {
        return cachedCore;
    }

    // Try to load WASM module first
    try {
        const wasmJsPath = path.join(extensionPath, 'wasm', 'dx_serializer.js');
        const wasmBinaryPath = path.join(extensionPath, 'wasm', 'dx_serializer_bg.wasm');

        if (fs.existsSync(wasmJsPath) && fs.existsSync(wasmBinaryPath)) {
            // Dynamic import of the WASM module
            const wasmModule = await import(wasmJsPath) as WasmModule;

            // Read the WASM binary and initialize
            const wasmBinary = fs.readFileSync(wasmBinaryPath);
            await wasmModule.default(wasmBinary);

            // Create the serializer instance
            const serializer = new wasmModule.DxSerializer();
            cachedCore = new WasmDxCore(serializer, keyPadding);

            console.log('DX Serializer: Using WASM core (battle-hardened) with TypeScript formatter');
            console.log(`  - Max input size: ${serializer.maxInputSize()} bytes`);
            console.log(`  - Max recursion depth: ${serializer.maxRecursionDepth()}`);
            console.log(`  - Max table rows: ${serializer.maxTableRows()}`);

            return cachedCore;
        }
    } catch (error) {
        console.warn('DX Serializer: WASM load failed, using TypeScript fallback', error);
    }

    // Fallback to TypeScript implementation
    cachedCore = new FallbackDxCore(indentSize, keyPadding);
    console.log('DX Serializer: Using TypeScript core (fallback)');
    return cachedCore;
}

/**
 * Get the cached DxCore instance, or create a fallback if not loaded
 */
export function getDxCore(indentSize: number = 2, keyPadding: number = 20): DxCore {
    if (!cachedCore) {
        cachedCore = new FallbackDxCore(indentSize, keyPadding);
    }
    return cachedCore;
}

/**
 * Clear the cached DxCore instance (for testing)
 */
export function clearDxCoreCache(): void {
    cachedCore = null;
}

/**
 * Create a fallback DxCore instance directly (for testing)
 */
export function createFallbackCore(indentSize: number = 2, keyPadding: number = 20): DxCore {
    return new FallbackDxCore(indentSize, keyPadding);
}
