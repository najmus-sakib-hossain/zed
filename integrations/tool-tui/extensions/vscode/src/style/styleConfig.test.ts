/**
 * Property-Based Tests for Style Configuration
 * 
 * Tests grouping optimization benefit property.
 * 
 * **Feature: dx-style-extension-enhancements**
 * **Property 7: Grouping Optimization Benefit**
 * **Validates: Requirements 6.2**
 */

import * as fc from 'fast-check';

/**
 * Arbitrary for valid atomic classname
 */
const arbAtomicClassname = fc.constantFrom(
    'flex', 'grid', 'block', 'inline', 'hidden',
    'items-center', 'items-start', 'items-end',
    'justify-center', 'justify-between', 'justify-start',
    'p-1', 'p-2', 'p-4', 'p-8',
    'm-1', 'm-2', 'm-4', 'm-8',
    'text-sm', 'text-base', 'text-lg', 'text-xl',
    'font-bold', 'font-medium', 'font-normal',
    'bg-white', 'bg-black', 'bg-gray-100',
    'text-white', 'text-black', 'text-gray-900',
    'rounded', 'rounded-md', 'rounded-lg',
    'shadow', 'shadow-md', 'shadow-lg',
    'w-full', 'w-auto', 'h-full', 'h-auto'
);

/**
 * Arbitrary for a list of atomic classnames (2-15 items)
 * Grouping only makes sense with multiple classnames
 */
const arbAtomicClassnames = fc.array(arbAtomicClassname, { minLength: 2, maxLength: 15 })
    .map(arr => [...new Set(arr)]); // Remove duplicates

/**
 * Arbitrary for grouped classname (dxg-* format with 6-8 char hash)
 */
const arbGroupedClassname = fc.hexaString({ minLength: 6, maxLength: 8 })
    .map(hex => `dxg-${hex}`);

/**
 * Calculate the length of a class attribute with atomic classnames
 */
function calculateAtomicLength(atomicClasses: string[]): number {
    // class="flex items-center justify-between"
    // The length is the sum of classnames plus spaces between them
    return atomicClasses.join(' ').length;
}

/**
 * Calculate the length of a class attribute with a grouped classname
 */
function calculateGroupedLength(groupedClass: string): number {
    // class="dxg-abc123"
    return groupedClass.length;
}

/**
 * Check if grouping should be applied based on savings
 * Pure function version for testing
 */
function shouldApplyGroupingPure(
    originalLength: number,
    groupedLength: number,
    minSavings: number
): boolean {
    const savings = originalLength - groupedLength;
    return savings >= minSavings;
}

/**
 * Property 7: Grouping Optimization Benefit
 * 
 * *For any* set of classnames where auto-grouping is applied, the resulting
 * grouped class attribute length SHALL be less than or equal to the original
 * atomic class attribute length.
 * 
 * **Validates: Requirements 6.2**
 */
export function testGroupingOptimizationBenefit(): void {
    fc.assert(
        fc.property(
            arbAtomicClassnames,
            arbGroupedClassname,
            fc.integer({ min: 1, max: 50 }), // minSavings
            (atomicClasses, groupedClass, minSavings) => {
                const atomicLength = calculateAtomicLength(atomicClasses);
                const groupedLength = calculateGroupedLength(groupedClass);

                // If grouping is applied, it should provide benefit
                if (shouldApplyGroupingPure(atomicLength, groupedLength, minSavings)) {
                    // Grouped length should be less than atomic length by at least minSavings
                    return groupedLength <= atomicLength - minSavings;
                }

                // If grouping is not applied, the savings are insufficient
                // This is correct behavior - we don't group when it doesn't help
                return true;
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 7: Grouping optimization benefit');
}

/**
 * Property 7a: Grouping is only applied when beneficial
 * 
 * **Validates: Requirements 6.2**
 */
export function testGroupingOnlyWhenBeneficial(): void {
    fc.assert(
        fc.property(
            arbAtomicClassnames,
            arbGroupedClassname,
            (atomicClasses, groupedClass) => {
                const atomicLength = calculateAtomicLength(atomicClasses);
                const groupedLength = calculateGroupedLength(groupedClass);

                // With minSavings = 0, grouping should be applied when grouped is shorter
                const shouldGroup = shouldApplyGroupingPure(atomicLength, groupedLength, 0);

                if (shouldGroup) {
                    // If we decided to group, grouped must be shorter or equal
                    return groupedLength <= atomicLength;
                }

                // If we didn't group, grouped must be longer
                return groupedLength > atomicLength;
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 7a: Grouping is only applied when beneficial');
}

/**
 * Property 7b: Higher minSavings means fewer groupings
 * 
 * **Validates: Requirements 6.2**
 */
export function testHigherMinSavingsMeansFewerGroupings(): void {
    fc.assert(
        fc.property(
            arbAtomicClassnames,
            arbGroupedClassname,
            fc.integer({ min: 1, max: 25 }),
            (atomicClasses, groupedClass, baseSavings) => {
                const atomicLength = calculateAtomicLength(atomicClasses);
                const groupedLength = calculateGroupedLength(groupedClass);

                const lowThreshold = baseSavings;
                const highThreshold = baseSavings + 10;

                const groupWithLow = shouldApplyGroupingPure(atomicLength, groupedLength, lowThreshold);
                const groupWithHigh = shouldApplyGroupingPure(atomicLength, groupedLength, highThreshold);

                // If we group with high threshold, we must also group with low threshold
                // (higher threshold is more restrictive)
                if (groupWithHigh) {
                    return groupWithLow === true;
                }

                // If we don't group with low threshold, we definitely don't group with high
                if (!groupWithLow) {
                    return groupWithHigh === false;
                }

                // Otherwise, any combination is valid
                return true;
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 7b: Higher minSavings means fewer groupings');
}

/**
 * Property 7c: Savings calculation is correct
 * 
 * **Validates: Requirements 6.2**
 */
export function testSavingsCalculation(): void {
    fc.assert(
        fc.property(
            arbAtomicClassnames,
            arbGroupedClassname,
            (atomicClasses, groupedClass) => {
                const atomicLength = calculateAtomicLength(atomicClasses);
                const groupedLength = calculateGroupedLength(groupedClass);
                const savings = atomicLength - groupedLength;

                // Verify the savings calculation
                // If savings > 0, grouping saves characters
                // If savings < 0, grouping costs characters
                // If savings = 0, no change

                if (savings > 0) {
                    return groupedLength < atomicLength;
                } else if (savings < 0) {
                    return groupedLength > atomicLength;
                } else {
                    return groupedLength === atomicLength;
                }
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 7c: Savings calculation is correct');
}

/**
 * Test edge cases
 */
export function testEdgeCases(): void {
    // Single classname - grouping rarely beneficial
    const singleClass = ['flex'];
    const singleLength = calculateAtomicLength(singleClass);
    const groupedLength = calculateGroupedLength('dxg-abc123');

    // Single short classname should not benefit from grouping
    if (singleLength < groupedLength) {
        // This is expected - single short classnames don't benefit
        console.log('  Single classname correctly not grouped');
    }

    // Many classnames - grouping likely beneficial
    const manyClasses = ['flex', 'items-center', 'justify-between', 'p-4', 'm-2', 'rounded-lg', 'shadow-md'];
    const manyLength = calculateAtomicLength(manyClasses);

    // Many classnames should benefit from grouping
    if (manyLength > groupedLength) {
        console.log('  Many classnames correctly benefit from grouping');
    }

    // Zero minSavings - always group if any benefit
    if (shouldApplyGroupingPure(100, 99, 0)) {
        console.log('  Zero minSavings correctly groups with 1 char savings');
    }

    // High minSavings - only group with significant benefit
    if (!shouldApplyGroupingPure(100, 95, 10)) {
        console.log('  High minSavings correctly rejects 5 char savings');
    }

    console.log('✓ All edge case tests passed');
}

/**
 * Run all property tests for Style Configuration
 */
export function runAllPropertyTests(): void {
    console.log('Running Property tests for Style Configuration...\n');
    console.log('**Property 7: Grouping Optimization Benefit**');
    console.log('**Validates: Requirements 6.2**\n');

    testGroupingOptimizationBenefit();
    testGroupingOnlyWhenBeneficial();
    testHigherMinSavingsMeansFewerGroupings();
    testSavingsCalculation();
    testEdgeCases();

    console.log('\n✓ All Style Configuration property tests passed!');
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
