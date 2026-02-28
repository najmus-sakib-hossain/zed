/**
 * Property-based tests for Smart Quoting
 * 
 * Feature: dx-serializer-extension, Property 4: Smart quoting correctness
 * 
 * For any string containing an apostrophe (single quote), the human format
 * output SHALL wrap the string in double quotes.
 * 
 * For any string containing both single and double quotes, the output SHALL
 * use double quote delimiters with escaped internal double quotes.
 * 
 * **Validates: Requirements 2.3, 2.4**
 */

import * as fc from 'fast-check';
import { smartQuote, formatDx, minifyDx, createFallbackCore } from './dxCore';

// ============================================================================
// Generators for strings with various quote patterns
// ============================================================================

/**
 * Generate a string containing an apostrophe
 */
const stringWithApostrophe = fc.tuple(
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz '), { minLength: 1, maxLength: 10 }),
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz '), { minLength: 1, maxLength: 10 })
).map(([before, after]: [string, string]) => `${before}'${after}`);

/**
 * Generate a string containing a double quote
 */
const stringWithDoubleQuote = fc.tuple(
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz '), { minLength: 1, maxLength: 10 }),
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz '), { minLength: 1, maxLength: 10 })
).map(([before, after]: [string, string]) => `${before}"${after}`);

/**
 * Generate a string containing both single and double quotes
 */
const stringWithBothQuotes = fc.tuple(
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 5 }),
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 5 }),
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 5 })
).map(([a, b, c]: [string, string, string]) => `${a}'${b}"${c}`);

/**
 * Generate a simple string without quotes
 */
const simpleString = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'),
    { minLength: 1, maxLength: 15 }
);

/**
 * Generate a string with spaces (needs quoting)
 */
const stringWithSpaces = fc.tuple(
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 8 }),
    fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 8 })
).map(([a, b]: [string, string]) => `${a} ${b}`);

// ============================================================================
// Property Tests for Smart Quoting
// ============================================================================

/**
 * Property 4.1: Strings with apostrophes use double quotes
 * For any string containing an apostrophe, smartQuote should wrap in double quotes
 */
export function testApostropheUsesDoubleQuotes(): void {
    fc.assert(
        fc.property(stringWithApostrophe, (value: string) => {
            const quoted = smartQuote(value);

            // Should use double quotes
            if (!quoted.startsWith('"') || !quoted.endsWith('"')) {
                throw new Error(
                    `String with apostrophe should use double quotes: '${value}' -> ${quoted}`
                );
            }

            // Should preserve the apostrophe
            if (!quoted.includes("'")) {
                throw new Error(
                    `Apostrophe should be preserved: '${value}' -> ${quoted}`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4.1: Strings with apostrophes use double quotes');
}


/**
 * Property 4.2: Strings with double quotes use single quotes
 * For any string containing only double quotes (no apostrophes), 
 * smartQuote should wrap in single quotes
 */
export function testDoubleQuoteUsesSingleQuotes(): void {
    fc.assert(
        fc.property(stringWithDoubleQuote, (value: string) => {
            const quoted = smartQuote(value);

            // Should use single quotes
            if (!quoted.startsWith("'") || !quoted.endsWith("'")) {
                throw new Error(
                    `String with double quote should use single quotes: '${value}' -> ${quoted}`
                );
            }

            // Should preserve the double quote
            if (!quoted.includes('"')) {
                throw new Error(
                    `Double quote should be preserved: '${value}' -> ${quoted}`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4.2: Strings with double quotes use single quotes');
}

/**
 * Property 4.3: Strings with both quote types use double quotes with escaping
 * For any string containing both single and double quotes,
 * smartQuote should use double quotes and escape internal double quotes
 */
export function testBothQuotesUsesDoubleWithEscaping(): void {
    fc.assert(
        fc.property(stringWithBothQuotes, (value: string) => {
            const quoted = smartQuote(value);

            // Should use double quotes as delimiters
            if (!quoted.startsWith('"') || !quoted.endsWith('"')) {
                throw new Error(
                    `String with both quotes should use double quote delimiters: '${value}' -> ${quoted}`
                );
            }

            // Should escape internal double quotes
            // The original double quote should be escaped as \"
            if (!quoted.includes('\\"')) {
                throw new Error(
                    `Internal double quotes should be escaped: '${value}' -> ${quoted}`
                );
            }

            // Should preserve the apostrophe (unescaped)
            if (!quoted.includes("'")) {
                throw new Error(
                    `Apostrophe should be preserved: '${value}' -> ${quoted}`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4.3: Strings with both quote types use double quotes with escaping');
}

/**
 * Property 4.4: Simple strings without special chars are not quoted
 * For any simple alphanumeric string, smartQuote should return it unchanged
 */
export function testSimpleStringsNotQuoted(): void {
    fc.assert(
        fc.property(simpleString, (value: string) => {
            const quoted = smartQuote(value);

            // Simple strings should not be quoted
            if (quoted !== value) {
                throw new Error(
                    `Simple string should not be quoted: '${value}' -> ${quoted}`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4.4: Simple strings without special chars are not quoted');
}

/**
 * Property 4.5: Strings with spaces are quoted
 * For any string containing spaces, smartQuote should add quotes
 */
export function testStringsWithSpacesAreQuoted(): void {
    fc.assert(
        fc.property(stringWithSpaces, (value: string) => {
            const quoted = smartQuote(value);

            // Should be quoted (either single or double)
            const isQuoted = (quoted.startsWith('"') && quoted.endsWith('"')) ||
                (quoted.startsWith("'") && quoted.endsWith("'"));

            if (!isQuoted) {
                throw new Error(
                    `String with spaces should be quoted: '${value}' -> ${quoted}`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4.5: Strings with spaces are quoted');
}

/**
 * Property 4.6: Quoted strings preserve original content
 * For any string, the content inside quotes should match the original
 */
export function testQuotedStringsPreserveContent(): void {
    fc.assert(
        fc.property(stringWithApostrophe, (value: string) => {
            const quoted = smartQuote(value);

            // Extract content from quotes
            const content = quoted.slice(1, -1);

            // Content should match original (apostrophe preserved)
            if (content !== value) {
                throw new Error(
                    `Quoted content should match original: '${value}' -> content: '${content}'`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4.6: Quoted strings preserve original content');
}


// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Smart Quoting...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'Simple string without quotes returns unchanged',
            test: () => smartQuote('hello') === 'hello'
        },
        {
            name: 'String with space uses double quotes',
            test: () => smartQuote('hello world') === '"hello world"'
        },
        {
            name: 'String with apostrophe uses double quotes',
            test: () => {
                const result = smartQuote("don't");
                return result === '"don\'t"';
            }
        },
        {
            name: 'String with double quote uses single quotes',
            test: () => {
                const result = smartQuote('say "hello"');
                return result === "'say \"hello\"'";
            }
        },
        {
            name: 'String with both quotes uses double quotes with escaping',
            test: () => {
                const result = smartQuote("don't say \"hello\"");
                return result.startsWith('"') &&
                    result.endsWith('"') &&
                    result.includes('\\"') &&
                    result.includes("'");
            }
        },
        {
            name: 'String with hash uses quotes',
            test: () => {
                const result = smartQuote('test#value');
                return result.startsWith('"') && result.endsWith('"');
            }
        },
        {
            name: 'String with colon uses quotes',
            test: () => {
                const result = smartQuote('key:value');
                return result.startsWith('"') && result.endsWith('"');
            }
        },
        {
            name: 'String with pipe uses quotes',
            test: () => {
                const result = smartQuote('a|b');
                return result.startsWith('"') && result.endsWith('"');
            }
        },
        {
            name: 'Empty string returns empty (no special chars)',
            test: () => smartQuote('') === ''
        },
        {
            name: 'Contraction "it\'s" uses double quotes',
            test: () => {
                const result = smartQuote("it's");
                return result === '"it\'s"';
            }
        },
        {
            name: 'Possessive "John\'s" uses double quotes',
            test: () => {
                const result = smartQuote("John's");
                return result === '"John\'s"';
            }
        },
        {
            name: 'Multiple apostrophes preserved',
            test: () => {
                const result = smartQuote("don't won't can't");
                return result.startsWith('"') &&
                    result.includes("don't") &&
                    result.includes("won't") &&
                    result.includes("can't");
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
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property 4: Smart Quoting Correctness tests...\n');

    testApostropheUsesDoubleQuotes();
    testDoubleQuoteUsesSingleQuotes();
    testBothQuotesUsesDoubleWithEscaping();
    testSimpleStringsNotQuoted();
    testStringsWithSpacesAreQuoted();
    testQuotedStringsPreserveContent();

    console.log('\n✓ All Property 4 tests passed!');
}

// Run tests if this file is executed directly
if (require.main === module) {
    try {
        runUnitTests();
        console.log('');
        runAllPropertyTests();
    } catch (error) {
        console.error('Tests failed:', error);
        process.exit(1);
    }
}
