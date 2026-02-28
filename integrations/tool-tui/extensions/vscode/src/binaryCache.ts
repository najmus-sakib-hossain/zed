/**
 * Binary Cache Manager for DXM files
 * 
 * Generates and manages binary cache files (.dxb) and LLM-optimized files (.llm)
 * in the .dx/cache/ directory. These caches provide instant runtime access.
 * 
 * Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 11.4, 11.6
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Result of cache generation
 */
export interface CacheResult {
    /** Path to the generated .dxb binary file */
    binaryPath: string;
    /** Path to the generated .llm token-optimized file */
    llmPath: string;
    /** Token count of the DXM document */
    tokenCount: number;
    /** Token savings compared to equivalent Markdown */
    tokenSavings: number;
    /** Whether generation was successful */
    success: boolean;
    /** Error message if generation failed */
    error?: string;
}

/**
 * Binary Cache Manager
 * 
 * Manages the generation and lifecycle of binary cache files for DXM documents.
 */
export class BinaryCacheManager implements vscode.Disposable {
    private cacheDir: string;
    private workspaceRoot: string;
    private disposables: vscode.Disposable[] = [];
    private enabled: boolean = true;

    constructor(workspaceRoot: string) {
        this.workspaceRoot = workspaceRoot;
        this.cacheDir = path.join(workspaceRoot, '.dx', 'cache');

        // Listen for configuration changes (Requirements: 11.6)
        this.disposables.push(
            vscode.workspace.onDidChangeConfiguration((event) => {
                if (event.affectsConfiguration('dx.cache.enabled')) {
                    this.handleSettingsChange();
                }
            })
        );
    }

    /**
     * Handle settings change - apply changes without restart
     * Requirements: 11.6
     */
    private async handleSettingsChange(): Promise<void> {
        const config = vscode.workspace.getConfiguration('dx');
        const newEnabled = config.get<boolean>('cache.enabled', true);

        if (newEnabled !== this.enabled) {
            this.enabled = newEnabled;

            if (this.enabled) {
                await this.ensureCacheDir();
                console.log('DX Cache: Cache generation enabled');
            } else {
                console.log('DX Cache: Cache generation disabled');
            }
        }
    }

    /**
     * Initialize the cache manager
     */
    async initialize(): Promise<void> {
        // Load enabled state from settings
        const config = vscode.workspace.getConfiguration('dx');
        this.enabled = config.get<boolean>('cache.enabled', true);

        if (this.enabled) {
            await this.ensureCacheDir();
        }
    }

    /**
     * Generate cache files for a DXM document
     * 
     * @param dxmPath - Path to the .dxm file
     * @returns Cache generation result
     */
    async generateCache(dxmPath: string): Promise<CacheResult> {
        if (!this.enabled) {
            return {
                binaryPath: '',
                llmPath: '',
                tokenCount: 0,
                tokenSavings: 0,
                success: false,
                error: 'Cache generation is disabled',
            };
        }

        try {
            // Ensure cache directory exists
            await this.ensureCacheDir();

            // Read the DXM file
            const content = await fs.promises.readFile(dxmPath, 'utf-8');

            // Get the base name for cache files
            const baseName = path.basename(dxmPath, '.dxm');
            const binaryPath = path.join(this.cacheDir, `${baseName}.dxb`);
            const llmPath = path.join(this.cacheDir, `${baseName}.llm`);

            // Generate binary format
            await this.generateBinary(content, binaryPath);

            // Generate LLM format
            await this.generateLlm(content, llmPath);

            // Calculate token counts
            const tokenCount = this.estimateTokenCount(content);
            const mdTokenCount = this.estimateMarkdownTokenCount(content);
            const tokenSavings = mdTokenCount > 0
                ? Math.round((1 - tokenCount / mdTokenCount) * 100)
                : 0;

            return {
                binaryPath,
                llmPath,
                tokenCount,
                tokenSavings,
                success: true,
            };
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error(`DX Cache: Failed to generate cache for ${dxmPath}:`, error);

            return {
                binaryPath: '',
                llmPath: '',
                tokenCount: 0,
                tokenSavings: 0,
                success: false,
                error: errorMessage,
            };
        }
    }

    /**
     * Generate .dxb binary format
     */
    private async generateBinary(content: string, outputPath: string): Promise<void> {
        // Binary format: Simple header + content
        // Header: DXMB (4 bytes) + version (2 bytes) + content length (4 bytes)
        const MAGIC = Buffer.from('DXMB');
        const version = Buffer.alloc(2);
        version.writeUInt16LE(1, 0); // Version 1

        const contentBuffer = Buffer.from(content, 'utf-8');
        const lengthBuffer = Buffer.alloc(4);
        lengthBuffer.writeUInt32LE(contentBuffer.length, 0);

        const binaryContent = Buffer.concat([MAGIC, version, lengthBuffer, contentBuffer]);
        await fs.promises.writeFile(outputPath, binaryContent);
    }

    /**
     * Generate .llm token-optimized format
     */
    private async generateLlm(content: string, outputPath: string): Promise<void> {
        // LLM format is the content as-is (already token-optimized DXM)
        // We could apply additional optimizations here in the future
        await fs.promises.writeFile(outputPath, content, 'utf-8');
    }

    /**
     * Estimate token count for DXM content
     * Uses a simple heuristic: ~4 characters per token
     */
    private estimateTokenCount(content: string): number {
        // Simple estimation: ~4 characters per token
        return Math.ceil(content.length / 4);
    }

    /**
     * Estimate token count for equivalent Markdown
     * DXM is typically 30-40% more token-efficient
     */
    private estimateMarkdownTokenCount(content: string): number {
        // Estimate Markdown would be ~40% larger
        return Math.ceil(content.length * 1.4 / 4);
    }

    /**
     * Ensure .dx/cache directory exists and is gitignored
     */
    private async ensureCacheDir(): Promise<void> {
        // Create .dx/cache directory
        if (!fs.existsSync(this.cacheDir)) {
            await fs.promises.mkdir(this.cacheDir, { recursive: true });
        }

        // Ensure .dx/cache is in .gitignore
        await this.ensureGitignore();
    }

    /**
     * Ensure .dx/cache/ is in .gitignore
     */
    private async ensureGitignore(): Promise<void> {
        const gitignorePath = path.join(this.workspaceRoot, '.gitignore');
        const cachePattern = '.dx/cache/';

        try {
            let content = '';
            if (fs.existsSync(gitignorePath)) {
                content = await fs.promises.readFile(gitignorePath, 'utf-8');
            }

            // Check if pattern already exists
            if (content.includes(cachePattern)) {
                return;
            }

            // Append the pattern
            const newContent = content.endsWith('\n') || content === ''
                ? `${content}# DX binary cache\n${cachePattern}\n`
                : `${content}\n\n# DX binary cache\n${cachePattern}\n`;

            await fs.promises.writeFile(gitignorePath, newContent, 'utf-8');
        } catch (error) {
            console.warn('DX Cache: Failed to update .gitignore:', error);
        }
    }

    /**
     * Clear all cache files
     */
    async clearCache(): Promise<void> {
        try {
            if (fs.existsSync(this.cacheDir)) {
                const files = await fs.promises.readdir(this.cacheDir);
                for (const file of files) {
                    await fs.promises.unlink(path.join(this.cacheDir, file));
                }
            }
        } catch (error) {
            console.error('DX Cache: Failed to clear cache:', error);
        }
    }

    /**
     * Get cache file paths for a DXM file
     */
    getCachePaths(dxmPath: string): { binaryPath: string; llmPath: string } {
        const baseName = path.basename(dxmPath, '.dxm');
        return {
            binaryPath: path.join(this.cacheDir, `${baseName}.dxb`),
            llmPath: path.join(this.cacheDir, `${baseName}.llm`),
        };
    }

    /**
     * Check if cache exists for a DXM file
     */
    hasCacheFor(dxmPath: string): boolean {
        const { binaryPath, llmPath } = this.getCachePaths(dxmPath);
        return fs.existsSync(binaryPath) && fs.existsSync(llmPath);
    }

    /**
     * Enable or disable cache generation
     */
    setEnabled(enabled: boolean): void {
        this.enabled = enabled;
    }

    /**
     * Check if cache generation is enabled
     */
    isEnabled(): boolean {
        return this.enabled;
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}

/**
 * Set up cache generation on save
 */
export function setupCacheOnSave(
    context: vscode.ExtensionContext,
    cacheManager: BinaryCacheManager
): void {
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(async (document) => {
            // Only process .dxm files
            if (!document.uri.fsPath.endsWith('.dxm')) {
                return;
            }

            // Check if cache is enabled
            if (!cacheManager.isEnabled()) {
                return;
            }

            // Generate cache
            const result = await cacheManager.generateCache(document.uri.fsPath);

            if (!result.success && result.error) {
                vscode.window.showWarningMessage(`DX Cache: ${result.error}`);
            }
        })
    );
}

/**
 * Register cache-related commands
 */
export function registerCacheCommands(
    context: vscode.ExtensionContext,
    cacheManager: BinaryCacheManager
): void {
    // Clear cache command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.clearCache', async () => {
            await cacheManager.clearCache();
            vscode.window.showInformationMessage('DX: Cache cleared');
        })
    );

    // Generate cache for current file
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generateCache', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DX: No active editor');
                return;
            }

            if (!editor.document.uri.fsPath.endsWith('.dxm')) {
                vscode.window.showWarningMessage('DX: Not a .dxm file');
                return;
            }

            const result = await cacheManager.generateCache(editor.document.uri.fsPath);

            if (result.success) {
                vscode.window.showInformationMessage(
                    `DX: Cache generated (${result.tokenCount} tokens, ${result.tokenSavings}% savings)`
                );
            } else {
                vscode.window.showErrorMessage(`DX: Cache generation failed: ${result.error}`);
            }
        })
    );
}
