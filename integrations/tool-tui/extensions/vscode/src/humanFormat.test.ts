/**
 * Property-based tests for Human Format Structure
 * 
 * Feature: dx-serializer-extension, Property 2: Human format structure
 * 
 * For any valid DX content, the human format output SHALL:
 * - Contain proper indentation (each nested level indented by the configured indent size)
 * - Have colons followed by a space for key-value pairs
 * - Have newlines after opening brackets and commas
 * 
 * For any valid DX content, the dense format output SHALL:
 * - Contain no whitespace outside of string literals
 * - Contain no comments
 * 
 * **Validates: Requirements 1.3, 1.4, 1.5**
 */

import * as fc from 'fast-check';
import { formatDx, minifyDx, createFallbackCore } from './dxCore';

// ============================================================================
// Generators for DX content
// ============================================================================

/**
 * Generate a valid key (alphanumeric with underscores, starting with letter)
 */
const validKey = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789_'),
    { minLength: 1, maxLength: 12 }
).filter((s: string) => /^[a-z]/.test(s));

/**
 * Generate a simple value (no special characters that need quoting)
 */
const simpleValue = fc.oneof(
    fc.stringOf(
        fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'),
        { minLength: 1, maxLength: 12 }
    ),
    fc.integer({ min: 0, max: 10000 }).map((n: number) => n.toString()),
    fc.constant('true'),
    fc.constant('false'),
);

/**
 * Generate a key:value pair in dense format
 */
const keyValuePair = fc.tuple(validKey, simpleValue)
    .map(([k, v]: [string, string]) => `${k}:${v}`);

/**
 * Generate an object in dense format: key#field:val#field:val
 */
const denseObject = fc.tuple(
    validKey,
    fc.array(fc.tuple(validKey, simpleValue), { minLength: 1, maxLength: 4 })
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
// Property Tests for Human Format Structure
// ============================================================================

/**
 * Property 2.1: Human format has proper indentation
 * For any nested content, each level should be indented by the configured indent size
 */
export function testHumanFormatIndentation(): void {
    const indentSizes = [2, 4];

    for (const indentSize of indentSizes) {
        const indent = ' '.repeat(indentSize);

        fc.assert(
            fc.property(denseObject, (dense: string) => {
                const human = formatDx(dense, indentSize);
                const lines = human.split('\n');

                // First line should not be indented (root key)
                if (lines.length > 0 && lines[0].startsWith(' ')) {
                    throw new Error(`First line should not be indented: '${lines[0]}'`);
                }

                // Nested lines should be indented
                for (let i = 1; i < lines.length; i++) {
                    const line = lines[i];
                    if (line.trim() === '') continue;

                    // Check that nested lines start with the correct indent
                    if (!line.startsWith(indent)) {
                        throw new Error(
                            `Line ${i + 1} should be indented with ${indentSize} spaces: '${line}'`
                        );
                    }
                }

                return true;
            }),
            { numRuns: 100 }
        );
    }
    console.log('✓ Property 2.1: Human format has proper indentation');
}


/**
 * Property 2.2: Human format has colons followed by space
 * For any key-value pair, the colon should be followed by a space
 */
export function testHumanFormatColonSpacing(): void {
    fc.assert(
        fc.property(keyValuePair, (dense: string) => {
            const human = formatDx(dense, 2);
            const lines = human.split('\n');

            for (const line of lines) {
                const trimmed = line.trim();
                if (trimmed === '') continue;

                // Skip array items (start with -)
                if (trimmed.startsWith('-')) continue;

                // Find colon in the line (not inside quotes)
                const colonMatch = trimmed.match(/^([^:]+):\s*(.*)$/);
                if (colonMatch) {
                    const afterColon = colonMatch[2];
                    // If there's content after colon, there should be a space
                    if (afterColon && !trimmed.includes(': ')) {
                        throw new Error(
                            `Colon should be followed by space in: '${trimmed}'`
                        );
                    }
                }
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 2.2: Human format has colons followed by space');
}

/**
 * Property 2.3: Human format has newlines for nested content
 * For any object with nested fields, each field should be on its own line
 */
export function testHumanFormatNewlines(): void {
    fc.assert(
        fc.property(denseObject, (dense: string) => {
            const human = formatDx(dense, 2);
            const lines = human.split('\n');

            // Objects with multiple fields should have multiple lines
            const fieldCount = (dense.match(/#/g) || []).length;
            if (fieldCount > 0) {
                // Should have at least fieldCount + 1 lines (root + fields)
                if (lines.length < fieldCount + 1) {
                    throw new Error(
                        `Expected at least ${fieldCount + 1} lines for ${fieldCount} fields, got ${lines.length}`
                    );
                }
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 2.3: Human format has newlines for nested content');
}

// ============================================================================
// Property Tests for Dense Format Structure
// ============================================================================

/**
 * Property 2.4: Dense format has no unnecessary whitespace
 * For any content, the dense format should not contain whitespace outside strings
 */
export function testDenseFormatNoWhitespace(): void {
    // Generate human format content
    const humanContent = fc.tuple(validKey, simpleValue)
        .map(([k, v]: [string, string]) => `${k}: ${v}`);

    fc.assert(
        fc.property(humanContent, (human: string) => {
            const dense = minifyDx(human);

            // Check for whitespace outside of string literals
            let inString = false;
            let stringChar = '';

            for (let i = 0; i < dense.length; i++) {
                const ch = dense[i];

                // Handle string boundaries
                if (!inString && (ch === '"' || ch === "'")) {
                    inString = true;
                    stringChar = ch;
                    continue;
                }

                if (inString && ch === stringChar) {
                    // Check for escape
                    if (i > 0 && dense[i - 1] !== '\\') {
                        inString = false;
                    }
                    continue;
                }

                // Check for whitespace outside strings
                if (!inString && /\s/.test(ch)) {
                    throw new Error(
                        `Dense format should not contain whitespace outside strings: '${dense}' at position ${i}`
                    );
                }
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 2.4: Dense format has no unnecessary whitespace');
}

/**
 * Property 2.5: Dense format has no comments
 * For any content with comments, the dense format should strip them
 */
export function testDenseFormatNoComments(): void {
    // Generate human format content with comments
    const humanWithComment = fc.tuple(validKey, simpleValue)
        .map(([k, v]: [string, string]) => `// This is a comment\n${k}: ${v}`);

    fc.assert(
        fc.property(humanWithComment, (human: string) => {
            const dense = minifyDx(human);

            // Dense format should not contain comment markers
            if (dense.includes('//')) {
                throw new Error(
                    `Dense format should not contain line comments: '${dense}'`
                );
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 2.5: Dense format has no comments');
}


// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Human Format Structure...\n');

    const core = createFallbackCore(2);
    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'Human format indents nested fields with 2 spaces',
            test: () => {
                const human = formatDx('server#host:localhost#port:5432', 2);
                const lines = human.split('\n');
                return lines.length >= 2 &&
                    !lines[0].startsWith(' ') &&
                    lines[1].startsWith('  ');
            }
        },
        {
            name: 'Human format indents nested fields with 4 spaces',
            test: () => {
                const human = formatDx('server#host:localhost#port:5432', 4);
                const lines = human.split('\n');
                return lines.length >= 2 &&
                    !lines[0].startsWith(' ') &&
                    lines[1].startsWith('    ');
            }
        },
        {
            name: 'Human format has colon-space for key-value',
            test: () => {
                const human = formatDx('name:John', 2);
                return human.includes(': ');
            }
        },
        {
            name: 'Human format puts each field on new line',
            test: () => {
                const human = formatDx('server#host:localhost#port:5432', 2);
                const lines = human.split('\n').filter(l => l.trim());
                return lines.length === 3; // server:, host:, port:
            }
        },
        {
            name: 'Dense format removes whitespace',
            test: () => {
                const dense = minifyDx('name: John');
                return !dense.includes(' ');
            }
        },
        {
            name: 'Dense format removes line comments',
            test: () => {
                const dense = minifyDx('// comment\nname: John');
                return !dense.includes('//');
            }
        },
        {
            name: 'Dense format preserves string content',
            test: () => {
                const dense = minifyDx('name: "John Doe"');
                return dense.includes('John Doe');
            }
        },
        {
            name: 'Empty input produces empty output',
            test: () => {
                const human = formatDx('', 2);
                const dense = minifyDx('');
                return human === '' && dense === '';
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
    console.log('Running Property 2: Human Format Structure tests...\n');

    testHumanFormatIndentation();
    testHumanFormatColonSpacing();
    testHumanFormatNewlines();
    testDenseFormatNoWhitespace();
    testDenseFormatNoComments();

    console.log('\n✓ All Property 2 tests passed!');
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
