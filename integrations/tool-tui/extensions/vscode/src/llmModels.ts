/**
 * LLM Model Definitions with Pricing
 * 
 * Comprehensive list of LLM models with their pricing per 1M tokens.
 * Data sourced from official provider pricing pages (January 2026).
 */

export interface LlmModel {
    name: string;
    provider: 'OpenAI/Azure' | 'Anthropic' | 'Google';
    contextWindow: string;  // e.g., "400K", "200K", "1M", "2M"
    inputPer1M: number;     // $ per 1M input tokens
    inputCachedPer1M: number; // $ per 1M cached input tokens
    outputPer1M: number;    // $ per 1M output tokens
}

/**
 * All supported LLM models with current pricing
 * Order: Anthropic (Claude) → Google (Gemini) → OpenAI
 */
export const LLM_MODELS: LlmModel[] = [
    // Anthropic Models (Claude)
    {
        name: 'Claude Opus 4.5',
        provider: 'Anthropic',
        contextWindow: '200K',
        inputPer1M: 5,
        inputCachedPer1M: 0.5,
        outputPer1M: 25,
    },
    {
        name: 'Claude Opus 4',
        provider: 'Anthropic',
        contextWindow: '200K',
        inputPer1M: 15,
        inputCachedPer1M: 1.5,
        outputPer1M: 75,
    },
    {
        name: 'Claude Sonnet 4.5',
        provider: 'Anthropic',
        contextWindow: '200K',
        inputPer1M: 3,
        inputCachedPer1M: 0.3,
        outputPer1M: 15,
    },
    {
        name: 'Claude Sonnet 4',
        provider: 'Anthropic',
        contextWindow: '200K',
        inputPer1M: 3,
        inputCachedPer1M: 0.3,
        outputPer1M: 15,
    },
    {
        name: 'Claude Haiku 4.5',
        provider: 'Anthropic',
        contextWindow: '200K',
        inputPer1M: 1,
        inputCachedPer1M: 0.1,
        outputPer1M: 5,
    },
    {
        name: 'Claude Haiku 3.5',
        provider: 'Anthropic',
        contextWindow: '200K',
        inputPer1M: 0.8,
        inputCachedPer1M: 0.08,
        outputPer1M: 4,
    },

    // Google Models (Gemini)
    {
        name: 'Gemini 3 Pro (Preview)',
        provider: 'Google',
        contextWindow: '1M',
        inputPer1M: 2,
        inputCachedPer1M: 0.2,
        outputPer1M: 12,
    },
    {
        name: 'Gemini 3 Flash (Preview)',
        provider: 'Google',
        contextWindow: '1M',
        inputPer1M: 0.5,
        inputCachedPer1M: 0.05,
        outputPer1M: 3,
    },
    {
        name: 'Gemini 2.5 Pro',
        provider: 'Google',
        contextWindow: '2M',
        inputPer1M: 1.25,
        inputCachedPer1M: 0.125,
        outputPer1M: 10,
    },
    {
        name: 'Gemini 2.5 Flash',
        provider: 'Google',
        contextWindow: '1M',
        inputPer1M: 0.3,
        inputCachedPer1M: 0.03,
        outputPer1M: 2.5,
    },
    {
        name: 'Gemini 2.5 Flash-Lite',
        provider: 'Google',
        contextWindow: '1M',
        inputPer1M: 0.1,
        inputCachedPer1M: 0.01,
        outputPer1M: 0.4,
    },
    {
        name: 'Gemini 2.0 Flash',
        provider: 'Google',
        contextWindow: '1M',
        inputPer1M: 0.1,
        inputCachedPer1M: 0.025,
        outputPer1M: 0.4,
    },
    {
        name: 'Gemini 2.0 Flash-Lite',
        provider: 'Google',
        contextWindow: '1M',
        inputPer1M: 0.075,
        inputCachedPer1M: 0,
        outputPer1M: 0.3,
    },

    // OpenAI/Azure Models
    {
        name: 'GPT-5.2',
        provider: 'OpenAI/Azure',
        contextWindow: '400K',
        inputPer1M: 1.75,
        inputCachedPer1M: 0.175,
        outputPer1M: 14,
    },
    {
        name: 'GPT-5.2 pro',
        provider: 'OpenAI/Azure',
        contextWindow: '400K',
        inputPer1M: 21,
        inputCachedPer1M: 0,
        outputPer1M: 168,
    },
    {
        name: 'GPT-5 mini',
        provider: 'OpenAI/Azure',
        contextWindow: '400K',
        inputPer1M: 0.25,
        inputCachedPer1M: 0.025,
        outputPer1M: 2,
    },
];

/**
 * Character-per-token ratios for different tokenizers (fallback)
 * These are empirically derived averages
 */
export const CHARS_PER_TOKEN: Record<string, number> = {
    'openai': 4.0,      // GPT models (cl100k_base, o200k_base)
    'anthropic': 3.8,   // Claude models
    'google': 4.2,      // Gemini models
};

/**
 * Get tokenizer type for a model
 */
export function getTokenizerType(model: LlmModel): 'openai' | 'anthropic' | 'google' {
    switch (model.provider) {
        case 'OpenAI/Azure': return 'openai';
        case 'Anthropic': return 'anthropic';
        case 'Google': return 'google';
    }
}

// Cache for token counts to avoid re-encoding
let tokenCache: Map<string, Map<string, number>> = new Map();
let lastText: string = '';

/**
 * Clear token cache when text changes
 */
function checkCacheValidity(text: string): void {
    if (text !== lastText) {
        tokenCache.clear();
        lastText = text;
    }
}

/**
 * Estimate token count for text using a specific model (sync fallback)
 */
export function estimateTokens(text: string, model: LlmModel): number {
    checkCacheValidity(text);
    
    const tokenizerType = getTokenizerType(model);
    
    // Check cache first
    if (tokenCache.has(tokenizerType)) {
        const cached = tokenCache.get(tokenizerType)!.get(model.name);
        if (cached !== undefined) return cached;
    }
    
    // Fallback to character-based estimation
    const charsPerToken = CHARS_PER_TOKEN[tokenizerType];
    return Math.max(1, Math.ceil(text.length / charsPerToken));
}

/**
 * Async token counting with real tiktoken (when available)
 */
export async function countTokensAsync(text: string, model: LlmModel): Promise<number> {
    checkCacheValidity(text);
    
    const tokenizerType = getTokenizerType(model);
    
    // Check cache
    if (!tokenCache.has(tokenizerType)) {
        tokenCache.set(tokenizerType, new Map());
    }
    const providerCache = tokenCache.get(tokenizerType)!;
    
    if (providerCache.has(model.name)) {
        return providerCache.get(model.name)!;
    }
    
    // Try to use tiktoken
    try {
        const { countTokens } = await import('./tiktoken');
        const result = await countTokens(text, tokenizerType);
        providerCache.set(model.name, result.tokens);
        return result.tokens;
    } catch {
        // Fallback to estimation
        const tokens = estimateTokens(text, model);
        providerCache.set(model.name, tokens);
        return tokens;
    }
}

/**
 * Calculate cost for a given token count
 */
export function calculateCost(
    tokens: number,
    model: LlmModel,
    type: 'input' | 'inputCached' | 'output'
): number {
    const pricePerMillion = type === 'input' 
        ? model.inputPer1M 
        : type === 'inputCached' 
            ? model.inputCachedPer1M 
            : model.outputPer1M;
    return (tokens / 1_000_000) * pricePerMillion;
}

/**
 * Format cost as string
 */
export function formatCost(cost: number): string {
    if (cost === 0) return '$0.0000';
    if (cost < 0.0001) return '<$0.0001';
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    if (cost < 1) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(2)}`;
}

/**
 * Format token count with exact numbers
 */
export function formatTokenCount(tokens: number): string {
    return tokens.toLocaleString();
}

/**
 * Get models grouped by provider
 */
export function getModelsByProvider(): Map<string, LlmModel[]> {
    const grouped = new Map<string, LlmModel[]>();
    for (const model of LLM_MODELS) {
        const existing = grouped.get(model.provider) || [];
        existing.push(model);
        grouped.set(model.provider, existing);
    }
    return grouped;
}
