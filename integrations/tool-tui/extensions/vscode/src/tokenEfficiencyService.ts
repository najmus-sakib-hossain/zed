/**
 * Token Efficiency Service
 * 
 * Provides comprehensive token counting and efficiency metrics for DX files.
 * Uses WASM bindings when available, with fallback to character-based estimation.
 * 
 * Requirements: 1.1, 1.2, 1.3, 1.4, 4.1, 4.2, 4.4
 */

import { FormatConverterService } from './formatConverterService';

/**
 * Token information for a single model
 */
export interface TokenInfo {
    count: number;
    ids: number[];
    model: string;
}

/**
 * Token counts for all primary models
 */
export interface MultiModelTokenCounts {
    openai: TokenInfo;
    claude: TokenInfo;
    gemini: TokenInfo;
    other: TokenInfo;
}

/**
 * Format equivalents for comparison
 */
export interface FormatEquivalents {
    dx: string;
    json: string;
    yaml: string;
    toml: string;
    toon: string;
}

/**
 * Complete efficiency report
 */
export interface EfficiencyReport {
    dxTokens: MultiModelTokenCounts;
    jsonTokens: MultiModelTokenCounts;
    yamlTokens: MultiModelTokenCounts;
    tomlTokens: MultiModelTokenCounts;
    toonTokens: MultiModelTokenCounts;
    savings: {
        vsJson: number;
        vsYaml: number;
        vsToml: number;
        vsToon: number;
    };
    equivalents: FormatEquivalents;
}

/**
 * WASM token counting interface (when available)
 */
interface WasmTokenCounter {
    count_tokens(text: string, model: string): { count: number; model: string };
    count_tokens_all(text: string): { gpt4o: number; claude: number; gemini: number; other: number };
}

/**
 * Token Efficiency Service
 * 
 * Coordinates token counting and format conversion for efficiency analysis.
 */
export class TokenEfficiencyService {
    private wasmCounter: WasmTokenCounter | null = null;
    private formatConverter: FormatConverterService;

    // Fallback character-per-token ratios
    private static readonly CHARS_PER_TOKEN = {
        openai: 4.0,
        claude: 3.8,
        gemini: 3.5,
        other: 3.7,
    };

    constructor(formatConverter?: FormatConverterService) {
        this.formatConverter = formatConverter || new FormatConverterService();
    }

    /**
     * Initialize WASM token counter if available
     */
    async initWasm(wasmModule: any): Promise<void> {
        if (wasmModule && typeof wasmModule.count_tokens === 'function') {
            this.wasmCounter = wasmModule;
        }
    }

    /**
     * Count tokens for all models
     * 
     * @param content - The text content to tokenize
     * @returns Token counts for all 4 primary models
     */
    countTokens(content: string): MultiModelTokenCounts {
        if (this.wasmCounter) {
            try {
                const counts = this.wasmCounter.count_tokens_all(content);
                return {
                    openai: { count: counts.gpt4o, ids: [], model: 'GPT-4o' },
                    claude: { count: counts.claude, ids: [], model: 'Claude Sonnet 4' },
                    gemini: { count: counts.gemini, ids: [], model: 'Gemini 3' },
                    other: { count: counts.other, ids: [], model: 'Other' },
                };
            } catch (error) {
                console.warn('WASM token counting failed, using fallback:', error);
            }
        }

        // Fallback: character-based estimation
        return this.estimateTokenCounts(content);
    }

    /**
     * Estimate token counts using character-based heuristics
     */
    private estimateTokenCounts(content: string): MultiModelTokenCounts {
        const len = content.length;
        return {
            openai: {
                count: Math.max(1, Math.ceil(len / TokenEfficiencyService.CHARS_PER_TOKEN.openai)),
                ids: [],
                model: 'GPT-4o',
            },
            claude: {
                count: Math.max(1, Math.ceil(len / TokenEfficiencyService.CHARS_PER_TOKEN.claude)),
                ids: [],
                model: 'Claude Sonnet 4',
            },
            gemini: {
                count: Math.max(1, Math.ceil(len / TokenEfficiencyService.CHARS_PER_TOKEN.gemini)),
                ids: [],
                model: 'Gemini 3',
            },
            other: {
                count: Math.max(1, Math.ceil(len / TokenEfficiencyService.CHARS_PER_TOKEN.other)),
                ids: [],
                model: 'Other',
            },
        };
    }

    /**
     * Generate all format equivalents for a DX document
     * 
     * @param dxContent - The DX format content
     * @returns Equivalent representations in all formats
     */
    async generateEquivalents(dxContent: string): Promise<FormatEquivalents> {
        return {
            dx: dxContent,
            json: await this.formatConverter.dxToJson(dxContent),
            yaml: await this.formatConverter.dxToYaml(dxContent),
            toml: await this.formatConverter.dxToToml(dxContent),
            toon: await this.formatConverter.dxToToon(dxContent),
        };
    }

    /**
     * Calculate savings percentage between two token counts
     * 
     * @param dxTokens - Token count for DX format
     * @param otherTokens - Token count for comparison format
     * @returns Savings percentage (positive = DX is more efficient)
     */
    calculateSavings(dxTokens: number, otherTokens: number): number {
        if (otherTokens <= 0) {
            return 0;
        }
        const savings = ((otherTokens - dxTokens) / otherTokens) * 100;
        return Math.round(savings * 10) / 10; // Round to 1 decimal place
    }

    /**
     * Get complete efficiency report for DX content
     * 
     * @param dxContent - The DX format content
     * @returns Complete efficiency report with all metrics
     */
    async getEfficiencyReport(dxContent: string): Promise<EfficiencyReport> {
        // Generate equivalents
        const equivalents = await this.generateEquivalents(dxContent);

        // Count tokens for all formats
        const dxTokens = this.countTokens(equivalents.dx);
        const jsonTokens = this.countTokens(equivalents.json);
        const yamlTokens = this.countTokens(equivalents.yaml);
        const tomlTokens = this.countTokens(equivalents.toml);
        const toonTokens = this.countTokens(equivalents.toon);

        // Calculate savings (using OpenAI as reference model)
        const dxCount = dxTokens.openai.count;
        const savings = {
            vsJson: this.calculateSavings(dxCount, jsonTokens.openai.count),
            vsYaml: this.calculateSavings(dxCount, yamlTokens.openai.count),
            vsToml: this.calculateSavings(dxCount, tomlTokens.openai.count),
            vsToon: this.calculateSavings(dxCount, toonTokens.openai.count),
        };

        return {
            dxTokens,
            jsonTokens,
            yamlTokens,
            tomlTokens,
            toonTokens,
            savings,
            equivalents,
        };
    }

    /**
     * Get a summary string for display
     */
    getSummary(report: EfficiencyReport): string {
        const dxCount = report.dxTokens.openai.count;
        const bestSavings = Math.max(
            report.savings.vsJson,
            report.savings.vsYaml,
            report.savings.vsToml,
            report.savings.vsToon
        );

        return `${dxCount} tokens (${bestSavings > 0 ? '+' : ''}${bestSavings}% vs best)`;
    }
}
