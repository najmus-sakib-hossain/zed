/**
 * Format Detector for DX Serializer VS Code Extension
 * 
 * Detects input file format to enable automatic conversion:
 * - JSON: starts with { or [
 * - YAML: : patterns, ---, - at line start
 * - TOML: [section] with key = value
 * - CSV: comma-separated with consistent columns
 * - LLM: #c:, #:, #<letter>(
 * - Human V3: key = value patterns
 * - Machine: DXMB magic bytes
 * 
 * Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 6.1, 6.2, 6.3
 */

// ============================================================================
// Types
// ============================================================================

/**
 * Markdown Format enum for Markdown-specific format detection
 * Requirements: 6.1, 6.2, 6.3
 */
export enum MarkdownFormat {
    /** LLM format (token-optimized) - starts with numbered sections or metadata */
    Llm = 'llm',
    /** Human format (Markdown-like) - starts with [meta] or # */
    Human = 'human',
    /** Machine format (binary) - starts with DXMB magic bytes */
    Machine = 'machine',
    /** Unknown format */
    Unknown = 'unknown'
}

export type DetectedFormat =
    | 'json'
    | 'yaml'
    | 'toml'
    | 'csv'
    | 'llm'
    | 'human-v3'
    | 'machine'
    | 'unknown';

export interface FormatDetectionResult {
    format: DetectedFormat;
    confidence: number; // 0-1
    hints: string[];
}

// ============================================================================
// Markdown Format Detection (Requirements: 6.1, 6.2, 6.3)
// ============================================================================

/** DXMB magic bytes for machine format detection */
const DXMB_MAGIC = 'DXMB';
const DXMB_MAGIC_BYTES = [0x44, 0x58, 0x4D, 0x42]; // D X M B

/**
 * Detect Markdown format from content
 * Requirements: 6.1, 6.2, 6.3
 * 
 * @param content - String or byte content to detect
 * @returns MarkdownFormat enum value
 */
export function detectMarkdownFormat(content: string | Uint8Array): MarkdownFormat {
    // Handle byte array input
    if (content instanceof Uint8Array) {
        return detectMarkdownFormatFromBytes(content);
    }

    // Handle string input
    const trimmed = content.trim();

    // Check for machine format (DXMB magic bytes as string)
    if (trimmed.startsWith(DXMB_MAGIC)) {
        return MarkdownFormat.Machine;
    }

    // Check for LLM format markers (Requirements: 6.1)
    // - Starts with numbered sections
    // - Line matches N| pattern (numbered header)
    if (/^\d+\|/.test(trimmed) || trimmed.startsWith('meta|')) {
        return MarkdownFormat.Llm;
    }

    // Check for LLM format sigils
    if (trimmed.includes('#c:') || trimmed.includes('#:') || /#[a-z]\(/.test(trimmed)) {
        return MarkdownFormat.Llm;
    }

    // Check for root-level key|value pairs (new LLM format)
    const lines = trimmed.split('\n');
    const rootKeyValuePattern = /^[a-z]{1,4}\|/;
    const rootKeyValueLines = lines.filter(l => rootKeyValuePattern.test(l.trim())).length;
    if (rootKeyValueLines >= 2) {
        return MarkdownFormat.Llm;
    }

    // Check for Human format markers (Requirements: 6.2)
    // - Starts with [meta]
    // - Starts with # (Markdown header)
    // - Contains [refs] section
    if (trimmed.startsWith('[meta]') || trimmed.startsWith('#') || trimmed.includes('[refs]')) {
        return MarkdownFormat.Human;
    }

    // Check for Human format key = value patterns with padding
    const keyValuePattern = /^[a-zA-Z_][a-zA-Z0-9_]*\s{2,}=\s+.+$/;
    const keyValueLines = lines.filter(l => keyValuePattern.test(l.trim())).length;
    if (keyValueLines >= 2) {
        return MarkdownFormat.Human;
    }

    return MarkdownFormat.Unknown;
}

/**
 * Detect Markdown format from byte array
 * Requirements: 6.3
 */
function detectMarkdownFormatFromBytes(bytes: Uint8Array): MarkdownFormat {
    // Check for DXMB magic bytes
    if (bytes.length >= 4 &&
        bytes[0] === DXMB_MAGIC_BYTES[0] &&
        bytes[1] === DXMB_MAGIC_BYTES[1] &&
        bytes[2] === DXMB_MAGIC_BYTES[2] &&
        bytes[3] === DXMB_MAGIC_BYTES[3]) {
        return MarkdownFormat.Machine;
    }

    // Convert to string and use string detection
    const decoder = new TextDecoder('utf-8', { fatal: false });
    const content = decoder.decode(bytes);
    return detectMarkdownFormat(content);
}

/**
 * Check if content is Markdown Machine format (binary)
 * Requirements: 6.3
 */
export function isMachineFormat(content: string | Uint8Array): boolean {
    return detectMarkdownFormat(content) === MarkdownFormat.Machine;
}

/**
 * Check if content is Markdown LLM format
 * Requirements: 6.1
 */
export function isLlmFormat(content: string): boolean {
    return detectMarkdownFormat(content) === MarkdownFormat.Llm;
}

/**
 * Check if content is Markdown Human format
 * Requirements: 6.2
 */
export function isHumanFormat(content: string): boolean {
    return detectMarkdownFormat(content) === MarkdownFormat.Human;
}

// ============================================================================
// Detection Functions
// ============================================================================

/**
 * Detect if content is JSON format
 * Requirement: 5.1
 */
export function detectJson(content: string): FormatDetectionResult {
    const trimmed = content.trim();
    const hints: string[] = [];
    let confidence = 0;

    // Check for JSON object or array start
    if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
        hints.push('Starts with { or [');
        confidence += 0.5;

        // Try to parse as JSON
        try {
            JSON.parse(trimmed);
            hints.push('Valid JSON syntax');
            confidence += 0.5;
        } catch {
            hints.push('Invalid JSON syntax');
            confidence -= 0.3;
        }
    }

    return { format: 'json', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

/**
 * Detect if content is YAML format
 * Requirement: 5.2
 */
export function detectYaml(content: string): FormatDetectionResult {
    const lines = content.split('\n');
    const hints: string[] = [];
    let confidence = 0;

    // Check for YAML document start
    if (content.trim().startsWith('---')) {
        hints.push('Starts with ---');
        confidence += 0.4;
    }

    // Check for key: value patterns
    let colonPatterns = 0;
    let listItems = 0;

    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;

        // key: value pattern (not URL)
        if (/^[a-zA-Z_][a-zA-Z0-9_]*:\s?/.test(trimmed) && !trimmed.includes('://')) {
            colonPatterns++;
        }

        // List item: - value (with or without space)
        if (/^-\s*\S/.test(trimmed)) {
            listItems++;
        }
    }

    if (colonPatterns >= 2) {
        hints.push(`Found ${colonPatterns} key: value patterns`);
        confidence += 0.3;
    }

    if (listItems >= 1) {
        hints.push(`Found ${listItems} list items`);
        confidence += 0.2;
    }

    // Combined: key: with list items is strong YAML signal
    if (colonPatterns >= 1 && listItems >= 1) {
        confidence += 0.2;
    }

    // Negative: has = signs (more likely TOML)
    const equalSigns = lines.filter(l => /^\s*[a-zA-Z_][a-zA-Z0-9_]*\s*=/.test(l)).length;
    if (equalSigns > colonPatterns) {
        confidence -= 0.3;
    }

    return { format: 'yaml', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

/**
 * Detect if content is TOML format
 * Requirement: 5.3
 */
export function detectToml(content: string): FormatDetectionResult {
    const lines = content.split('\n');
    const hints: string[] = [];
    let confidence = 0;

    let sectionHeaders = 0;
    let keyValuePairs = 0;

    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;

        // [section] header (not array)
        if (/^\[[a-zA-Z_][a-zA-Z0-9_]*\]$/.test(trimmed)) {
            sectionHeaders++;
        }

        // key = value pattern
        if (/^[a-zA-Z_][a-zA-Z0-9_]*\s*=\s*.+$/.test(trimmed)) {
            keyValuePairs++;
        }
    }

    if (sectionHeaders >= 1) {
        hints.push(`Found ${sectionHeaders} [section] headers`);
        confidence += 0.4;
    }

    if (keyValuePairs >= 2) {
        hints.push(`Found ${keyValuePairs} key = value pairs`);
        confidence += 0.3;
    }

    // Negative: has LLM markers
    if (content.includes('#c:') || content.includes('#f(') || content.includes('#k(')) {
        confidence -= 0.5;
    }

    return { format: 'toml', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

/**
 * Detect if content is CSV format
 * Requirement: 5.4
 */
export function detectCsv(content: string): FormatDetectionResult {
    const lines = content.split('\n').filter(l => l.trim());
    const hints: string[] = [];
    let confidence = 0;

    if (lines.length < 2) {
        return { format: 'csv', confidence: 0, hints: ['Too few lines'] };
    }

    // Count commas per line
    const commaCounts = lines.map(l => (l.match(/,/g) || []).length);

    // Check for consistent comma count
    const firstCount = commaCounts[0];
    if (firstCount >= 1) {
        const consistent = commaCounts.every(c => c === firstCount);
        if (consistent) {
            hints.push(`Consistent ${firstCount + 1} columns`);
            confidence += 0.5;
        } else {
            hints.push('Inconsistent column count');
            confidence += 0.2;
        }
    }

    // Check for header row (no numbers in first row, numbers in subsequent)
    const firstRow = lines[0].split(',');
    const hasHeaderLike = firstRow.every(cell => !/^\d+$/.test(cell.trim()));
    if (hasHeaderLike && lines.length > 1) {
        hints.push('Has header-like first row');
        confidence += 0.2;
    }

    // Negative: has = signs or : patterns
    if (content.includes(' = ') || /^[a-z]+:/.test(content)) {
        confidence -= 0.3;
    }

    return { format: 'csv', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

/**
 * Detect if content is LLM/DSR format
 * Requirement: 5.5
 * 
 * DSR format (latest): key=value, name[key=value,key2=value2], name:count(schema)[data]
 * Legacy format: #c:key|value;key|value (still supported)
 */
export function detectLlm(content: string): FormatDetectionResult {
    const hints: string[] = [];
    let confidence = 0;

    const lines = content.split('\n');

    // Check for DSR format (latest) - comma-separated with = syntax
    // Objects: name[key=value,key2=value2]
    const dsrObjectPattern = /^[a-zA-Z_][a-zA-Z0-9_]*\[[a-zA-Z_][a-zA-Z0-9_]*=/;
    const dsrObjects = lines.filter(l => dsrObjectPattern.test(l.trim())).length;
    if (dsrObjects >= 1) {
        hints.push(`Has ${dsrObjects} DSR object patterns`);
        confidence += 0.5;
    }

    // Tables: name:count(schema)[data]
    const dsrTablePattern = /^[a-zA-Z_][a-zA-Z0-9_]*:\d+\([^)]+\)\[/;
    const dsrTables = lines.filter(l => dsrTablePattern.test(l.trim())).length;
    if (dsrTables >= 1) {
        hints.push(`Has ${dsrTables} DSR table patterns`);
        confidence += 0.5;
    }

    // Arrays: name:count=item1,item2
    const dsrArrayPattern = /^[a-zA-Z_][a-zA-Z0-9_]*:\d+=[^,]+,/;
    const dsrArrays = lines.filter(l => dsrArrayPattern.test(l.trim())).length;
    if (dsrArrays >= 1) {
        hints.push(`Has ${dsrArrays} DSR array patterns`);
        confidence += 0.4;
    }

    // Simple key=value (DSR style - no spaces around =)
    const dsrKeyValuePattern = /^[a-zA-Z_][a-zA-Z0-9_]*=[^\s=]+$/;
    const dsrKeyValues = lines.filter(l => dsrKeyValuePattern.test(l.trim())).length;
    if (dsrKeyValues >= 2) {
        hints.push(`Has ${dsrKeyValues} DSR key=value pairs`);
        confidence += 0.3;
    }

    // Check for legacy context marker (still supported)
    if (content.includes('#c:')) {
        hints.push('Has #c: context marker (legacy)');
        confidence += 0.4;
    }

    // Check for reference marker (legacy)
    if (content.includes('#:')) {
        hints.push('Has #: reference marker (legacy)');
        confidence += 0.2;
    }

    // Check for section markers #f( #k( #y( etc. (legacy)
    if (/#[a-z]\(/.test(content)) {
        hints.push('Has #x( section markers (legacy)');
        confidence += 0.4;
    }

    // Check for pipe-separated data rows (legacy format)
    const pipeRows = lines.filter(l => !l.startsWith('#') && l.includes('|') && !l.includes(' | ')).length;
    if (pipeRows >= 1) {
        hints.push(`Has ${pipeRows} pipe-separated rows (legacy)`);
        confidence += 0.2;
    }

    return { format: 'llm', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

/**
 * Detect if content is Human V3 format
 * Requirement: 5.6
 */
export function detectHumanV3(content: string): FormatDetectionResult {
    const lines = content.split('\n');
    const hints: string[] = [];
    let confidence = 0;

    let keyValuePairs = 0;
    let sectionHeaders = 0;
    let pipeArrays = 0;

    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;

        // [section] header with optional = schema
        if (/^\[[a-zA-Z0-9_]+\](\s*=\s*.+)?$/.test(trimmed)) {
            sectionHeaders++;
        }

        // key = value with padding (2+ spaces before =)
        if (/^[a-zA-Z_][a-zA-Z0-9_]*\s{2,}=\s+.+$/.test(trimmed)) {
            keyValuePairs++;
        }

        // Pipe-separated arrays
        if (trimmed.includes(' | ')) {
            pipeArrays++;
        }
    }

    if (keyValuePairs >= 2) {
        hints.push(`Found ${keyValuePairs} padded key = value pairs`);
        confidence += 0.4;
    }

    if (sectionHeaders >= 1) {
        hints.push(`Found ${sectionHeaders} [section] headers`);
        confidence += 0.2;
    }

    if (pipeArrays >= 1) {
        hints.push(`Found ${pipeArrays} pipe-separated arrays`);
        confidence += 0.2;
    }

    // Section header with schema is strong Human V3 signal
    if (sectionHeaders >= 1 && pipeArrays >= 1) {
        confidence += 0.2;
    }

    // Negative: has LLM markers
    if (content.includes('#c:') || /#[a-z]\(/.test(content)) {
        confidence -= 0.5;
    }

    // Negative: has decorative borders (old format)
    if (content.includes('════') || content.includes('┌───')) {
        confidence -= 0.3;
    }

    return { format: 'human-v3', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

// ============================================================================
// Main Detector
// ============================================================================

/**
 * Detect if content is Machine (binary) format
 * Requirements: 6.3
 */
export function detectMachine(content: string): FormatDetectionResult {
    const hints: string[] = [];
    let confidence = 0;

    // Check for DXMB magic bytes
    if (content.startsWith(DXMB_MAGIC)) {
        hints.push('Starts with DXMB magic bytes');
        confidence = 1.0;
    }

    // Check for binary-like content (non-printable characters)
    const nonPrintable = content.slice(0, 100).split('').filter(c => {
        const code = c.charCodeAt(0);
        return code < 32 && code !== 9 && code !== 10 && code !== 13;
    }).length;

    if (nonPrintable > 5) {
        hints.push('Contains non-printable characters');
        confidence += 0.3;
    }

    return { format: 'machine', confidence: Math.max(0, Math.min(1, confidence)), hints };
}

/**
 * Detect the format of input content
 * Returns the most likely format with confidence score
 */
export function detectFormat(content: string): FormatDetectionResult {
    if (!content || !content.trim()) {
        return { format: 'unknown', confidence: 0, hints: ['Empty content'] };
    }

    // Check for machine format first (binary)
    const machineResult = detectMachine(content);
    if (machineResult.confidence >= 0.8) {
        return machineResult;
    }

    // Run all detectors
    const results: FormatDetectionResult[] = [
        detectLlm(content),      // Check LLM first (most specific)
        detectJson(content),
        detectYaml(content),
        detectToml(content),
        detectCsv(content),
        detectHumanV3(content),
        machineResult,
    ];

    // Find highest confidence
    let best = results[0];
    for (const result of results) {
        if (result.confidence > best.confidence) {
            best = result;
        }
    }

    // If no clear winner, return unknown
    if (best.confidence < 0.3) {
        return { format: 'unknown', confidence: best.confidence, hints: ['No clear format detected'] };
    }

    return best;
}

/**
 * Check if format is a source format that should be converted to LLM
 */
export function isSourceFormat(format: DetectedFormat): boolean {
    return format === 'json' || format === 'yaml' || format === 'toml' || format === 'csv';
}

/**
 * Check if format is a DX format (LLM or Human)
 */
export function isDxFormat(format: DetectedFormat): boolean {
    return format === 'llm' || format === 'human-v3';
}
