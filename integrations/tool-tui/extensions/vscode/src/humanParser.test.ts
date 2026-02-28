/**
 * Property-based tests for Human Parser
 * 
 * Feature: dx-serializer-extension-fix, Property 2: Human to LLM to Human Round-Trip
 * 
 * For any valid human format content, parsing and re-formatting should preserve
 * the semantic content (though formatting may differ).
 * 
 * **Validates: Requirements 3.1-3.5, 3.6**
 */

import * as fc from 'fast-check';
import {
    parseHuman,
    parseSectionHeader,
    sectionNameToId,
    parseConfigLine,
    parseHumanValue,
    isTableBorder,
    isTableRow,
    parseTableRow,
    parseTableCellValue,
    parseSchemaComment,
    detectReferences,
    serializeValue,
    serializeToLlm,
    HumanParseResult,
} from './humanParser';
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
    refValue,
} from './llmParser';
import { formatDocument } from './humanFormatter';

// ============================================================================
// Generators
// ============================================================================

/**
 * Generate a valid key (alphanumeric with underscores)
 */
const validKey = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789_'),
    { minLength: 1, maxLength: 10 }
).filter((s: string) => /^[a-z]/.test(s));

/**
 * Generate a simple string value (must start with letter to avoid being parsed as number)
 */
const simpleString = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'),
    { minLength: 2, maxLength: 15 }
).filter((s: string) => /^[a-zA-Z]/.test(s));

/**
 * Generate a number value
 */
const numberValue = fc.integer({ min: -10000, max: 10000 });

// ============================================================================
// Property Tests
// ============================================================================

/**
 * Property 3.1: parseSectionHeader recognizes valid headers
 */
export function testParseSectionHeader(): void {
    const sectionNames = ['config', 'data', 'users', 'items', 'test123'];

    for (const name of sectionNames) {
        const result = parseSectionHeader(`[${name}]`);
        if (result !== name) {
            throw new Error(`Expected '${name}', got '${result}'`);
        }
    }

    // Test invalid headers
    const invalidHeaders = ['config', '[config', 'config]', '[]', '[123]'];
    for (const header of invalidHeaders) {
        const result = parseSectionHeader(header);
        if (result !== null && header !== '[123]') {
            throw new Error(`Expected null for invalid header '${header}', got '${result}'`);
        }
    }

    console.log('✓ Property 3.1: parseSectionHeader recognizes valid headers');
}

/**
 * Property 3.2: sectionNameToId maps names correctly
 */
export function testSectionNameToId(): void {
    // Special mappings
    if (sectionNameToId('config') !== 'c') {
        throw new Error(`Expected 'c' for 'config', got '${sectionNameToId('config')}'`);
    }
    if (sectionNameToId('context') !== 'c') {
        throw new Error(`Expected 'c' for 'context', got '${sectionNameToId('context')}'`);
    }
    if (sectionNameToId('data') !== 'd') {
        throw new Error(`Expected 'd' for 'data', got '${sectionNameToId('data')}'`);
    }

    // Generic mapping (first letter)
    if (sectionNameToId('users') !== 'u') {
        throw new Error(`Expected 'u' for 'users', got '${sectionNameToId('users')}'`);
    }

    console.log('✓ Property 3.2: sectionNameToId maps names correctly');
}

/**
 * Property 3.3: parseConfigLine parses key = value format
 */
export function testParseConfigLine(): void {
    fc.assert(
        fc.property(validKey, simpleString, (key: string, value: string) => {
            const line = `${key} = ${value}`;
            const result = parseConfigLine(line);

            if (!result) {
                throw new Error(`Failed to parse config line: ${line}`);
            }

            const [parsedKey, parsedValue] = result;
            if (parsedValue.type !== 'string' || parsedValue.value !== value) {
                throw new Error(`Expected value '${value}', got '${parsedValue.value}'`);
            }

            return true;
        }),
        { numRuns: 50 }
    );
    console.log('✓ Property 3.3: parseConfigLine parses key = value format');
}

/**
 * Property 3.4: parseHumanValue handles all value types
 */
export function testParseHumanValue(): void {
    // String
    const strResult = parseHumanValue('hello');
    if (strResult.type !== 'string' || strResult.value !== 'hello') {
        throw new Error(`Expected string 'hello', got ${JSON.stringify(strResult)}`);
    }

    // Number
    const numResult = parseHumanValue('42');
    if (numResult.type !== 'number' || numResult.value !== 42) {
        throw new Error(`Expected number 42, got ${JSON.stringify(numResult)}`);
    }

    // Boolean true
    const trueResult = parseHumanValue('true');
    if (trueResult.type !== 'bool' || trueResult.value !== true) {
        throw new Error(`Expected bool true, got ${JSON.stringify(trueResult)}`);
    }

    // Boolean false
    const falseResult = parseHumanValue('false');
    if (falseResult.type !== 'bool' || falseResult.value !== false) {
        throw new Error(`Expected bool false, got ${JSON.stringify(falseResult)}`);
    }

    // Null
    const nullResult = parseHumanValue('null');
    if (nullResult.type !== 'null' || nullResult.value !== null) {
        throw new Error(`Expected null, got ${JSON.stringify(nullResult)}`);
    }

    // Array
    const arrResult = parseHumanValue('[a, b, c]');
    if (arrResult.type !== 'array') {
        throw new Error(`Expected array, got ${arrResult.type}`);
    }
    const arr = arrResult.value as DxValue[];
    if (arr.length !== 3) {
        throw new Error(`Expected 3 items, got ${arr.length}`);
    }

    // Quoted string
    const quotedResult = parseHumanValue('"hello world"');
    if (quotedResult.type !== 'string' || quotedResult.value !== 'hello world') {
        throw new Error(`Expected 'hello world', got ${JSON.stringify(quotedResult)}`);
    }

    console.log('✓ Property 3.4: parseHumanValue handles all value types');
}

/**
 * Property 3.5: Table parsing functions work correctly
 */
export function testTableParsing(): void {
    // isTableBorder
    if (!isTableBorder('┌───┬───┐')) {
        throw new Error('Should recognize top border');
    }
    if (!isTableBorder('├───┼───┤')) {
        throw new Error('Should recognize middle border');
    }
    if (!isTableBorder('└───┴───┘')) {
        throw new Error('Should recognize bottom border');
    }
    if (isTableBorder('hello')) {
        throw new Error('Should not recognize text as border');
    }

    // isTableRow
    if (!isTableRow('│ a │ b │')) {
        throw new Error('Should recognize table row');
    }
    if (isTableRow('hello')) {
        throw new Error('Should not recognize text as table row');
    }

    // parseTableRow
    const cells = parseTableRow('│ Alice │ 42 │ ✓ │');
    if (cells.length !== 3) {
        throw new Error(`Expected 3 cells, got ${cells.length}`);
    }
    if (cells[0] !== 'Alice') {
        throw new Error(`Expected 'Alice', got '${cells[0]}'`);
    }

    console.log('✓ Property 3.5: Table parsing functions work correctly');
}

/**
 * Property 3.6: parseTableCellValue handles special symbols
 */
export function testParseTableCellValue(): void {
    // Checkmark -> true
    const trueResult = parseTableCellValue('✓');
    if (trueResult.type !== 'bool' || trueResult.value !== true) {
        throw new Error(`Expected bool true for ✓, got ${JSON.stringify(trueResult)}`);
    }

    // X mark -> false
    const falseResult = parseTableCellValue('✗');
    if (falseResult.type !== 'bool' || falseResult.value !== false) {
        throw new Error(`Expected bool false for ✗, got ${JSON.stringify(falseResult)}`);
    }

    // Em dash -> null
    const nullResult = parseTableCellValue('—');
    if (nullResult.type !== 'null') {
        throw new Error(`Expected null for —, got ${JSON.stringify(nullResult)}`);
    }

    // Number
    const numResult = parseTableCellValue('42');
    if (numResult.type !== 'number' || numResult.value !== 42) {
        throw new Error(`Expected number 42, got ${JSON.stringify(numResult)}`);
    }

    console.log('✓ Property 3.6: parseTableCellValue handles special symbols');
}

/**
 * Property 3.7: parseSchemaComment extracts schema
 */
export function testParseSchemaComment(): void {
    const schema = parseSchemaComment('# Schema: id | name | active');
    if (!schema) {
        throw new Error('Failed to parse schema comment');
    }
    if (schema.length !== 3) {
        throw new Error(`Expected 3 columns, got ${schema.length}`);
    }

    // Should compress keys
    if (schema[1] !== 'nm') {
        throw new Error(`Expected 'nm' for 'name', got '${schema[1]}'`);
    }

    // Invalid schema comment
    const invalid = parseSchemaComment('// not a schema');
    if (invalid !== null) {
        throw new Error('Should return null for invalid schema comment');
    }

    console.log('✓ Property 3.7: parseSchemaComment extracts schema');
}

/**
 * Property 3.8: serializeValue produces correct LLM format
 */
export function testSerializeValue(): void {
    // String
    if (serializeValue(strValue('hello')) !== 'hello') {
        throw new Error('String serialization failed');
    }

    // Number
    if (serializeValue(numValue(42)) !== '42') {
        throw new Error('Number serialization failed');
    }

    // Boolean true
    if (serializeValue(boolValue(true)) !== '+') {
        throw new Error('Boolean true serialization failed');
    }

    // Boolean false
    if (serializeValue(boolValue(false)) !== '-') {
        throw new Error('Boolean false serialization failed');
    }

    // Null
    if (serializeValue(nullValue()) !== '~') {
        throw new Error('Null serialization failed');
    }

    // Array
    const arrResult = serializeValue(arrValue([strValue('a'), strValue('b')]));
    if (arrResult !== '*a,b') {
        throw new Error(`Expected '*a,b', got '${arrResult}'`);
    }

    // Reference
    if (serializeValue(refValue('A')) !== '^A') {
        throw new Error('Reference serialization failed');
    }

    console.log('✓ Property 3.8: serializeValue produces correct LLM format');
}

/**
 * Property 3.9: serializeToLlm produces valid LLM format
 */
export function testSerializeToLlm(): void {
    const doc = createDocument();
    doc.context.set('nm', strValue('Test'));
    doc.context.set('ct', numValue(42));
    doc.refs.set('A', 'CommonValue');

    const section = createSection('d', ['id', 'nm', 'ac']);
    section.rows.push([numValue(1), strValue('Alice'), boolValue(true)]);
    section.rows.push([numValue(2), strValue('Bob'), boolValue(false)]);
    doc.sections.set('d', section);

    const result = serializeToLlm(doc);

    // New format: root-level key|value pairs without #c: prefix
    if (result.includes('#c:')) {
        throw new Error(`Should NOT contain #c: prefix (new format): ${result}`);
    }

    // Should contain context key-value pairs
    if (!result.includes('nm|Test')) {
        throw new Error(`Should contain context key-value pair: ${result}`);
    }

    // Should contain reference
    if (!result.includes('#:A|CommonValue')) {
        throw new Error(`Should contain reference: ${result}`);
    }

    // Should contain data section header
    if (!result.includes('#d(id|nm|ac)')) {
        throw new Error(`Should contain data section header: ${result}`);
    }

    // Should contain data rows
    if (!result.includes('1|Alice|+')) {
        throw new Error(`Should contain first data row: ${result}`);
    }
    if (!result.includes('2|Bob|-')) {
        throw new Error(`Should contain second data row: ${result}`);
    }

    console.log('✓ Property 3.9: serializeToLlm produces valid LLM format');
}

/**
 * Property 2: Human to LLM to Human round-trip preserves data
 */
export function testHumanToLlmRoundTrip(): void {
    // Create a document
    const doc = createDocument();
    doc.context.set('nm', strValue('TestApp'));
    doc.context.set('ct', numValue(100));

    const section = createSection('d', ['id', 'nm', 'ac']);
    section.rows.push([numValue(1), strValue('Alice'), boolValue(true)]);
    section.rows.push([numValue(2), strValue('Bob'), boolValue(false)]);
    doc.sections.set('d', section);

    // Format to human
    const human = formatDocument(doc);

    // Parse back
    const parseResult = parseHuman(human);
    if (!parseResult.success) {
        throw new Error(`Parse failed: ${parseResult.error?.message}`);
    }

    const parsed = parseResult.document!;

    // Verify context preserved
    if (!parsed.context.has('nm')) {
        throw new Error('Context key "nm" not preserved');
    }

    // Verify sections preserved
    if (!parsed.sections.has('d')) {
        throw new Error('Section "d" not preserved');
    }

    const parsedSection = parsed.sections.get('d')!;
    if (parsedSection.rows.length !== 2) {
        throw new Error(`Expected 2 rows, got ${parsedSection.rows.length}`);
    }

    console.log('✓ Property 2: Human to LLM to Human round-trip preserves data');
}

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Human Parser...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'parseHuman handles empty input',
            test: () => {
                const result = parseHuman('');
                return result.success && result.document !== undefined;
            }
        },
        {
            name: 'parseHuman skips comments',
            test: () => {
                const result = parseHuman('// comment\n[config]\nname = Test');
                return result.success && result.document!.context.has('nm');
            }
        },
        {
            name: 'parseHuman skips header decorations',
            test: () => {
                const input = '# ────────────────\n# CONFIG\n# ────────────────\n[config]\nname = Test';
                const result = parseHuman(input);
                return result.success && result.document!.context.has('nm');
            }
        },
        {
            name: 'parseHuman handles multiple sections',
            test: () => {
                const input = '[config]\nname = Test\n[data]\n# Schema: id\n┌───┐\n│ Id │\n├───┤\n│ 1 │\n└───┘';
                const result = parseHuman(input);
                return result.success &&
                    result.document!.context.size > 0 &&
                    result.document!.sections.size > 0;
            }
        },
        {
            name: 'parseConfigLine returns null for comments',
            test: () => {
                return parseConfigLine('# comment') === null &&
                    parseConfigLine('// comment') === null;
            }
        },
        {
            name: 'parseConfigLine returns null for empty lines',
            test: () => {
                return parseConfigLine('') === null &&
                    parseConfigLine('   ') === null;
            }
        },
        {
            name: 'parseHumanValue handles empty array',
            test: () => {
                const result = parseHumanValue('[]');
                return result.type === 'array' && (result.value as DxValue[]).length === 0;
            }
        },
        {
            name: 'parseHumanValue handles negative numbers',
            test: () => {
                const result = parseHumanValue('-42');
                return result.type === 'number' && result.value === -42;
            }
        },
        {
            name: 'parseHumanValue handles decimal numbers',
            test: () => {
                const result = parseHumanValue('3.14');
                return result.type === 'number' && result.value === 3.14;
            }
        },
        {
            name: 'detectReferences finds repeated strings',
            test: () => {
                const doc = createDocument();
                const section = createSection('d', ['nm']);
                section.rows.push([strValue('CommonValue')]);
                section.rows.push([strValue('CommonValue')]);
                section.rows.push([strValue('UniqueValue')]);
                doc.sections.set('d', section);

                const refs = detectReferences(doc);
                return refs.has('CommonValue') && !refs.has('UniqueValue');
            }
        },
        {
            name: 'serializeToLlm handles empty document',
            test: () => {
                const doc = createDocument();
                const result = serializeToLlm(doc);
                return result === '';
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
    console.log('Running Property 2: Human to LLM Round-Trip tests...\n');

    testParseSectionHeader();
    testSectionNameToId();
    testParseConfigLine();
    testParseHumanValue();
    testTableParsing();
    testParseTableCellValue();
    testParseSchemaComment();
    testSerializeValue();
    testSerializeToLlm();
    testHumanToLlmRoundTrip();

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
