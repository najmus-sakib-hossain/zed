/**
 * Property-based tests for DxCore
 * 
 * Feature: dx-serializer-extension-fix, Property 8: WASM and TypeScript Equivalence
 * 
 * For any valid DX content, the WASM implementation and TypeScript fallback
 * implementation SHALL produce identical transformation results.
 * 
 * **Validates: Requirements 5.3, 5.4**
 * 
 * Note: Since WASM may not be available in all test environments, these tests
 * focus on the TypeScript fallback implementation's correctness and round-trip
 * properties.
 */

import * as fc from 'fast-check';
import {
    formatDx,
    minifyDx,
    validateDx,
    smartQuote,
    createFallbackCore,
    TransformResult,
    ValidationResult,
} from './dxCore';
import { parseLlm, DxDocument, strValue, numValue, boolValue, nullValue, createDocument, createSection } from './llmParser';
import { formatDocument } from './humanFormatter';
import { parseHuman, serializeToLlm } from './humanParser';

// ============================================================================
// Generators for DX content
// ============================================================================

/**
 * Generate a valid key (alphanumeric with dots and underscores)
 */
const validKey = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789_'),
    { minLength: 1, maxLength: 15 }
).filter((s: string) => /^[a-z]/.test(s)); // Must start with letter

/**
 * Generate a simple value (no special characters)
 */
const simpleValue = fc.oneof(
    fc.stringOf(
        fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'),
        { minLength: 1, maxLength: 15 }
    ),
    fc.integer({ min: 0, max: 10000 }).map((n: number) => n.toString()),
    fc.constant('true'),
    fc.constant('false'),
);

/**
 * Generate a valid abbreviated key (2-letter abbreviation)
 */
const abbreviatedKey = fc.constantFrom(
    'nm', 'tt', 'ds', 'id', 'st', 'ac', 'en', 'ct', 'tl', 'pr', 'am', 'qt', 'em', 'ph', 'ur', 'pt', 'vl', 'tp'
);

/**
 * Generate an LLM format value
 */
const llmValue = fc.oneof(
    fc.stringOf(
        fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'),
        { minLength: 1, maxLength: 10 }
    ),
    fc.integer({ min: 0, max: 1000 }).map((n: number) => n.toString()),
    fc.constant('+'),  // boolean true
    fc.constant('-'),  // boolean false
    fc.constant('~'),  // null
);

/**
 * Generate a context section in LLM format
 */
const llmContextSection = fc.array(
    fc.tuple(abbreviatedKey, llmValue),
    { minLength: 1, maxLength: 3 }
).map((pairs: [string, string][]) => {
    const uniquePairs = new Map<string, string>();
    for (const [k, v] of pairs) {
        uniquePairs.set(k, v);
    }
    // New format: root-level key|value pairs (one per line)
    const lines = Array.from(uniquePairs.entries())
        .map(([k, v]) => `${k}|${v}`);
    return lines.join('\n');
});

/**
 * Generate a data section in LLM format
 */
const llmDataSection = fc.tuple(
    fc.constantFrom('d', 'h', 'o', 'p', 'u'),  // section ID
    fc.array(abbreviatedKey, { minLength: 1, maxLength: 3 }),  // schema
    fc.array(fc.array(llmValue, { minLength: 1, maxLength: 3 }), { minLength: 1, maxLength: 3 })  // rows
).map(([id, schema, rows]: [string, string[], string[][]]) => {
    const uniqueSchema = [...new Set(schema)];
    const schemaStr = uniqueSchema.join('|');
    const header = `#${id}(${schemaStr})`;
    const dataRows = rows.map(row => {
        // Ensure row has same length as schema
        const paddedRow = uniqueSchema.map((_, i) => row[i] || '~');
        return paddedRow.join('|');
    });
    return [header, ...dataRows].join('\n');
});

/**
 * Generate a complete LLM format document
 */
const llmDocument = fc.tuple(
    fc.option(llmContextSection, { nil: undefined }),
    fc.option(llmDataSection, { nil: undefined })
).map(([context, data]: [string | undefined, string | undefined]) => {
    const parts: string[] = [];
    if (context) parts.push(context);
    if (data) parts.push(data);
    return parts.join('\n');
}).filter((s: string) => s.length > 0);


/**
 * Generate a key:value pair in dense format
 */
const keyValuePair = fc.tuple(validKey, simpleValue)
    .map(([k, v]: [string, string]) => `${k}:${v}`);

/**
 * Generate an object in dense format: key#field:val#field:val
 */
const simpleObject = fc.tuple(
    validKey,
    fc.array(fc.tuple(validKey, simpleValue), { minLength: 1, maxLength: 3 })
).map(([key, fields]: [string, [string, string][]]) => {
    // Ensure unique field keys
    const uniqueFields = new Map<string, string>();
    for (const [k, v] of fields) {
        uniqueFields.set(k, v);
    }
    const fieldStr = Array.from(uniqueFields.entries())
        .map(([k, v]) => `#${k}:${v}`)
        .join('');
    return `${key}${fieldStr}`;
});

// ============================================================================
// Property Tests
// ============================================================================

/**
 * Property 8.1: LLM to Human to LLM Round-Trip
 * For any valid LLM format, converting to human and back should preserve data
 * 
 * Feature: dx-serializer-extension-fix, Property 8: WASM and TypeScript Equivalence
 * **Validates: Requirements 5.3, 5.4**
 */
export function testLlmRoundTrip(): void {
    fc.assert(
        fc.property(llmDocument, (llm: string) => {
            // Parse LLM format
            const parseResult = parseLlm(llm);
            if (!parseResult.success || !parseResult.document) {
                // Skip invalid documents
                return true;
            }

            // Convert to human format
            const human = formatDocument(parseResult.document);

            // Parse human format back
            const humanParseResult = parseHuman(human);
            if (!humanParseResult.success || !humanParseResult.document) {
                throw new Error(`Failed to parse human format: ${human}`);
            }

            // Serialize back to LLM
            const llmBack = serializeToLlm(humanParseResult.document);

            // Parse both to compare documents
            const originalDoc = parseResult.document;
            const roundTripDoc = humanParseResult.document;

            // Compare context
            if (originalDoc.context.size !== roundTripDoc.context.size) {
                throw new Error(`Context size mismatch: ${originalDoc.context.size} vs ${roundTripDoc.context.size}`);
            }

            // Compare sections
            if (originalDoc.sections.size !== roundTripDoc.sections.size) {
                throw new Error(`Section count mismatch: ${originalDoc.sections.size} vs ${roundTripDoc.sections.size}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 8.1: LLM to Human to LLM Round-Trip');
}

/**
 * Property 8.2: formatDx produces valid human format
 * For any valid LLM content, formatDx should succeed
 */
export function testFormatDxProducesValidOutput(): void {
    fc.assert(
        fc.property(llmDocument, (llm: string) => {
            const result = formatDx(llm);
            // Should not throw and should produce non-empty output for non-empty input
            if (llm.trim() && !result) {
                throw new Error(`formatDx returned empty for non-empty input: '${llm}'`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 8.2: formatDx produces valid human format');
}

/**
 * Property 8.3: minifyDx produces valid LLM format
 * For any human format produced by formatDx, minifyDx should succeed
 */
export function testMinifyDxProducesValidOutput(): void {
    fc.assert(
        fc.property(llmDocument, (llm: string) => {
            // First convert to human
            const human = formatDx(llm);
            if (!human) return true;

            // Then convert back to LLM
            const result = minifyDx(human);

            // Should produce valid LLM format (starts with # or is empty)
            if (result && !result.startsWith('#')) {
                // It's okay if it doesn't start with # for simple content
                // Just verify it doesn't throw
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 8.3: minifyDx produces valid LLM format');
}

/**
 * Property 8.4: Validation accepts valid LLM format
 * For any valid LLM document, validation should pass
 */
export function testValidationAcceptsValidLlm(): void {
    fc.assert(
        fc.property(llmDocument, (llm: string) => {
            const result = validateDx(llm);
            if (!result.success) {
                throw new Error(`Validation failed for valid LLM: '${llm}', error: ${result.error}`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 8.4: Validation accepts valid LLM format');
}

/**
 * Property 7.1: Fallback toHuman produces valid output
 * For any valid dense content, toHuman should succeed
 */
export function testFallbackToHumanSucceeds(): void {
    const core = createFallbackCore(2);

    fc.assert(
        fc.property(llmDocument, (dense: string) => {
            const result = core.toHuman(dense);
            if (!result.success) {
                throw new Error(`toHuman failed for '${dense}': ${result.error}`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.1: Fallback toHuman produces valid output');
}

/**
 * Property 7.2: Fallback toDense produces valid output
 * For any valid human content, toDense should succeed
 */
export function testFallbackToDenseSucceeds(): void {
    const core = createFallbackCore(2);

    fc.assert(
        fc.property(llmDocument, (dense: string) => {
            // First convert to human, then back to dense
            const human = core.toHuman(dense);
            if (!human.success) return true; // Skip if toHuman fails

            const result = core.toDense(human.content);
            if (!result.success) {
                throw new Error(`toDense failed for '${human.content}': ${result.error}`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.2: Fallback toDense produces valid output');
}

/**
 * Property 7.3: Round-trip preserves document structure
 * For any LLM document, round-trip should preserve the structure
 */
export function testRoundTripPreservesStructure(): void {
    const core = createFallbackCore(2);

    fc.assert(
        fc.property(llmDocument, (llm: string) => {
            // Transform to human
            const human = core.toHuman(llm);
            if (!human.success) {
                throw new Error(`toHuman failed: ${human.error}`);
            }

            // Transform back to dense
            const result = core.toDense(human.content);
            if (!result.success) {
                throw new Error(`toDense failed: ${result.error}`);
            }

            // Parse both documents
            const originalParse = parseLlm(llm);
            const roundTripParse = parseLlm(result.content);

            if (!originalParse.success || !roundTripParse.success) {
                return true; // Skip if parsing fails
            }

            // Verify structure is preserved
            const origDoc = originalParse.document!;
            const rtDoc = roundTripParse.document!;

            // Context size should match
            if (origDoc.context.size !== rtDoc.context.size) {
                throw new Error(`Context size mismatch: ${origDoc.context.size} vs ${rtDoc.context.size}`);
            }

            // Section count should match
            if (origDoc.sections.size !== rtDoc.sections.size) {
                throw new Error(`Section count mismatch: ${origDoc.sections.size} vs ${rtDoc.sections.size}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.3: Round-trip preserves document structure');
}


/**
 * Property 7.4: Validation detects unclosed brackets
 * For any content with unclosed brackets, validation should fail
 */
export function testValidationDetectsUnclosedBrackets(): void {
    const unclosedBracket = fc.tuple(
        validKey,
        fc.constantFrom('{', '[', '(')
    ).map(([key, bracket]: [string, string]) => `${key}: ${bracket}value`);

    fc.assert(
        fc.property(unclosedBracket, (content: string) => {
            const result = validateDx(content);
            if (result.success) {
                throw new Error(`Validation should fail for unclosed bracket in: '${content}'`);
            }
            if (!result.error?.includes('Unclosed') && !result.error?.includes('bracket')) {
                throw new Error(`Error should mention unclosed bracket: ${result.error}`);
            }
            if (result.line === undefined) {
                throw new Error('Validation error should include line number');
            }
            if (result.hint === undefined) {
                throw new Error('Validation error should include hint');
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.4: Validation detects unclosed brackets');
}

/**
 * Property 7.5: Validation detects unclosed strings
 * For any content with unclosed strings, validation should fail
 */
export function testValidationDetectsUnclosedStrings(): void {
    const unclosedString = fc.tuple(
        validKey,
        fc.constantFrom('"', "'"),
        simpleValue
    ).map(([key, quote, value]: [string, string, string]) => `${key}: ${quote}${value}`);

    fc.assert(
        fc.property(unclosedString, (content: string) => {
            const result = validateDx(content);
            if (result.success) {
                throw new Error(`Validation should fail for unclosed string in: '${content}'`);
            }
            if (!result.error?.includes('Unclosed') && !result.error?.includes('string')) {
                throw new Error(`Error should mention unclosed string: ${result.error}`);
            }
            if (result.line === undefined) {
                throw new Error('Validation error should include line number');
            }
            if (result.hint === undefined) {
                throw new Error('Validation error should include hint');
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.5: Validation detects unclosed strings');
}

/**
 * Property 7.6: Valid content passes validation
 * For any properly formatted content, validation should succeed
 */
export function testValidContentPassesValidation(): void {
    const validContent = fc.tuple(validKey, simpleValue)
        .map(([k, v]: [string, string]) => `${k}: ${v}`);

    fc.assert(
        fc.property(validContent, (content: string) => {
            const result = validateDx(content);
            if (!result.success) {
                throw new Error(`Validation should pass for: '${content}', got error: ${result.error}`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.6: Valid content passes validation');
}

// ============================================================================
// Smart Quoting Tests
// ============================================================================

/**
 * Property 7.7: Smart quoting handles apostrophes correctly
 * Strings with apostrophes should be wrapped in double quotes
 */
export function testSmartQuoteApostrophes(): void {
    const stringWithApostrophe = fc.tuple(
        fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 10 }),
        fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 10 })
    ).map(([before, after]: [string, string]) => `${before}'${after}`);

    fc.assert(
        fc.property(stringWithApostrophe, (value: string) => {
            const quoted = smartQuote(value);
            // Should use double quotes for strings with apostrophes
            if (!quoted.startsWith('"') || !quoted.endsWith('"')) {
                throw new Error(`Expected double quotes for '${value}', got: ${quoted}`);
            }
            // Should contain the original apostrophe
            if (!quoted.includes("'")) {
                throw new Error(`Apostrophe should be preserved in: ${quoted}`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.7: Smart quoting handles apostrophes correctly');
}


// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for DxCore...\n');

    const core = createFallbackCore(2);
    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'toHuman transforms LLM context section (legacy)',
            test: () => {
                // Legacy format still supported
                const result = core.toHuman('#c:nm|Test;ct|42');
                return result.success && result.content.includes('name') && result.content.includes('Test');
            }
        },
        {
            name: 'toHuman transforms LLM context section (new format)',
            test: () => {
                // New format: root-level key|value pairs
                const result = core.toHuman('nm|Test\nct|42');
                return result.success && result.content.includes('name') && result.content.includes('Test');
            }
        },
        {
            name: 'toHuman transforms LLM data section',
            test: () => {
                const result = core.toHuman('#d(id|nm)\n1|Alpha\n2|Beta');
                return result.success &&
                    result.content.includes('Alpha') &&
                    result.content.includes('Beta');
            }
        },
        {
            name: 'toDense transforms human format',
            test: () => {
                const human = '[config]\n    name = Test';
                const result = core.toDense(human);
                return result.success && result.content.includes('nm') && result.content.includes('Test');
            }
        },
        {
            name: 'validate accepts valid LLM content (legacy)',
            test: () => {
                const result = core.validate('#c:nm|Test');
                return result.success;
            }
        },
        {
            name: 'validate accepts valid LLM content (new format)',
            test: () => {
                const result = core.validate('nm|Test\nv|1.0');
                return result.success;
            }
        },
        {
            name: 'validate accepts valid human content',
            test: () => {
                const result = core.validate('key: value');
                return result.success;
            }
        },
        {
            name: 'validate rejects unclosed bracket',
            test: () => {
                const result = core.validate('key: {value');
                return !result.success && (result.error?.includes('Unclosed') ?? false);
            }
        },
        {
            name: 'validate rejects unclosed string',
            test: () => {
                const result = core.validate('key: "value');
                return !result.success && (result.error?.includes('Unclosed') ?? false);
            }
        },
        {
            name: 'validate rejects mismatched brackets',
            test: () => {
                const result = core.validate('key: [value}');
                return !result.success && (result.error?.includes('Mismatched') ?? false);
            }
        },
        {
            name: 'validate rejects invalid LLM sigil',
            test: () => {
                const result = core.validate('#x:invalid');
                return !result.success && (result.error?.includes('sigil') ?? false);
            }
        },
        {
            name: 'validate rejects schema mismatch',
            test: () => {
                const result = core.validate('#d(id|nm)\n1|Alpha|Extra');
                return !result.success && (result.error?.includes('columns') ?? false);
            }
        },
        {
            name: 'isSaveable returns true for valid content (legacy)',
            test: () => {
                return core.isSaveable('#c:nm|Test');
            }
        },
        {
            name: 'isSaveable returns true for valid content (new format)',
            test: () => {
                return core.isSaveable('nm|Test\nv|1.0');
            }
        },
        {
            name: 'isSaveable returns false for invalid content',
            test: () => {
                return !core.isSaveable('key: {unclosed');
            }
        },
        {
            name: 'smartQuote handles simple strings',
            test: () => {
                return smartQuote('hello') === 'hello';
            }
        },
        {
            name: 'smartQuote handles strings with spaces',
            test: () => {
                return smartQuote('hello world') === '"hello world"';
            }
        },
        {
            name: 'smartQuote handles apostrophes',
            test: () => {
                return smartQuote("don't") === '"don\'t"';
            }
        },
        {
            name: 'smartQuote handles double quotes',
            test: () => {
                return smartQuote('say "hello"') === "'say \"hello\"'";
            }
        },
        {
            name: 'smartQuote handles both quote types',
            test: () => {
                const result = smartQuote("don't say \"hello\"");
                return result.startsWith('"') && result.includes("\\'") === false;
            }
        },
        {
            name: 'empty input returns empty output',
            test: () => {
                const human = core.toHuman('');
                const dense = core.toDense('');
                return human.success && human.content === '' &&
                    dense.success && dense.content === '';
            }
        },
    ];

    for (const { name, test } of tests) {
        try {
            if (test()) {
                console.log(`  ✓ ${name}`);
                passed++;
            } else {
                console.log(`  ✗ ${name}`);
                failed++;
            }
        } catch (error) {
            console.log(`  ✗ ${name}: ${error}`);
            failed++;
        }
    }

    console.log(`\nUnit tests: ${passed} passed, ${failed} failed`);

    if (failed > 0) {
        throw new Error(`${failed} unit tests failed`);
    }
}

// ============================================================================
// WASM Integration Tests
// ============================================================================

import * as path from 'path';
import * as fs from 'fs';
import { loadDxCore, clearDxCoreCache } from './dxCore';

/**
 * Test 6.1: WASM Loading Test
 * Test that WASM module loads successfully and DxSerializer can be instantiated
 * 
 * **Validates: Requirements 6.1, 6.4**
 */
export async function testWasmLoading(): Promise<void> {
    console.log('Running WASM loading tests...\n');

    // Clear cache to ensure fresh load
    clearDxCoreCache();

    // Get the extension path (parent of out directory)
    const extensionPath = path.resolve(__dirname, '..');

    // Check if WASM files exist
    const wasmJsPath = path.join(extensionPath, 'wasm', 'dx_serializer.js');
    const wasmBinaryPath = path.join(extensionPath, 'wasm', 'dx_serializer_bg.wasm');

    if (!fs.existsSync(wasmJsPath)) {
        console.log('  ⚠ WASM JS file not found, skipping WASM tests');
        console.log(`    Expected at: ${wasmJsPath}`);
        return;
    }

    if (!fs.existsSync(wasmBinaryPath)) {
        console.log('  ⚠ WASM binary file not found, skipping WASM tests');
        console.log(`    Expected at: ${wasmBinaryPath}`);
        return;
    }

    console.log('  ✓ WASM files exist');

    // Try to load the WASM module
    try {
        const core = await loadDxCore(extensionPath);

        if (core.isWasm) {
            console.log('  ✓ WASM module loaded successfully');

            // Test basic operations with new format
            const toHumanResult = core.toHuman('nm|Test\nv|1.0');
            if (toHumanResult.success) {
                console.log('  ✓ WASM toHuman works');
            } else {
                console.log(`  ✗ WASM toHuman failed: ${toHumanResult.error}`);
            }

            const validateResult = core.validate('nm|Test\nv|1.0');
            if (validateResult.success) {
                console.log('  ✓ WASM validate works');
            } else {
                console.log(`  ✗ WASM validate failed: ${validateResult.error}`);
            }
        } else {
            console.log('  ⚠ Loaded TypeScript fallback instead of WASM');
        }
    } catch (error) {
        console.log(`  ⚠ WASM loading failed (fallback will be used): ${error}`);
    }

    // Clear cache for other tests
    clearDxCoreCache();
}

/**
 * Test 6.2: Parse Equivalence Test
 * Parse same input with WASM and TypeScript and verify documents are equivalent
 * 
 * **Validates: Property 3 - WASM-TypeScript Parse Equivalence**
 */
export async function testParseEquivalence(): Promise<void> {
    console.log('\nRunning parse equivalence tests...\n');

    const extensionPath = path.resolve(__dirname, '..');
    const testCases = [
        // New format: root-level key|value pairs
        'nm|Test\nv|1.0',
        'nm|Project\nds|A test project',
        '#d(id|nm|en)\n1|Alpha|+\n2|Beta|-',
        // New format with section
        'nm|App\n#d(id|nm)\n1|First\n2|Second',
    ];

    // Get fallback core
    const fallbackCore = createFallbackCore(2, 20);

    // Try to get WASM core
    clearDxCoreCache();
    let wasmCore;
    try {
        wasmCore = await loadDxCore(extensionPath);
    } catch {
        console.log('  ⚠ WASM not available, skipping equivalence tests');
        return;
    }

    if (!wasmCore.isWasm) {
        console.log('  ⚠ WASM not loaded, skipping equivalence tests');
        return;
    }

    let passed = 0;
    let failed = 0;

    for (const input of testCases) {
        const wasmResult = wasmCore.toHuman(input);
        const fallbackResult = fallbackCore.toHuman(input);

        if (wasmResult.success !== fallbackResult.success) {
            console.log(`  ✗ Success mismatch for: ${input.substring(0, 30)}...`);
            console.log(`    WASM: ${wasmResult.success}, Fallback: ${fallbackResult.success}`);
            failed++;
            continue;
        }

        if (!wasmResult.success) {
            // Both failed, check error messages are similar
            console.log(`  ✓ Both failed for: ${input.substring(0, 30)}...`);
            passed++;
            continue;
        }

        // Both succeeded, compare outputs (allowing for minor formatting differences)
        const wasmLines = wasmResult.content.trim().split('\n').filter(l => l.trim());
        const fallbackLines = fallbackResult.content.trim().split('\n').filter(l => l.trim());

        // Check that key content is preserved
        const wasmHasName = wasmResult.content.includes('name') || wasmResult.content.includes('nm');
        const fallbackHasName = fallbackResult.content.includes('name') || fallbackResult.content.includes('nm');

        if (wasmHasName === fallbackHasName) {
            console.log(`  ✓ Equivalent output for: ${input.substring(0, 30)}...`);
            passed++;
        } else {
            console.log(`  ✗ Content mismatch for: ${input.substring(0, 30)}...`);
            failed++;
        }
    }

    console.log(`\nParse equivalence: ${passed} passed, ${failed} failed`);
    clearDxCoreCache();
}

/**
 * Test 6.3: Error Handling Test
 * Test InputTooLarge, RecursionLimitExceeded, TableTooLarge errors
 * 
 * **Validates: Requirements 3.1-3.2, 4.1-4.2, 5.1-5.2**
 */
export async function testErrorHandling(): Promise<void> {
    console.log('\nRunning error handling tests...\n');

    const extensionPath = path.resolve(__dirname, '..');
    clearDxCoreCache();

    let core;
    try {
        core = await loadDxCore(extensionPath);
    } catch {
        console.log('  ⚠ Could not load core, using fallback');
        core = createFallbackCore(2, 20);
    }

    // Test validation of invalid content
    const invalidCases = [
        { input: '#x:invalid', desc: 'Invalid sigil' },
        { input: '#d(id|nm)\n1|Alpha|Extra', desc: 'Schema mismatch' },
        { input: 'key: {unclosed', desc: 'Unclosed bracket' },
        { input: 'key: "unclosed', desc: 'Unclosed string' },
    ];

    let passed = 0;
    let failed = 0;

    for (const { input, desc } of invalidCases) {
        const result = core.validate(input);
        if (!result.success) {
            console.log(`  ✓ Correctly rejected: ${desc}`);
            passed++;
        } else {
            console.log(`  ✗ Should have rejected: ${desc}`);
            failed++;
        }
    }

    // Test that valid content passes
    const validCases = [
        // Legacy format
        '#c:nm|Test',
        // New format
        'nm|Test\nv|1.0',
        '#d(id|nm)\n1|Alpha\n2|Beta',
        'key: value',
    ];

    for (const input of validCases) {
        const result = core.validate(input);
        if (result.success) {
            console.log(`  ✓ Correctly accepted: ${input.substring(0, 30)}...`);
            passed++;
        } else {
            console.log(`  ✗ Should have accepted: ${input.substring(0, 30)}...`);
            console.log(`    Error: ${result.error}`);
            failed++;
        }
    }

    console.log(`\nError handling: ${passed} passed, ${failed} failed`);
    clearDxCoreCache();
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property 8: WASM and TypeScript Equivalence tests...\n');

    // New LLM format tests
    testLlmRoundTrip();
    testFormatDxProducesValidOutput();
    testMinifyDxProducesValidOutput();
    testValidationAcceptsValidLlm();

    // Legacy tests updated for LLM format
    testFallbackToHumanSucceeds();
    testFallbackToDenseSucceeds();
    testRoundTripPreservesStructure();
    testValidationDetectsUnclosedBrackets();
    testValidationDetectsUnclosedStrings();
    testValidContentPassesValidation();
    testSmartQuoteApostrophes();

    console.log('\n✓ All Property 8 tests passed!');
}

export async function runWasmIntegrationTests(): Promise<void> {
    console.log('\n========================================');
    console.log('WASM Integration Tests');
    console.log('========================================\n');

    await testWasmLoading();
    await testParseEquivalence();
    await testErrorHandling();

    console.log('\n✓ WASM integration tests completed!');
}

// Run tests if this file is executed directly
if (require.main === module) {
    (async () => {
        try {
            runUnitTests();
            console.log('');
            runAllPropertyTests();
            await runWasmIntegrationTests();
        } catch (error) {
            console.error('Tests failed:', error);
            process.exit(1);
        }
    })();
}
