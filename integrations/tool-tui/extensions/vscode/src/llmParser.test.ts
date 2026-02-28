/**
 * Property-based tests for LLM Parser
 * 
 * Tests both the new dx-serializer format and legacy pipe-separated format
 * 
 * New Format (dx-serializer):
 * - Objects: name[key=value key2=value2]
 * - Tabular: name(col1 col2)[row1 row2]
 * - Arrays: items=val1 val2 val3
 * 
 * Legacy Format:
 * - Context: key|value or #c:key|value
 * - Refs: #:key|value
 * - Sections: #<letter>(col1|col2)
 * 
 * **Validates: Requirements 2.0**
 */

import * as fc from 'fast-check';
import {
    parseLlm,
    parseValue,
    parseContextSection,
    parseRefDefinition,
    parseDataSectionHeader,
    parseDataRow,
    DxValue,
    DxDocument,
    strValue,
    numValue,
    boolValue,
    nullValue,
    arrValue,
    createDocument,
    createSection,
} from './llmParser';

// ============================================================================
// Generators
// ============================================================================

const validKey = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789_'),
    { minLength: 1, maxLength: 10 }
).filter((s: string) => /^[a-z]/.test(s));

const simpleString = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'),
    { minLength: 2, maxLength: 15 }
).filter((s: string) => /^[a-zA-Z]/.test(s));

const numberValue = fc.oneof(
    fc.integer({ min: -10000, max: 10000 }),
    fc.float({ min: -1000, max: 1000, noNaN: true, noDefaultInfinity: true })
        .map((n: number) => Math.round(n * 100) / 100)
);

// ============================================================================
// New Format Tests
// ============================================================================

/**
 * Test 2.1: Parse object with key=value pairs
 */
export function testParseNewFormatObject(): void {
    const input = 'config[host=localhost port=8080 debug=true]';
    const result = parseLlm(input);

    if (!result.success) {
        throw new Error(`Parse failed: ${result.error?.message}`);
    }

    const doc = result.document!;
    if (!doc.context.has('config')) {
        throw new Error('Missing config object');
    }

    const config = doc.context.get('config')!;
    if (config.type !== 'object') {
        throw new Error(`Expected object type, got ${config.type}`);
    }

    const obj = config.value as Map<string, DxValue>;
    if (obj.get('host')?.value !== 'localhost') {
        throw new Error('Missing or incorrect host value');
    }
    if (obj.get('port')?.value !== 8080) {
        throw new Error('Missing or incorrect port value');
    }
    if (obj.get('debug')?.value !== true) {
        throw new Error('Missing or incorrect debug value');
    }

    console.log('✓ Test 2.1: Parse object with key=value pairs');
}

/**
 * Test 2.2: Parse tabular array with space-separated schema
 */
export function testParseNewFormatTabular(): void {
    const input = `users(id name active)[
1 Alice true
2 Bob false]`;

    const result = parseLlm(input);

    if (!result.success) {
        throw new Error(`Parse failed: ${result.error?.message}`);
    }

    const doc = result.document!;
    if (!doc.sections.has('users')) {
        throw new Error('Missing users section');
    }

    const section = doc.sections.get('users')!;
    if (section.schema.length !== 3) {
        throw new Error(`Expected 3 schema fields, got ${section.schema.length}`);
    }
    if (section.rows.length !== 2) {
        throw new Error(`Expected 2 rows, got ${section.rows.length}`);
    }

    const row1 = section.rows[0];
    if (row1[0].value !== 1) {
        throw new Error(`Expected id=1, got ${row1[0].value}`);
    }
    if (row1[1].value !== 'Alice') {
        throw new Error(`Expected name=Alice, got ${row1[1].value}`);
    }
    if (row1[2].value !== true) {
        throw new Error(`Expected active=true, got ${row1[2].value}`);
    }

    console.log('✓ Test 2.2: Parse tabular array with space-separated schema');
}

/**
 * Test 2.3: Parse simple array assignment
 */
export function testParseNewFormatArray(): void {
    const input = 'colors=red green blue';
    const result = parseLlm(input);

    if (!result.success) {
        throw new Error(`Parse failed: ${result.error?.message}`);
    }

    const doc = result.document!;
    if (!doc.context.has('colors')) {
        throw new Error('Missing colors array');
    }

    const colors = doc.context.get('colors')!;
    if (colors.type !== 'array') {
        throw new Error(`Expected array type, got ${colors.type}`);
    }

    const arr = colors.value as DxValue[];
    if (arr.length !== 3) {
        throw new Error(`Expected 3 items, got ${arr.length}`);
    }

    console.log('✓ Test 2.3: Parse simple array assignment');
}

/**
 * Test 2.4: Parse quoted strings with spaces
 */
export function testParseNewFormatQuotedStrings(): void {
    const input = 'context[task="Our favorite hikes together" location=Boulder]';
    const result = parseLlm(input);

    if (!result.success) {
        throw new Error(`Parse failed: ${result.error?.message}`);
    }

    const doc = result.document!;
    const context = doc.context.get('context')!;
    const obj = context.value as Map<string, DxValue>;

    if (obj.get('task')?.value !== 'Our favorite hikes together') {
        throw new Error('Quoted string not parsed correctly');
    }

    console.log('✓ Test 2.4: Parse quoted strings with spaces');
}

/**
 * Test 2.5: Parse nested schema in tabular arrays
 */
export function testParseNewFormatNestedSchema(): void {
    const input = `logs(timestamp level error(message retryable))[
"2025-01-15T10:00:00Z" info
"2025-01-15T10:01:00Z" error ("Database timeout" true)]`;

    const result = parseLlm(input);

    if (!result.success) {
        throw new Error(`Parse failed: ${result.error?.message}`);
    }

    const doc = result.document!;
    const section = doc.sections.get('logs')!;

    const errorField = section.schema.find(f => f.name === 'error');
    if (!errorField || !errorField.nested) {
        throw new Error('Missing nested error schema');
    }
    if (errorField.nested.length !== 2) {
        throw new Error(`Expected 2 nested fields, got ${errorField.nested.length}`);
    }

    console.log('✓ Test 2.5: Parse nested schema in tabular arrays');
}

// ============================================================================
// Legacy Format Tests (Backward Compatibility)
// ============================================================================

export function testParseValueBooleans(): void {
    const trueResult = parseValue('true');
    if (trueResult.type !== 'bool' || trueResult.value !== true) {
        throw new Error(`Expected bool true, got ${JSON.stringify(trueResult)}`);
    }

    const legacyTrue = parseValue('+');
    if (legacyTrue.type !== 'bool' || legacyTrue.value !== true) {
        throw new Error(`Expected bool true for '+', got ${JSON.stringify(legacyTrue)}`);
    }

    const falseResult = parseValue('false');
    if (falseResult.type !== 'bool' || falseResult.value !== false) {
        throw new Error(`Expected bool false, got ${JSON.stringify(falseResult)}`);
    }

    const legacyFalse = parseValue('-');
    if (legacyFalse.type !== 'bool' || legacyFalse.value !== false) {
        throw new Error(`Expected bool false for '-', got ${JSON.stringify(legacyFalse)}`);
    }

    console.log('✓ Property 1.1: parseValue correctly parses boolean values');
}

export function testParseValueNull(): void {
    const result = parseValue('~');
    if (result.type !== 'null' || result.value !== null) {
        throw new Error(`Expected null, got ${JSON.stringify(result)}`);
    }

    console.log('✓ Property 1.2: parseValue correctly parses null values');
}

export function testParseValueNumbers(): void {
    fc.assert(
        fc.property(numberValue, (num: number) => {
            const result = parseValue(num.toString());
            if (result.type !== 'number') {
                throw new Error(`Expected number type, got ${result.type}`);
            }
            if (result.value !== num) {
                throw new Error(`Expected ${num}, got ${result.value}`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 1.3: parseValue correctly parses numbers');
}

export function testParseValueStrings(): void {
    fc.assert(
        fc.property(simpleString, (str: string) => {
            const result = parseValue(str);
            if (result.type !== 'string') {
                throw new Error(`Expected string type, got ${result.type}`);
            }
            if (result.value !== str) {
                throw new Error(`Expected '${str}', got '${result.value}'`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 1.4: parseValue correctly parses strings');
}

export function testParseValueReferences(): void {
    fc.assert(
        fc.property(validKey, (key: string) => {
            const result = parseValue(`^${key}`);
            if (result.type !== 'string') {
                throw new Error(`Expected string type, got ${result.type}`);
            }
            if (result.value !== `^${key}`) {
                throw new Error(`Expected '^${key}', got '${result.value}'`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 1.5: parseValue correctly parses references');
}

export function testParseValueArrays(): void {
    fc.assert(
        fc.property(
            fc.array(simpleString, { minLength: 1, maxLength: 5 }),
            (items: string[]) => {
                const arrayStr = '*' + items.join(',');
                const result = parseValue(arrayStr);
                if (result.type !== 'array') {
                    throw new Error(`Expected array type, got ${result.type}`);
                }
                const arr = result.value as DxValue[];
                if (arr.length !== items.length) {
                    throw new Error(`Expected ${items.length} items, got ${arr.length}`);
                }
                return true;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 1.6: parseValue correctly parses arrays');
}

export function testParseContextSection(): void {
    fc.assert(
        fc.property(
            fc.array(fc.tuple(validKey, simpleString), { minLength: 1, maxLength: 5 }),
            (pairs: [string, string][]) => {
                const uniquePairs = new Map<string, string>();
                for (const [k, v] of pairs) {
                    uniquePairs.set(k, v);
                }

                const content = Array.from(uniquePairs.entries())
                    .map(([k, v]) => `${k}|${v}`)
                    .join(';');

                const result = parseContextSection(content);

                for (const [key, value] of uniquePairs) {
                    if (!result.has(key)) {
                        throw new Error(`Missing key '${key}' in result`);
                    }
                    const parsed = result.get(key)!;
                    if (parsed.value !== value) {
                        throw new Error(`Expected '${value}' for key '${key}', got '${parsed.value}'`);
                    }
                }
                return true;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 1.7: parseContextSection parses key-value pairs');
}

export function testParseDataSectionHeader(): void {
    fc.assert(
        fc.property(
            fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'),
            fc.array(validKey, { minLength: 1, maxLength: 5 }),
            (letter: string, schema: string[]) => {
                const header = `#${letter}(${schema.join('|')})`;
                const result = parseDataSectionHeader(header);

                if (!result) {
                    throw new Error(`Failed to parse header: ${header}`);
                }

                const [sectionId, parsedSchema] = result;
                if (sectionId !== letter) {
                    throw new Error(`Expected section ID '${letter}', got '${sectionId}'`);
                }
                if (parsedSchema.length !== schema.length) {
                    throw new Error(`Expected ${schema.length} columns, got ${parsedSchema.length}`);
                }
                return true;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 1.8: parseDataSectionHeader parses section headers');
}

export function testParseLlmLegacyComplete(): void {
    const input = `nm|Test
ct|42
#:A|CommonValue
#d(id|nm|ac)
1|Alice|+
2|Bob|-`;

    const result = parseLlm(input);

    if (!result.success) {
        throw new Error(`Parse failed: ${result.error?.message}`);
    }

    const doc = result.document!;

    if (!doc.context.has('nm')) {
        throw new Error('Missing context key "nm"');
    }
    if (!doc.context.has('ct')) {
        throw new Error('Missing context key "ct"');
    }

    if (!(doc as any).refs?.has('A')) {
        throw new Error('Missing ref "A"');
    }

    if (!doc.sections.has('d')) {
        throw new Error('Missing section "d"');
    }

    const section = doc.sections.get('d')!;
    if (section.schema.length !== 3) {
        throw new Error(`Expected 3 schema columns, got ${section.schema.length}`);
    }
    if (section.rows.length !== 2) {
        throw new Error(`Expected 2 rows, got ${section.rows.length}`);
    }

    console.log('✓ Property 1.9: parseLlm parses complete legacy documents');
}

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for LLM Parser...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'parseValue handles empty string',
            test: () => {
                const result = parseValue('');
                return result.type === 'string' && result.value === '';
            }
        },
        {
            name: 'parseValue handles quoted strings',
            test: () => {
                const result = parseValue('"hello world"');
                return result.type === 'string' && result.value === 'hello world';
            }
        },
        {
            name: 'parseValue handles single-quoted strings',
            test: () => {
                const result = parseValue("'hello'");
                return result.type === 'string' && result.value === 'hello';
            }
        },
        {
            name: 'parseRefDefinition parses valid refs',
            test: () => {
                const result = parseRefDefinition('A|CommonValue');
                return result !== null && result[0] === 'A' && result[1] === 'CommonValue';
            }
        },
        {
            name: 'parseRefDefinition returns null for invalid refs',
            test: () => {
                const result = parseRefDefinition('invalid');
                return result === null;
            }
        },
        {
            name: 'parseDataRow handles correct number of values',
            test: () => {
                const row = parseDataRow('a|b|c', 3);
                return row.length === 3;
            }
        },
        {
            name: 'parseDataRow pads missing values',
            test: () => {
                const row = parseDataRow('a|b', 4);
                return row.length === 4 && row[3].type === 'string' && row[3].value === '';
            }
        },
        {
            name: 'parseLlm handles empty input',
            test: () => {
                const result = parseLlm('');
                return result.success && result.document !== undefined;
            }
        },
        {
            name: 'parseLlm parses legacy format with #c: prefix',
            test: () => {
                const result = parseLlm('// comment\n#c:nm|Test');
                return result.success && result.document!.context.has('nm');
            }
        },
        {
            name: 'parseLlm parses legacy root-level format',
            test: () => {
                const result = parseLlm('nm|Test\nv|1.0');
                return result.success &&
                    result.document!.context.has('nm') &&
                    result.document!.context.has('v');
            }
        },
        {
            name: 'parseLlm handles multiple legacy sections',
            test: () => {
                const input = '#a(x|y)\n1|2\n#b(p|q)\n3|4';
                const result = parseLlm(input);
                return result.success &&
                    result.document!.sections.has('a') &&
                    result.document!.sections.has('b');
            }
        },
        {
            name: 'parseLlm reports error for invalid section header',
            test: () => {
                const result = parseLlm('#invalid(');
                return !result.success && result.error !== undefined;
            }
        },
        {
            name: 'parseLlm parses new format object',
            test: () => {
                const result = parseLlm('config[host=localhost port=8080]');
                return result.success && result.document!.context.has('config');
            }
        },
        {
            name: 'parseLlm parses new format tabular array',
            test: () => {
                const result = parseLlm('users(id name)[1 Alice\n2 Bob]');
                return result.success && result.document!.sections.has('users');
            }
        },
        {
            name: 'parseLlm parses new format simple array',
            test: () => {
                const result = parseLlm('items=apple banana cherry');
                return result.success && result.document!.context.has('items');
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
    console.log('Running Property Tests for LLM Parser...\n');

    console.log('--- New dx-serializer Format Tests ---');
    testParseNewFormatObject();
    testParseNewFormatTabular();
    testParseNewFormatArray();
    testParseNewFormatQuotedStrings();
    testParseNewFormatNestedSchema();

    console.log('\n--- Legacy Format Tests (Backward Compatibility) ---');
    testParseValueBooleans();
    testParseValueNull();
    testParseValueNumbers();
    testParseValueStrings();
    testParseValueReferences();
    testParseValueArrays();
    testParseContextSection();
    testParseDataSectionHeader();
    testParseLlmLegacyComplete();

    console.log('\n✓ All Property tests passed!');
}

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
