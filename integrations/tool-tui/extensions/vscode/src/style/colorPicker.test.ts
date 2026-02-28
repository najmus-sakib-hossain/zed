/**
 * Property-Based Tests for Color Picker Module
 * 
 * Tests color parsing round-trip and color manipulation functions.
 * 
 * **Feature: dx-style-extension-enhancements, Property 4: Color Parsing Round-Trip**
 * **Validates: Requirements 4.4, 4.5**
 */

import * as fc from 'fast-check';
import {
    parseHex,
    parseRgb,
    parseHsl,
    parseColor,
    formatColor,
    rgbToHex,
    rgbToHsl,
    hslToRgb,
    isColorClass,
    parseColorFromClassname,
    updateClassnameColor,
    ColorValue
} from './colorCore';

/**
 * Arbitrary for valid RGB values (0-255)
 */
const arbRgbComponent = fc.integer({ min: 0, max: 255 });

/**
 * Arbitrary for valid RGB color
 */
const arbRgb = fc.record({
    r: arbRgbComponent,
    g: arbRgbComponent,
    b: arbRgbComponent
});

/**
 * Arbitrary for valid HSL values
 */
const arbHsl = fc.record({
    h: fc.integer({ min: 0, max: 360 }),
    s: fc.integer({ min: 0, max: 100 }),
    l: fc.integer({ min: 0, max: 100 })
});

/**
 * Arbitrary for valid HEX color string (6 characters)
 */
const arbHexString = fc.hexaString({ minLength: 6, maxLength: 6 }).map(hex => `#${hex}`);

/**
 * Arbitrary for color classname prefixes
 */
const arbColorPrefix = fc.constantFrom('bg-', 'text-', 'border-', 'ring-', 'fill-', 'stroke-');

/**
 * Arbitrary for named colors
 */
const arbNamedColor = fc.constantFrom(
    'red-500', 'blue-500', 'green-500', 'yellow-500', 'purple-500',
    'gray-500', 'slate-500', 'indigo-500', 'pink-500', 'orange-500'
);

/**
 * Property 4a: HEX format round-trip preserves color
 * 
 * *For any* color value in HEX format, parsing and then formatting 
 * SHALL produce an equivalent color value.
 * 
 * **Validates: Requirements 4.4, 4.5**
 */
export function testHexRoundTrip(): void {
    fc.assert(
        fc.property(arbHexString, (hex) => {
            const parsed = parseColor(hex);
            if (!parsed) {
                return false; // Should always parse valid hex
            }

            const formatted = formatColor(parsed, 'hex');
            const reparsed = parseColor(formatted);

            if (!reparsed) {
                return false;
            }

            // Compare RGB values (hex comparison may differ in case)
            return (
                parsed.rgb.r === reparsed.rgb.r &&
                parsed.rgb.g === reparsed.rgb.g &&
                parsed.rgb.b === reparsed.rgb.b
            );
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4a: HEX format round-trip preserves color');
}

/**
 * Property 4b: RGB format round-trip preserves color
 * 
 * **Validates: Requirements 4.4, 4.5**
 */
export function testRgbRoundTrip(): void {
    fc.assert(
        fc.property(arbRgb, ({ r, g, b }) => {
            const rgbString = `rgb(${r}, ${g}, ${b})`;
            const parsed = parseColor(rgbString);

            if (!parsed) {
                return false;
            }

            const formatted = formatColor(parsed, 'rgb');
            const reparsed = parseColor(formatted);

            if (!reparsed) {
                return false;
            }

            return (
                parsed.rgb.r === reparsed.rgb.r &&
                parsed.rgb.g === reparsed.rgb.g &&
                parsed.rgb.b === reparsed.rgb.b
            );
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4b: RGB format round-trip preserves color');
}


/**
 * Property 4c: HSL format round-trip preserves color within tolerance
 * 
 * **Validates: Requirements 4.4, 4.5**
 */
export function testHslRoundTrip(): void {
    fc.assert(
        fc.property(arbHsl, ({ h, s, l }) => {
            const hslString = `hsl(${h}, ${s}%, ${l}%)`;
            const parsed = parseColor(hslString);

            if (!parsed) {
                return false;
            }

            const formatted = formatColor(parsed, 'hsl');
            const reparsed = parseColor(formatted);

            if (!reparsed) {
                return false;
            }

            // HSL -> RGB -> HSL may have small rounding differences
            // Allow tolerance of 1 for each component
            const hDiff = Math.abs(parsed.hsl.h - reparsed.hsl.h);
            const sDiff = Math.abs(parsed.hsl.s - reparsed.hsl.s);
            const lDiff = Math.abs(parsed.hsl.l - reparsed.hsl.l);

            // Handle hue wraparound (359 vs 0)
            const hueOk = hDiff <= 1 || hDiff >= 359;

            return hueOk && sDiff <= 1 && lDiff <= 1;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4c: HSL format round-trip preserves color within tolerance');
}

/**
 * Property 4d: RGB to HEX to RGB preserves values
 * 
 * **Validates: Requirements 4.4**
 */
export function testRgbHexRgbRoundTrip(): void {
    fc.assert(
        fc.property(arbRgb, ({ r, g, b }) => {
            const hex = rgbToHex(r, g, b);
            const parsed = parseHex(hex);

            if (!parsed) {
                return false;
            }

            return parsed.r === r && parsed.g === g && parsed.b === b;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4d: RGB to HEX to RGB preserves values');
}

/**
 * Property 4e: RGB to HSL to RGB preserves values within tolerance
 * 
 * Note: HSL conversion has inherent precision loss due to rounding in both
 * directions. The conversion RGB -> HSL rounds to integer percentages, and
 * HSL -> RGB rounds to integer RGB values. This can cause up to 3 units of
 * difference per channel in normal cases (especially for highly saturated colors),
 * and more for very dark/light colors where hue/saturation information is lost.
 * 
 * **Validates: Requirements 4.4**
 */
export function testRgbHslRgbRoundTrip(): void {
    fc.assert(
        fc.property(arbRgb, ({ r, g, b }) => {
            const hsl = rgbToHsl(r, g, b);
            const rgb = hslToRgb(hsl.h, hsl.s, hsl.l);

            // For very dark colors (lightness rounds to 0%) or very light colors
            // (lightness rounds to 100%), HSL loses precision. Allow larger tolerance
            // for these edge cases. Normal cases allow tolerance of 3 due to double rounding.
            const isVeryDark = Math.max(r, g, b) < 5;
            const isVeryLight = Math.min(r, g, b) > 250;
            const tolerance = (isVeryDark || isVeryLight) ? 5 : 3;

            return (
                Math.abs(rgb.r - r) <= tolerance &&
                Math.abs(rgb.g - g) <= tolerance &&
                Math.abs(rgb.b - b) <= tolerance
            );
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4e: RGB to HSL to RGB preserves values within tolerance');
}

/**
 * Property 4f: Color classname detection is consistent
 * 
 * **Validates: Requirements 4.1**
 */
export function testColorClassnameDetection(): void {
    fc.assert(
        fc.property(arbColorPrefix, arbNamedColor, (prefix, color) => {
            const classname = `${prefix}${color}`;
            return isColorClass(classname);
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4f: Color classname detection is consistent');
}

/**
 * Property 4g: Color parsing from classnames works for named colors
 * 
 * **Validates: Requirements 4.4, 4.5**
 */
export function testColorParsingFromClassnames(): void {
    fc.assert(
        fc.property(arbColorPrefix, arbNamedColor, (prefix, color) => {
            const classname = `${prefix}${color}`;
            const parsed = parseColorFromClassname(classname);

            // Should parse successfully for named colors
            return parsed !== null;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4g: Color parsing from classnames works for named colors');
}

/**
 * Property 4h: Variant prefixes are handled correctly
 * 
 * **Validates: Requirements 4.1**
 */
export function testVariantPrefixHandling(): void {
    fc.assert(
        fc.property(
            fc.constantFrom('hover:', 'focus:', 'active:', 'dark:'),
            arbColorPrefix,
            arbNamedColor,
            (variant, prefix, color) => {
                const classname = `${variant}${prefix}${color}`;
                return isColorClass(classname);
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 4h: Variant prefixes are handled correctly');
}


/**
 * Property 4i: Color update preserves classname structure
 * 
 * **Validates: Requirements 4.3**
 */
export function testColorUpdatePreservesStructure(): void {
    fc.assert(
        fc.property(arbColorPrefix, arbNamedColor, arbRgb, (prefix, color, newRgb) => {
            const classname = `${prefix}${color}`;
            const newColor: ColorValue = {
                hex: rgbToHex(newRgb.r, newRgb.g, newRgb.b),
                rgb: newRgb,
                hsl: rgbToHsl(newRgb.r, newRgb.g, newRgb.b),
                oklch: null,
                alpha: 1
            };

            const updated = updateClassnameColor(classname, newColor);

            // Should still be a color class and preserve prefix
            return isColorClass(updated) && updated.startsWith(prefix);
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 4i: Color update preserves classname structure');
}

/**
 * Property 4j: Color update preserves variant prefix
 * 
 * **Validates: Requirements 4.3**
 */
export function testColorUpdatePreservesVariant(): void {
    fc.assert(
        fc.property(
            fc.constantFrom('hover:', 'focus:', 'active:'),
            arbColorPrefix,
            arbNamedColor,
            arbRgb,
            (variant, prefix, color, newRgb) => {
                const classname = `${variant}${prefix}${color}`;
                const newColor: ColorValue = {
                    hex: rgbToHex(newRgb.r, newRgb.g, newRgb.b),
                    rgb: newRgb,
                    hsl: rgbToHsl(newRgb.r, newRgb.g, newRgb.b),
                    oklch: null,
                    alpha: 1
                };

                const updated = updateClassnameColor(classname, newColor);

                // Should preserve variant prefix
                return updated.startsWith(variant);
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 4j: Color update preserves variant prefix');
}

/**
 * Test edge cases (unit tests)
 */
export function testEdgeCases(): void {
    // Short hex format
    const shortHex = parseHex('#fff');
    if (!shortHex || shortHex.r !== 255 || shortHex.g !== 255 || shortHex.b !== 255) {
        throw new Error('Short hex format failed');
    }

    // Hex without hash
    const noHash = parseHex('ff0000');
    if (!noHash || noHash.r !== 255 || noHash.g !== 0 || noHash.b !== 0) {
        throw new Error('Hex without hash failed');
    }

    // Invalid hex
    if (parseHex('#gggggg') !== null) {
        throw new Error('Invalid hex should return null');
    }
    if (parseHex('#12') !== null) {
        throw new Error('Too short hex should return null');
    }

    // RGBA with alpha
    const rgba = parseRgb('rgba(255, 0, 0, 0.5)');
    if (!rgba || rgba.r !== 255 || rgba.g !== 0 || rgba.b !== 0 || rgba.a !== 0.5) {
        throw new Error('RGBA parsing failed');
    }

    // HSLA with alpha
    const hsla = parseHsl('hsla(0, 100%, 50%, 0.5)');
    if (!hsla || hsla.h !== 0 || hsla.s !== 100 || hsla.l !== 50 || hsla.a !== 0.5) {
        throw new Error('HSLA parsing failed');
    }

    // Out-of-range RGB
    if (parseRgb('rgb(256, 0, 0)') !== null) {
        throw new Error('Out-of-range RGB should return null');
    }

    // Out-of-range HSL
    if (parseHsl('hsl(361, 50%, 50%)') !== null) {
        throw new Error('Out-of-range HSL should return null');
    }

    // Non-color classnames
    if (isColorClass('flex')) {
        throw new Error('flex should not be a color class');
    }
    if (isColorClass('p-4')) {
        throw new Error('p-4 should not be a color class');
    }
    if (isColorClass('w-full')) {
        throw new Error('w-full should not be a color class');
    }

    console.log('✓ All edge case tests passed');
}

/**
 * Run all property tests for Color Picker
 */
export function runAllPropertyTests(): void {
    console.log('Running Property tests for Color Picker...\n');
    console.log('**Property 4: Color Parsing Round-Trip**');
    console.log('**Validates: Requirements 4.4, 4.5**\n');

    testHexRoundTrip();
    testRgbRoundTrip();
    testHslRoundTrip();
    testRgbHexRgbRoundTrip();
    testRgbHslRgbRoundTrip();
    testColorClassnameDetection();
    testColorParsingFromClassnames();
    testVariantPrefixHandling();
    testColorUpdatePreservesStructure();
    testColorUpdatePreservesVariant();
    testEdgeCases();

    console.log('\n✓ All Color Picker property tests passed!');
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
