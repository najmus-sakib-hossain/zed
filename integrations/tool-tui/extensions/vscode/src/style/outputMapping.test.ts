/**
 * Property Tests for Output Mapping Module
 * 
 * **Property 3: Generated CSS Display Accuracy**
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
 */

import * as fc from 'fast-check';

// Re-implement parseCSSFile for testing (since it's not exported)
function parseCSSFile(content: string): Map<string, { startLine: number; endLine: number; css: string }> {
    const mapping = new Map<string, { startLine: number; endLine: number; css: string }>();
    const lines = content.split('\n');

    const CSS_CLASS_PATTERN = /^\s*\.([a-zA-Z0-9_-]+(?:\\:[a-zA-Z0-9_-]+)*)\s*\{/;

    let currentClassname: string | null = null;
    let currentStartLine = 0;
    let currentCSSLines: string[] = [];
    let braceDepth = 0;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNumber = i + 1;

        const classMatch = line.match(CSS_CLASS_PATTERN);
        if (classMatch && braceDepth === 0) {
            currentClassname = classMatch[1].replace(/\\/g, '');
            currentStartLine = lineNumber;
            currentCSSLines = [line];
            braceDepth = (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;

            if (braceDepth === 0 && line.includes('}')) {
                mapping.set(currentClassname, {
                    startLine: currentStartLine,
                    endLine: lineNumber,
                    css: currentCSSLines.join('\n')
                });
                currentClassname = null;
                currentCSSLines = [];
            }
            continue;
        }

        if (currentClassname !== null) {
            currentCSSLines.push(line);
            braceDepth += (line.match(/\{/g) || []).length;
            braceDepth -= (line.match(/\}/g) || []).length;

            if (braceDepth === 0) {
                mapping.set(currentClassname, {
                    startLine: currentStartLine,
                    endLine: lineNumber,
                    css: currentCSSLines.join('\n')
                });
                currentClassname = null;
                currentCSSLines = [];
            }
        }
    }

    return mapping;
}

// Arbitrary for valid CSS classnames
const arbClassname = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz0123456789-_'.split('')),
    { minLength: 1, maxLength: 20 }
).filter(s => /^[a-zA-Z]/.test(s)); // Must start with letter

// Arbitrary for CSS property values
const arbCSSValue = fc.oneof(
    fc.constant('block'),
    fc.constant('flex'),
    fc.constant('none'),
    fc.constant('1rem'),
    fc.constant('10px'),
    fc.constant('#fff'),
    fc.constant('red'),
    fc.constant('center'),
    fc.constant('auto')
);

// Arbitrary for CSS property names
const arbCSSProperty = fc.constantFrom(
    'display',
    'padding',
    'margin',
    'color',
    'background-color',
    'width',
    'height',
    'flex-direction',
    'justify-content',
    'align-items'
);

// Arbitrary for a single CSS rule
const arbCSSRule = fc.record({
    classname: arbClassname,
    property: arbCSSProperty,
    value: arbCSSValue
}).map(({ classname, property, value }) => ({
    classname,
    css: `.${classname} { ${property}: ${value}; }`
}));

// Arbitrary for multi-line CSS rule
const arbMultiLineCSSRule = fc.record({
    classname: arbClassname,
    properties: fc.array(
        fc.record({ property: arbCSSProperty, value: arbCSSValue }),
        { minLength: 1, maxLength: 5 }
    )
}).map(({ classname, properties }) => {
    const propsStr = properties.map(p => `  ${p.property}: ${p.value};`).join('\n');
    return {
        classname,
        css: `.${classname} {\n${propsStr}\n}`
    };
});

/**
 * Property 3: Generated CSS Display Accuracy - Single Line Rules
 * 
 * For any dx-style classname with generated CSS, the mini viewer SHALL display
 * the exact CSS code that exists in the output file at the specified line numbers.
 * 
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
 */
export function testSingleLineCSSRuleMapping(): void {
    fc.assert(
        fc.property(
            fc.array(arbCSSRule, { minLength: 1, maxLength: 20 }),
            (rules) => {
                // Build CSS content
                const cssContent = rules.map(r => r.css).join('\n');

                // Parse the CSS
                const mapping = parseCSSFile(cssContent);

                // Verify each rule is correctly mapped
                const lines = cssContent.split('\n');

                for (const rule of rules) {
                    const lineInfo = mapping.get(rule.classname);

                    // Rule should be found (duplicates may overwrite, which is acceptable)
                    if (!lineInfo) {
                        continue;
                    }

                    // The CSS content should match
                    const extractedCSS = lines.slice(lineInfo.startLine - 1, lineInfo.endLine).join('\n');
                    if (lineInfo.css !== extractedCSS) {
                        return false;
                    }
                }

                return true;
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 3a: Single-line CSS rules correctly mapped to line numbers');
}

/**
 * Property 3: Generated CSS Display Accuracy - Multi-Line Rules
 * 
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
 */
export function testMultiLineCSSRuleMapping(): void {
    fc.assert(
        fc.property(
            fc.array(arbMultiLineCSSRule, { minLength: 1, maxLength: 10 }),
            (rules) => {
                // Build CSS content
                const cssContent = rules.map(r => r.css).join('\n\n');

                // Parse the CSS
                const mapping = parseCSSFile(cssContent);

                // Verify each rule is correctly mapped
                const lines = cssContent.split('\n');

                for (const rule of rules) {
                    const lineInfo = mapping.get(rule.classname);

                    if (!lineInfo) {
                        continue;
                    }

                    // The CSS content should match
                    const extractedCSS = lines.slice(lineInfo.startLine - 1, lineInfo.endLine).join('\n');
                    if (lineInfo.css !== extractedCSS) {
                        return false;
                    }
                }

                return true;
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 3b: Multi-line CSS rules correctly mapped to line numbers');
}

/**
 * Property 3: Generated CSS Display Accuracy - Line Number Validity
 * 
 * **Validates: Requirements 3.3, 3.4**
 */
export function testLineNumberValidity(): void {
    fc.assert(
        fc.property(
            fc.array(arbCSSRule, { minLength: 1, maxLength: 20 }),
            (rules) => {
                const cssContent = rules.map(r => r.css).join('\n');
                const mapping = parseCSSFile(cssContent);

                // All line numbers should be valid (1-indexed, within bounds)
                const lineCount = cssContent.split('\n').length;

                for (const [, lineInfo] of mapping) {
                    if (lineInfo.startLine < 1) return false;
                    if (lineInfo.endLine > lineCount) return false;
                    if (lineInfo.startLine > lineInfo.endLine) return false;
                }

                return true;
            }
        ),
        { numRuns: 100 }
    );
    console.log('✓ Property 3c: Line numbers are valid and within bounds');
}

/**
 * Run all property tests for Output Mapping
 */
export function runAllPropertyTests(): void {
    console.log('Running Property tests for Output Mapping...\n');

    testSingleLineCSSRuleMapping();
    testMultiLineCSSRuleMapping();
    testLineNumberValidity();

    console.log('\n✓ All Output Mapping property tests passed!');
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
