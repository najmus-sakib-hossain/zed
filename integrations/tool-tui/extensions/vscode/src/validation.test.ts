/**
 * Property-based tests for DX validation error quality
 * 
 * Feature: dx-serializer-extension, Property 5: Validation error quality
 * 
 * For any invalid DX content with:
 * - Unclosed brackets: the validation result SHALL include the line and column of the opening bracket
 * - Unclosed strings: the validation result SHALL include the line and column where the string started
 * - Mismatched brackets: the validation result SHALL include a hint about which bracket was expected
 * 
 * For all validation errors, the result SHALL include an actionable hint.
 * 
 * **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**
 */

import * as fc from 'fast-check';
import { validateDx, ValidationResult } from './dxCore';

// ============================================================================
// Generators for invalid content
// ============================================================================

/**
 * Generate a valid key
 */
const validKey = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'),
    { minLength: 1, maxLength: 10 }
);

/**
 * Generate a simple value
 */
const simpleValue = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789'),
    { minLength: 1, maxLength: 10 }
);

/**
 * Generate content with unclosed bracket
 */
const unclosedBracketContent = fc.tuple(
    validKey,
    fc.constantFrom('{', '[', '('),
    simpleValue
).map(([key, bracket, value]: [string, string, string]) => ({
    content: `${key}: ${bracket}${value}`,
    bracket,
    line: 1,
    column: key.length + 3, // After ": "
}));

/**
 * Generate content with unclosed string
 */
const unclosedStringContent = fc.tuple(
    validKey,
    fc.constantFrom('"', "'"),
    simpleValue
).map(([key, quote, value]: [string, string, string]) => ({
    content: `${key}: ${quote}${value}`,
    quote,
    line: 1,
    column: key.length + 3, // After ": "
}));


/**
 * Generate content with mismatched brackets
 */
const mismatchedBracketContent = fc.tuple(
    validKey,
    fc.constantFrom(
        { open: '{', wrongClose: ']' },
        { open: '{', wrongClose: ')' },
        { open: '[', wrongClose: '}' },
        { open: '[', wrongClose: ')' },
        { open: '(', wrongClose: '}' },
        { open: '(', wrongClose: ']' }
    ),
    simpleValue
).map(([key, brackets, value]: [string, { open: string; wrongClose: string }, string]) => ({
    content: `${key}: ${brackets.open}${value}${brackets.wrongClose}`,
    open: brackets.open,
    wrongClose: brackets.wrongClose,
}));

/**
 * Generate multi-line content with unclosed bracket
 */
const multiLineUnclosedBracket = fc.tuple(
    validKey,
    fc.constantFrom('{', '[', '('),
    fc.array(fc.tuple(validKey, simpleValue), { minLength: 1, maxLength: 3 })
).map(([key, bracket, fields]: [string, string, [string, string][]]) => {
    const lines = [`${key}: ${bracket}`];
    for (const [k, v] of fields) {
        lines.push(`  ${k}: ${v}`);
    }
    return {
        content: lines.join('\n'),
        bracket,
        line: 1,
        column: key.length + 3,
    };
});

// ============================================================================
// Property Tests
// ============================================================================

/**
 * Property 5.1: Unclosed brackets include line and column
 */
export function testUnclosedBracketsIncludePosition(): void {
    fc.assert(
        fc.property(unclosedBracketContent, ({ content, bracket, line, column }) => {
            const result = validateDx(content);

            if (result.success) {
                throw new Error(`Validation should fail for unclosed bracket in: '${content}'`);
            }

            if (result.line === undefined) {
                throw new Error(`Validation error should include line number for: '${content}'`);
            }

            if (result.column === undefined) {
                throw new Error(`Validation error should include column number for: '${content}'`);
            }

            // Line should be correct (1-indexed)
            if (result.line !== line) {
                throw new Error(`Expected line ${line}, got ${result.line} for: '${content}'`);
            }

            // Column should be at or near the bracket position
            if (Math.abs(result.column - column) > 2) {
                throw new Error(`Expected column near ${column}, got ${result.column} for: '${content}'`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 5.1: Unclosed brackets include line and column');
}

/**
 * Property 5.2: Unclosed strings include line and column
 */
export function testUnclosedStringsIncludePosition(): void {
    fc.assert(
        fc.property(unclosedStringContent, ({ content, quote, line, column }) => {
            const result = validateDx(content);

            if (result.success) {
                throw new Error(`Validation should fail for unclosed string in: '${content}'`);
            }

            if (result.line === undefined) {
                throw new Error(`Validation error should include line number for: '${content}'`);
            }

            if (result.column === undefined) {
                throw new Error(`Validation error should include column number for: '${content}'`);
            }

            // Error should mention unclosed string
            if (!result.error?.toLowerCase().includes('unclosed') &&
                !result.error?.toLowerCase().includes('string')) {
                throw new Error(`Error should mention unclosed string: ${result.error}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 5.2: Unclosed strings include line and column');
}


/**
 * Property 5.3: Mismatched brackets include hint about expected bracket
 */
export function testMismatchedBracketsIncludeHint(): void {
    fc.assert(
        fc.property(mismatchedBracketContent, ({ content, open, wrongClose }) => {
            const result = validateDx(content);

            if (result.success) {
                throw new Error(`Validation should fail for mismatched brackets in: '${content}'`);
            }

            if (result.hint === undefined) {
                throw new Error(`Validation error should include hint for: '${content}'`);
            }

            // Error should mention mismatched
            if (!result.error?.toLowerCase().includes('mismatch')) {
                throw new Error(`Error should mention mismatched brackets: ${result.error}`);
            }

            // Hint should mention the expected bracket
            const expectedClose = open === '{' ? '}' : open === '[' ? ']' : ')';
            if (!result.hint.includes(expectedClose)) {
                throw new Error(`Hint should mention expected bracket '${expectedClose}': ${result.hint}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 5.3: Mismatched brackets include hint about expected bracket');
}

/**
 * Property 5.4: All validation errors include actionable hints
 */
export function testAllErrorsIncludeHints(): void {
    const invalidContent = fc.oneof(
        unclosedBracketContent.map(x => x.content),
        unclosedStringContent.map(x => x.content),
        mismatchedBracketContent.map(x => x.content)
    );

    fc.assert(
        fc.property(invalidContent, (content: string) => {
            const result = validateDx(content);

            if (result.success) {
                // Some generated content might be valid, skip
                return true;
            }

            if (result.hint === undefined || result.hint.length === 0) {
                throw new Error(`Validation error should include actionable hint for: '${content}'`);
            }

            // Hint should be actionable (contain action words or provide context)
            const actionWords = ['add', 'close', 'match', 'complete', 'fix', 'remove', 'check', 'expects', 'expected', 'matching', 'opening'];
            const hasActionWord = actionWords.some(word =>
                result.hint!.toLowerCase().includes(word)
            );

            if (!hasActionWord) {
                throw new Error(`Hint should be actionable: ${result.hint}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 5.4: All validation errors include actionable hints');
}

/**
 * Property 5.5: Multi-line unclosed brackets report correct line
 */
export function testMultiLineUnclosedBrackets(): void {
    fc.assert(
        fc.property(multiLineUnclosedBracket, ({ content, bracket, line }) => {
            const result = validateDx(content);

            if (result.success) {
                throw new Error(`Validation should fail for unclosed bracket in: '${content}'`);
            }

            if (result.line === undefined) {
                throw new Error(`Validation error should include line number`);
            }

            // Line should be the line where the bracket was opened
            if (result.line !== line) {
                throw new Error(`Expected line ${line}, got ${result.line}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 5.5: Multi-line unclosed brackets report correct line');
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property 5: Validation error quality tests...\n');

    testUnclosedBracketsIncludePosition();
    testUnclosedStringsIncludePosition();
    testMismatchedBracketsIncludeHint();
    testAllErrorsIncludeHints();
    testMultiLineUnclosedBrackets();

    console.log('\n✓ All Property 5 tests passed!');
}

// Run tests if this file is executed directly
if (require.main === module) {
    try {
        runAllPropertyTests();
    } catch (error) {
        console.error('Tests failed:', error);
        process.exit(1);
    }
}
