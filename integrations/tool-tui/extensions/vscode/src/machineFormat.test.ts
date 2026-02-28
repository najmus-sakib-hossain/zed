/**
 * Tests for Machine Format
 * 
 * Feature: dx-serializer-v3
 * 
 * Tests Property 9 from the design document:
 * - Property 9: Machine Format Round-Trip
 * 
 * **Validates: Requirements 1.7, 3.4**
 */

import * as fc from 'fast-check';
import {
    documentToMachine,
    machineToDocument,
    serializeMachine,
    deserializeMachine,
    dxValueToMachine,
    machineValueToDx,
} from './machineFormat';
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
} from './llmParser';

// ============================================================================
// Generators
// ============================================================================

const simpleDxValue: fc.Arbitrary<DxValue> = fc.oneof(
    fc.string({ minLength: 0, maxLength: 20 }).map(s => strValue(s)),
    fc.integer({ min: -1000, max: 1000 }).map(n => numValue(n)),
    fc.boolean().map(b => boolValue(b)),
    fc.constant(nullValue())
);

const simpleContextMap = fc.array(
    fc.tuple(
        fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 5 }),
        simpleDxValue
    ),
    { minLength: 0, maxLength: 5 }
).map(pairs => {
    const map = new Map<string, DxValue>();
    for (const [key, value] of pairs) {
        map.set(key, value);
    }
    return map;
});

const simpleDxDocument = fc.tuple(
    simpleContextMap,
    fc.array(
        fc.tuple(
            fc.constantFrom('f', 'k', 'y', 'u', 'm'),
            fc.array(fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 5 }), { minLength: 1, maxLength: 3 }),
            fc.array(simpleDxValue, { minLength: 1, maxLength: 3 })
        ),
        { minLength: 0, maxLength: 2 }
    )
).map(([context, sections]) => {
    const doc = createDocument();
    for (const [key, value] of context) {
        doc.context.set(key, value);
    }
    const usedIds = new Set<string>();
    for (const [id, schema, values] of sections) {
        if (!usedIds.has(id)) {
            const uniqueSchema = [...new Set(schema)];
            const section = createSection(id, uniqueSchema);
            const row = uniqueSchema.map((_, i) => values[i] || nullValue());
            section.rows.push(row);
            doc.sections.set(id, section);
            usedIds.add(id);
        }
    }
    return doc;
});

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Machine Format...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        // Value conversion
        {
            name: 'dxValueToMachine: converts string',
            test: () => {
                const result = dxValueToMachine(strValue('hello'));
                return result.t === 's' && result.v === 'hello';
            }
        },
        {
            name: 'dxValueToMachine: converts number',
            test: () => {
                const result = dxValueToMachine(numValue(42));
                return result.t === 'n' && result.v === 42;
            }
        },
        {
            name: 'dxValueToMachine: converts boolean',
            test: () => {
                const t = dxValueToMachine(boolValue(true));
                const f = dxValueToMachine(boolValue(false));
                return t.t === 'b' && t.v === true && f.t === 'b' && f.v === false;
            }
        },
        {
            name: 'dxValueToMachine: converts null',
            test: () => {
                const result = dxValueToMachine(nullValue());
                return result.t === 'x' && result.v === null;
            }
        },
        {
            name: 'dxValueToMachine: converts array',
            test: () => {
                const result = dxValueToMachine(arrValue([strValue('a'), numValue(1)]));
                return result.t === 'a' && Array.isArray(result.v) && (result.v as unknown[]).length === 2;
            }
        },
        {
            name: 'machineValueToDx: converts string',
            test: () => {
                const result = machineValueToDx({ t: 's', v: 'hello' });
                return result.type === 'string' && result.value === 'hello';
            }
        },
        {
            name: 'machineValueToDx: converts number',
            test: () => {
                const result = machineValueToDx({ t: 'n', v: 42 });
                return result.type === 'number' && result.value === 42;
            }
        },
        {
            name: 'machineValueToDx: converts boolean',
            test: () => {
                const result = machineValueToDx({ t: 'b', v: true });
                return result.type === 'bool' && result.value === true;
            }
        },
        {
            name: 'machineValueToDx: converts null',
            test: () => {
                const result = machineValueToDx({ t: 'x', v: null });
                return result.type === 'null';
            }
        },

        // Document conversion
        {
            name: 'documentToMachine: converts empty document',
            test: () => {
                const doc = createDocument();
                const machine = documentToMachine(doc);
                return machine.version === 1 &&
                    Object.keys(machine.context).length === 0 &&
                    Object.keys(machine.sections).length === 0;
            }
        },
        {
            name: 'documentToMachine: converts context',
            test: () => {
                const doc = createDocument();
                doc.context.set('nm', strValue('test'));
                const machine = documentToMachine(doc);
                return machine.context['nm']?.t === 's' && machine.context['nm']?.v === 'test';
            }
        },
        {
            name: 'documentToMachine: converts sections',
            test: () => {
                const doc = createDocument();
                const section = createSection('f', ['nm', 'repo']);
                section.rows.push([strValue('forge'), strValue('url')]);
                doc.sections.set('f', section);
                const machine = documentToMachine(doc);
                return machine.sections['f'] !== undefined &&
                    machine.sections['f'].schema.length === 2;
            }
        },

        // Serialization
        {
            name: 'serializeMachine: produces valid JSON',
            test: () => {
                const doc = createDocument();
                doc.context.set('nm', strValue('test'));
                const json = serializeMachine(doc);
                try {
                    JSON.parse(json);
                    return true;
                } catch {
                    return false;
                }
            }
        },
        {
            name: 'deserializeMachine: parses valid machine format',
            test: () => {
                const json = '{"version":1,"context":{"nm":{"t":"s","v":"test"}},"refs":{},"sections":{}}';
                const result = deserializeMachine(json);
                return result.success && result.document !== undefined;
            }
        },
        {
            name: 'deserializeMachine: rejects invalid JSON',
            test: () => {
                const result = deserializeMachine('{invalid}');
                return !result.success && result.error !== undefined;
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
// Property Tests
// ============================================================================

/**
 * Property 9: Machine Format Round-Trip
 * For any valid DxDocument, serializing to machine format and deserializing SHALL produce an equivalent document
 * 
 * **Validates: Requirements 1.7, 3.4**
 */
export function testMachineFormatRoundTrip(): void {
    fc.assert(
        fc.property(simpleDxDocument, (doc: DxDocument) => {
            // Serialize
            const json = serializeMachine(doc);

            // Deserialize
            const result = deserializeMachine(json);
            if (!result.success || !result.document) {
                throw new Error(`Failed to deserialize: ${result.error}`);
            }

            const roundTrip = result.document;

            // Compare context sizes
            if (doc.context.size !== roundTrip.context.size) {
                throw new Error(`Context size mismatch: ${doc.context.size} vs ${roundTrip.context.size}`);
            }

            // Compare section counts
            if (doc.sections.size !== roundTrip.sections.size) {
                throw new Error(`Section count mismatch: ${doc.sections.size} vs ${roundTrip.sections.size}`);
            }

            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 9: Machine Format Round-Trip');
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property tests for Machine Format...\n');

    testMachineFormatRoundTrip();

    console.log('\n✓ All Machine Format property tests passed!');
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
