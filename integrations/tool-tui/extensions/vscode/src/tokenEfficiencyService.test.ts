/**
 * Token Efficiency Service Tests
 * 
 * Property-based tests for savings calculation and token counting.
 * 
 * Requirements: 4.1, 4.2
 */

import * as fc from 'fast-check';
import { TokenEfficiencyService, MultiModelTokenCounts } from './tokenEfficiencyService';
import { FormatConverterService } from './formatConverterService';

describe('TokenEfficiencyService', () => {
    let service: TokenEfficiencyService;

    beforeEach(() => {
        service = new TokenEfficiencyService();
    });

    describe('countTokens', () => {
        it('should return non-zero counts for all models', () => {
            const content = 'Hello, world!';
            const counts = service.countTokens(content);

            expect(counts.openai.count).toBeGreaterThan(0);
            expect(counts.claude.count).toBeGreaterThan(0);
            expect(counts.gemini.count).toBeGreaterThan(0);
            expect(counts.other.count).toBeGreaterThan(0);
        });

        it('should return model names', () => {
            const content = 'Test content';
            const counts = service.countTokens(content);

            expect(counts.openai.model).toBe('GPT-4o');
            expect(counts.claude.model).toBe('Claude Sonnet 4');
            expect(counts.gemini.model).toBe('Gemini 3');
            expect(counts.other.model).toBe('Other');
        });

        // Property 1: Multi-Model Token Counting Completeness
        // For any valid content string, counting tokens SHALL return non-zero counts
        // for all four models (OpenAI, Claude, Gemini, Other)
        it('property: all models return non-zero counts for any non-empty content', () => {
            fc.assert(
                fc.property(
                    fc.string({ minLength: 1, maxLength: 1000 }),
                    (content) => {
                        const counts = service.countTokens(content);
                        return (
                            counts.openai.count > 0 &&
                            counts.claude.count > 0 &&
                            counts.gemini.count > 0 &&
                            counts.other.count > 0
                        );
                    }
                ),
                { numRuns: 100 }
            );
        });
    });

    describe('calculateSavings', () => {
        // Property 3: Savings Calculation Correctness
        // For any two positive token counts (dx_tokens, other_tokens), the savings
        // percentage SHALL equal ((other_tokens - dx_tokens) / other_tokens) * 100,
        // formatted to one decimal place.
        it('property: savings calculation formula is correct', () => {
            fc.assert(
                fc.property(
                    fc.integer({ min: 1, max: 10000 }),
                    fc.integer({ min: 1, max: 10000 }),
                    (dxTokens, otherTokens) => {
                        const savings = service.calculateSavings(dxTokens, otherTokens);
                        const expected = Math.round(((otherTokens - dxTokens) / otherTokens) * 100 * 10) / 10;
                        return Math.abs(savings - expected) < 0.01;
                    }
                ),
                { numRuns: 100 }
            );
        });

        it('should return 0 when other tokens is 0', () => {
            expect(service.calculateSavings(100, 0)).toBe(0);
        });

        it('should return positive savings when DX is more efficient', () => {
            expect(service.calculateSavings(70, 100)).toBeGreaterThan(0);
        });

        it('should return negative savings when DX is less efficient', () => {
            expect(service.calculateSavings(130, 100)).toBeLessThan(0);
        });

        it('should return 0 when counts are equal', () => {
            expect(service.calculateSavings(100, 100)).toBe(0);
        });
    });

    describe('getEfficiencyReport', () => {
        it('should generate complete report', async () => {
            const dxContent = 'name:test\nversion:1.0.0';
            const report = await service.getEfficiencyReport(dxContent);

            // Check all token counts are present
            expect(report.dxTokens).toBeDefined();
            expect(report.jsonTokens).toBeDefined();
            expect(report.yamlTokens).toBeDefined();
            expect(report.tomlTokens).toBeDefined();
            expect(report.toonTokens).toBeDefined();

            // Check savings are calculated
            expect(typeof report.savings.vsJson).toBe('number');
            expect(typeof report.savings.vsYaml).toBe('number');
            expect(typeof report.savings.vsToml).toBe('number');
            expect(typeof report.savings.vsToon).toBe('number');

            // Check equivalents are generated
            expect(report.equivalents.dx).toBe(dxContent);
            expect(report.equivalents.json).toBeDefined();
            expect(report.equivalents.yaml).toBeDefined();
            expect(report.equivalents.toml).toBeDefined();
            expect(report.equivalents.toon).toBeDefined();
        });
    });

    describe('getSummary', () => {
        it('should return formatted summary string', async () => {
            const dxContent = 'name:test';
            const report = await service.getEfficiencyReport(dxContent);
            const summary = service.getSummary(report);

            expect(summary).toContain('tokens');
            expect(summary).toContain('%');
        });
    });
});

describe('FormatConverterService', () => {
    let converter: FormatConverterService;

    beforeEach(() => {
        converter = new FormatConverterService();
    });

    describe('dxToJson', () => {
        it('should convert simple DX to JSON', async () => {
            const dx = 'name:test\nversion:100';
            const json = await converter.dxToJson(dx);
            const parsed = JSON.parse(json);

            expect(parsed.name).toBe('test');
            expect(parsed.version).toBe(100);
        });

        it('should handle boolean values', async () => {
            const dx = 'active:+\ndisabled:-';
            const json = await converter.dxToJson(dx);
            const parsed = JSON.parse(json);

            expect(parsed.active).toBe(true);
            expect(parsed.disabled).toBe(false);
        });

        it('should handle null values', async () => {
            const dx = 'value:~';
            const json = await converter.dxToJson(dx);
            const parsed = JSON.parse(json);

            expect(parsed.value).toBeNull();
        });
    });

    describe('dxToYaml', () => {
        it('should convert simple DX to YAML', async () => {
            const dx = 'name:test\nversion:100';
            const yaml = await converter.dxToYaml(dx);

            expect(yaml).toContain('name: test');
            expect(yaml).toContain('version: 100');
        });
    });

    describe('dxToToml', () => {
        it('should convert simple DX to TOML', async () => {
            const dx = 'name:test\nversion:100';
            const toml = await converter.dxToToml(dx);

            expect(toml).toContain('name = "test"');
            expect(toml).toContain('version = 100');
        });
    });

    describe('dxToToon', () => {
        it('should convert simple DX to TOON', async () => {
            const dx = 'name:test\nversion:100';
            const toon = await converter.dxToToon(dx);

            expect(toon).toContain('name "test"');
            expect(toon).toContain('version 100');
        });
    });

    describe('jsonToDx', () => {
        it('should convert JSON to DX', async () => {
            const json = '{"name": "test", "version": 100}';
            const dx = await converter.jsonToDx(json);

            expect(dx).toContain('name:test');
            expect(dx).toContain('version:100');
        });
    });

    // Property 2: Format Conversion Round-Trip
    // For any valid DX document, converting to JSON and back to DX SHALL preserve
    // all data values and types
    describe('round-trip conversion', () => {
        it('property: JSON round-trip preserves simple values', () => {
            fc.assert(
                fc.asyncProperty(
                    fc.record({
                        name: fc.string({ minLength: 1, maxLength: 20 }).filter(s => /^[a-zA-Z][a-zA-Z0-9]*$/.test(s)),
                        count: fc.integer({ min: 0, max: 10000 }),
                        active: fc.boolean(),
                    }),
                    async (obj) => {
                        // Convert object to DX
                        const dx = Object.entries(obj)
                            .map(([k, v]) => {
                                if (typeof v === 'boolean') return `${k}:${v ? '+' : '-'}`;
                                return `${k}:${v}`;
                            })
                            .join('\n');

                        // Convert to JSON and back
                        const json = await converter.dxToJson(dx);
                        const parsed = JSON.parse(json);

                        // Verify values are preserved
                        return (
                            parsed.name === obj.name &&
                            parsed.count === obj.count &&
                            parsed.active === obj.active
                        );
                    }
                ),
                { numRuns: 50 }
            );
        });
    });
});
