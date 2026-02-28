/**
 * Tiktoken Integration for DX Extension
 * 
 * Provides accurate token counting using js-tiktoken library.
 * Falls back to character-based estimation if tiktoken fails.
 * 
 * Supports:
 * - OpenAI: cl100k_base (GPT-4), o200k_base (GPT-4o, o1)
 * - Anthropic: Approximation based on cl100k_base
 * - Google: Approximation based on character ratio
 */

// Type definitions for js-tiktoken
interface TiktokenEncoder {
    encode(text: string): number[];
    decode(tokens: number[]): string;
}

interface TiktokenModule {
    getEncoding(encoding: string): TiktokenEncoder;
}

// We'll use dynamic import to handle the case where tiktoken isn't available
let tiktokenModule: TiktokenModule | null = null;
let encoders: Map<string, TiktokenEncoder> = new Map();
let initPromise: Promise<boolean> | null = null;

/**
 * Initialize tiktoken module
 */
async function initTiktoken(): Promise<boolean> {
    if (tiktokenModule) return true;
    if (initPromise) return initPromise;
    
    initPromise = (async () => {
        try {
            const mod = await import('js-tiktoken');
            tiktokenModule = mod as unknown as TiktokenModule;
            return true;
        } catch (error) {
            console.warn('DX: js-tiktoken not available, using fallback estimation');
            return false;
        }
    })();
    
    return initPromise;
}

/**
 * Get or create an encoder for the specified encoding
 */
function getEncoder(encoding: 'cl100k_base' | 'o200k_base'): TiktokenEncoder | null {
    if (!tiktokenModule) return null;
    
    if (!encoders.has(encoding)) {
        try {
            const encoder = tiktokenModule.getEncoding(encoding);
            encoders.set(encoding, encoder);
        } catch (error) {
            console.warn(`DX: Failed to create ${encoding} encoder:`, error);
            return null;
        }
    }
    
    return encoders.get(encoding) || null;
}

/**
 * Token counting result
 */
export interface TokenCount {
    tokens: number;
    isEstimate: boolean;
    encoding?: string;
}

/**
 * Count tokens using OpenAI's o200k_base encoding (GPT-4o, o1, GPT-5)
 */
export async function countTokensO200k(text: string): Promise<TokenCount> {
    await initTiktoken();
    
    const encoder = getEncoder('o200k_base');
    if (encoder) {
        try {
            const tokens = encoder.encode(text);
            return { tokens: tokens.length, isEstimate: false, encoding: 'o200k_base' };
        } catch (error) {
            console.warn('DX: o200k encoding failed:', error);
        }
    }
    
    // Fallback: ~4 chars per token for GPT models
    return { tokens: Math.ceil(text.length / 4), isEstimate: true };
}

/**
 * Count tokens using OpenAI's cl100k_base encoding (GPT-4, GPT-3.5)
 */
export async function countTokensCl100k(text: string): Promise<TokenCount> {
    await initTiktoken();
    
    const encoder = getEncoder('cl100k_base');
    if (encoder) {
        try {
            const tokens = encoder.encode(text);
            return { tokens: tokens.length, isEstimate: false, encoding: 'cl100k_base' };
        } catch (error) {
            console.warn('DX: cl100k encoding failed:', error);
        }
    }
    
    // Fallback: ~4 chars per token
    return { tokens: Math.ceil(text.length / 4), isEstimate: true };
}

/**
 * Estimate tokens for Anthropic Claude models
 * Claude uses a similar BPE tokenizer to GPT, slightly more efficient
 */
export async function countTokensClaude(text: string): Promise<TokenCount> {
    // Try using cl100k as approximation (Claude is similar)
    const cl100k = await countTokensCl100k(text);
    
    if (!cl100k.isEstimate) {
        // Claude is typically ~5% more efficient than GPT-4
        return { 
            tokens: Math.ceil(cl100k.tokens * 0.95), 
            isEstimate: false, 
            encoding: 'claude-approx' 
        };
    }
    
    // Fallback: ~3.8 chars per token for Claude
    return { tokens: Math.ceil(text.length / 3.8), isEstimate: true };
}

/**
 * Estimate tokens for Google Gemini models
 * Gemini uses SentencePiece, typically less efficient than BPE
 */
export async function countTokensGemini(text: string): Promise<TokenCount> {
    // Gemini is typically ~10% less efficient than GPT-4
    const cl100k = await countTokensCl100k(text);
    
    if (!cl100k.isEstimate) {
        return { 
            tokens: Math.ceil(cl100k.tokens * 1.1), 
            isEstimate: false, 
            encoding: 'gemini-approx' 
        };
    }
    
    // Fallback: ~4.2 chars per token for Gemini
    return { tokens: Math.ceil(text.length / 4.2), isEstimate: true };
}

/**
 * Provider type for token counting
 */
export type TokenProvider = 'openai' | 'anthropic' | 'google';

/**
 * Count tokens for a specific provider
 */
export async function countTokens(text: string, provider: TokenProvider): Promise<TokenCount> {
    switch (provider) {
        case 'openai':
            return countTokensO200k(text);
        case 'anthropic':
            return countTokensClaude(text);
        case 'google':
            return countTokensGemini(text);
        default:
            return countTokensO200k(text);
    }
}

/**
 * Count tokens for all providers at once
 */
export async function countAllTokens(text: string): Promise<{
    openai: TokenCount;
    anthropic: TokenCount;
    google: TokenCount;
}> {
    const [openai, anthropic, google] = await Promise.all([
        countTokensO200k(text),
        countTokensClaude(text),
        countTokensGemini(text),
    ]);
    
    return { openai, anthropic, google };
}

/**
 * Check if tiktoken is available
 */
export async function isTiktokenAvailable(): Promise<boolean> {
    return initTiktoken();
}

/**
 * Get encoding info string
 */
export function getEncodingInfo(count: TokenCount): string {
    if (count.isEstimate) {
        return 'estimated (~4 chars/token)';
    }
    return count.encoding || 'unknown';
}

