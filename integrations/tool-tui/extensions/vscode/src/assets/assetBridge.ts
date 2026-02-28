/**
 * Asset Bridge for DX Unified Assets
 *
 * Provides communication layer between VS Code extension and dx CLI
 * for asset operations (icons, fonts, media).
 *
 * @module assets/assetBridge
 */

import { spawn } from 'child_process';
import * as path from 'path';
import {
    IconAsset,
    FontAsset,
    MediaAsset,
    AssetSamples,
    MediaTool,
    CLIResponse,
    isErrorResponse,
    MediaType,
} from './assetTypes';

/**
 * Error thrown by AssetBridge operations.
 */
export class AssetBridgeError extends Error {
    constructor(
        message: string,
        public code: string,
        public hint?: string
    ) {
        super(message);
        this.name = 'AssetBridgeError';
    }
}

/**
 * Error codes for AssetBridge operations.
 */
export const ErrorCodes = {
    /** dx CLI binary not found in PATH */
    CLI_NOT_FOUND: 'CLI_NOT_FOUND',
    /** dx CLI version is incompatible */
    CLI_VERSION_MISMATCH: 'CLI_VERSION_MISMATCH',
    /** Failed to parse CLI output */
    PARSE_ERROR: 'PARSE_ERROR',
    /** CLI command timed out */
    TIMEOUT: 'TIMEOUT',
    /** Requested asset not found */
    ASSET_NOT_FOUND: 'ASSET_NOT_FOUND',
    /** Network error during operation */
    NETWORK_ERROR: 'NETWORK_ERROR',
} as const;

/**
 * Cache entry with timestamp.
 */
interface CacheEntry<T> {
    data: T;
    timestamp: number;
}

/**
 * Options for AssetBridge constructor.
 */
export interface AssetBridgeOptions {
    /** Path to dx CLI binary (default: 'dx') */
    dxPath?: string;
    /** Cache TTL in milliseconds (default: 5 minutes) */
    cacheTTL?: number;
    /** Command timeout in milliseconds (default: 30 seconds) */
    timeout?: number;
}

/**
 * Search options for icons.
 */
export interface IconSearchOptions {
    /** Maximum number of results */
    limit?: number;
    /** Filter by icon set prefix */
    prefix?: string;
}

/**
 * Search options for fonts.
 */
export interface FontSearchOptions {
    /** Maximum number of results */
    limit?: number;
    /** Filter by provider */
    provider?: string;
    /** Filter by category */
    category?: string;
}

/**
 * Search options for media.
 */
export interface MediaSearchOptions {
    /** Maximum number of results */
    limit?: number;
    /** Filter by media type */
    type?: MediaType;
    /** Filter by provider */
    provider?: string;
}

/**
 * Bridge between VS Code extension and dx CLI for asset operations.
 *
 * Provides methods to search, retrieve, and manage icons, fonts, and media
 * through the dx CLI with caching support.
 */
export class AssetBridge {
    private cache: Map<string, CacheEntry<unknown>> = new Map();
    private dxPath: string;
    private cacheTTL: number;
    private timeout: number;

    constructor(options: AssetBridgeOptions = {}) {
        this.dxPath = options.dxPath ?? 'dx';
        this.cacheTTL = options.cacheTTL ?? 300000; // 5 minutes
        this.timeout = options.timeout ?? 30000; // 30 seconds
    }

    /**
     * Check if dx CLI is available and return version info.
     * @returns Version string if available
     * @throws AssetBridgeError if CLI not found
     */
    async checkCLIAvailability(): Promise<string> {
        try {
            const result = await this.execDx(['--version']);
            return result.trim();
        } catch (error) {
            throw new AssetBridgeError(
                'dx CLI not found. Please ensure dx is installed and in your PATH.',
                ErrorCodes.CLI_NOT_FOUND,
                'Install dx CLI or add it to your PATH'
            );
        }
    }

    /**
     * Search for icons across all icon sets.
     */
    async searchIcons(query: string, options: IconSearchOptions = {}): Promise<IconAsset[]> {
        const args = ['icon', 'search', query, '--format', 'json'];
        if (options.limit) {
            args.push('--limit', options.limit.toString());
        }
        if (options.prefix) {
            args.push('--prefix', options.prefix);
        }

        const response = await this.execDxJson<CLIResponse<IconAsset>>(args);
        if (isErrorResponse(response)) {
            throw new AssetBridgeError(response.error, response.code, response.hint);
        }
        return response.results ?? [];
    }

    /**
     * Get a specific icon by prefix and ID.
     */
    async getIcon(prefix: string, id: string): Promise<IconAsset | null> {
        const cacheKey = `icon:${prefix}:${id}`;
        const cached = this.getCached<IconAsset>(cacheKey);
        if (cached) {
            return cached;
        }

        const args = ['icon', 'get', prefix, id, '--format', 'json'];
        const response = await this.execDxJson<CLIResponse<IconAsset>>(args);
        if (isErrorResponse(response)) {
            if (response.code === 'NOT_FOUND') {
                return null;
            }
            throw new AssetBridgeError(response.error, response.code, response.hint);
        }

        const icon = response.result ?? null;
        if (icon) {
            this.setCache(cacheKey, icon);
        }
        return icon;
    }

    /**
     * Search for fonts across all providers.
     */
    async searchFonts(query: string, options: FontSearchOptions = {}): Promise<FontAsset[]> {
        const args = ['font', 'search', query, '--format', 'json'];
        if (options.limit) {
            args.push('--limit', options.limit.toString());
        }
        if (options.provider) {
            args.push('--provider', options.provider);
        }
        if (options.category) {
            args.push('--category', options.category);
        }

        const response = await this.execDxJson<CLIResponse<FontAsset>>(args);
        if (isErrorResponse(response)) {
            throw new AssetBridgeError(response.error, response.code, response.hint);
        }
        return response.results ?? [];
    }

    /**
     * Get detailed information about a font.
     */
    async getFontInfo(fontId: string, provider: string): Promise<FontAsset | null> {
        const cacheKey = `font:${provider}:${fontId}`;
        const cached = this.getCached<FontAsset>(cacheKey);
        if (cached) {
            return cached;
        }

        const args = ['font', 'info', fontId, '--provider', provider, '--format', 'json'];
        const response = await this.execDxJson<CLIResponse<FontAsset>>(args);
        if (isErrorResponse(response)) {
            if (response.code === 'NOT_FOUND') {
                return null;
            }
            throw new AssetBridgeError(response.error, response.code, response.hint);
        }

        const font = response.result ?? null;
        if (font) {
            this.setCache(cacheKey, font);
        }
        return font;
    }

    /**
     * Search for media assets.
     */
    async searchMedia(query: string, options: MediaSearchOptions = {}): Promise<MediaAsset[]> {
        const args = ['media', 'search', query, '--format', 'json'];
        if (options.limit) {
            args.push('--limit', options.limit.toString());
        }
        if (options.type) {
            args.push('--type', options.type);
        }
        if (options.provider) {
            args.push('--provider', options.provider);
        }

        const response = await this.execDxJson<CLIResponse<MediaAsset>>(args);
        if (isErrorResponse(response)) {
            throw new AssetBridgeError(response.error, response.code, response.hint);
        }
        return response.results ?? [];
    }

    /**
     * Get sample assets for UI previews.
     */
    async getSamples(): Promise<AssetSamples> {
        const cacheKey = 'samples:all';
        const cached = this.getCached<AssetSamples>(cacheKey);
        if (cached) {
            return cached;
        }

        // Fetch samples from each asset type in parallel
        const [iconResult, fontResult, mediaResult] = await Promise.all([
            this.execDxJson<CLIResponse<IconAsset>>(['icon', 'samples', '--format', 'json']),
            this.execDxJson<CLIResponse<FontAsset>>(['font', 'samples', '--format', 'json']),
            this.execDxJson<CLIResponse<MediaAsset>>(['media', 'samples', '--format', 'json']),
        ]);

        const samples: AssetSamples = {
            icons: isErrorResponse(iconResult) ? [] : (iconResult.results ?? []),
            fonts: isErrorResponse(fontResult) ? [] : (fontResult.results ?? []),
            media: isErrorResponse(mediaResult) ? [] : (mediaResult.results ?? []),
            generatedAt: new Date().toISOString(),
        };

        this.setCache(cacheKey, samples);
        return samples;
    }

    /**
     * Get available media processing tools.
     */
    async getMediaTools(mediaType?: MediaType): Promise<MediaTool[]> {
        const cacheKey = `tools:${mediaType ?? 'all'}`;
        const cached = this.getCached<MediaTool[]>(cacheKey);
        if (cached) {
            return cached;
        }

        const args = ['media', 'tools', '--format', 'json'];
        if (mediaType) {
            args.push('--type', mediaType);
        }

        const response = await this.execDxJson<CLIResponse<MediaTool>>(args);
        if (isErrorResponse(response)) {
            throw new AssetBridgeError(response.error, response.code, response.hint);
        }

        const tools = response.results ?? [];
        this.setCache(cacheKey, tools);
        return tools;
    }

    /**
     * Generate framework-specific component code for an icon.
     */
    async generateIconComponent(
        prefix: string,
        id: string,
        framework: string,
        typescript: boolean = false
    ): Promise<string> {
        const args = ['icon', 'component', prefix, id, '--framework', framework];
        if (typescript) {
            args.push('--typescript');
        }

        return await this.execDx(args);
    }

    /**
     * Download a media asset to the specified directory.
     */
    async downloadMedia(assetId: string, outputDir: string): Promise<string> {
        const args = ['media', 'download', assetId, '--output', outputDir];
        const result = await this.execDx(args);
        // Parse the output to get the downloaded file path
        const match = result.match(/Output: (.+)/);
        return match ? match[1].trim() : path.join(outputDir, 'downloaded');
    }

    /**
     * Clear the cache.
     */
    clearCache(): void {
        this.cache.clear();
    }

    /**
     * Set cache TTL.
     */
    setCacheTTL(ttl: number): void {
        this.cacheTTL = ttl;
    }

    /**
     * Get cached value if not expired.
     */
    private getCached<T>(key: string): T | null {
        const entry = this.cache.get(key) as CacheEntry<T> | undefined;
        if (!entry) {
            return null;
        }

        const now = Date.now();
        if (now - entry.timestamp > this.cacheTTL) {
            this.cache.delete(key);
            return null;
        }

        return entry.data;
    }

    /**
     * Set cache value.
     */
    private setCache(key: string, data: unknown): void {
        this.cache.set(key, {
            data,
            timestamp: Date.now(),
        });
    }

    /**
     * Execute dx CLI command and return raw output.
     */
    private execDx(args: string[]): Promise<string> {
        return new Promise((resolve, reject) => {
            const proc = spawn(this.dxPath, args, {
                timeout: this.timeout,
            });

            let stdout = '';
            let stderr = '';

            proc.stdout.on('data', (data) => {
                stdout += data.toString();
            });

            proc.stderr.on('data', (data) => {
                stderr += data.toString();
            });

            proc.on('close', (code) => {
                if (code === 0) {
                    resolve(stdout);
                } else {
                    reject(new AssetBridgeError(
                        stderr || `Command failed with exit code ${code}`,
                        'CLI_ERROR'
                    ));
                }
            });

            proc.on('error', (error) => {
                if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
                    reject(new AssetBridgeError(
                        'dx CLI not found',
                        ErrorCodes.CLI_NOT_FOUND,
                        'Ensure dx is installed and in your PATH'
                    ));
                } else {
                    reject(new AssetBridgeError(error.message, 'CLI_ERROR'));
                }
            });
        });
    }

    /**
     * Execute dx CLI command and parse JSON output.
     */
    private async execDxJson<T>(args: string[]): Promise<T> {
        const output = await this.execDx(args);
        try {
            return JSON.parse(output) as T;
        } catch (error) {
            throw new AssetBridgeError(
                'Failed to parse CLI output as JSON',
                ErrorCodes.PARSE_ERROR
            );
        }
    }
}
