/**
 * Markdown Lens File System Provider
 * 
 * ⚠️ DEPRECATED: Virtual file system is now commented out.
 * New architecture (2026):
 * - Front-facing .md files: Human format (on disk)
 * - .dx/markdown/*.llm: LLM format (token-optimized)
 * - .dx/markdown/*.machine: Machine format (binary)
 * 
 * This code is preserved for reference but not currently active.
 * 
 * OLD BEHAVIOR:
 * Provides holographic view for .md files:
 * - On disk: LLM format (token-optimized) in .md file
 * - In editor: Human format shown via virtual file system from .dx/markdown/{path}.human
 * - Machine format: Binary format saved to .dx/markdown/{path}.machine
 * 
 * When opening or saving .md files, the extension automatically generates:
 * 1. .human file (source of truth, readable format)
 * 2. .machine file (binary format for efficient storage/transmission)
 * 
 * Uses dx-markdown WASM for proper Human ↔ LLM ↔ Machine conversion.
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

const MARKDOWN_HUMAN_SCHEME = 'markdownhuman';

// Shared set to track files being written to prevent circular updates
export const writingMarkdownFiles = new Set<string>();

// Simple cache to prevent "Try Again" errors
const documentCache = new Map<string, { content: string; timestamp: number }>();
const CACHE_TTL = 5000; // 5 seconds

// ============================================================================
// WASM Markdown Converter
// ============================================================================

let wasmMarkdown: any = null;
let wasmMarkdownInitialized = false;

export async function initMarkdownWasm(extensionPath: string): Promise<void> {
    if (wasmMarkdownInitialized) return;
    
    try {
        // Convert Windows path to file:// URL for ESM import
        const wasmPath = path.join(extensionPath, 'wasm-markdown', 'markdown.js');
        const wasmUrl = new URL(`file:///${wasmPath.replace(/\\/g, '/')}`).href;
        
        console.log('dx-markdown: Loading WASM from:', wasmUrl);
        const wasm = await import(wasmUrl);
        
        // Initialize WASM
        const wasmBinaryPath = path.join(extensionPath, 'wasm-markdown', 'markdown_bg.wasm');
        const wasmBinary = await fs.promises.readFile(wasmBinaryPath);
        await wasm.default(wasmBinary);
        
        wasmMarkdown = wasm;
        wasmMarkdownInitialized = true;
        console.log('dx-markdown: WASM markdown converter initialized');
    } catch (e) {
        console.error('dx-markdown: Failed to initialize WASM:', e);
    }
}

// ============================================================================
// LLM ↔ Human Conversion
// ============================================================================

function llmToHuman(llmContent: string): string {
    if (!llmContent.trim()) return '';
    
    console.log('dx-markdown: llmToHuman called, content length:', llmContent.length);
    console.log('dx-markdown: WASM initialized:', wasmMarkdownInitialized);
    console.log('dx-markdown: wasmMarkdown exists:', !!wasmMarkdown);
    
    if (wasmMarkdown && wasmMarkdown.llm_to_human) {
        try {
            console.log('dx-markdown: Calling WASM llm_to_human...');
            const result = wasmMarkdown.llm_to_human(llmContent);
            console.log('dx-markdown: WASM result length:', result.length);
            console.log('dx-markdown: First 200 chars:', result.substring(0, 200));
            return result;
        } catch (e) {
            console.error('dx-markdown: WASM llm_to_human failed:', e);
            return llmContent;
        }
    }
    
    console.warn('dx-markdown: WASM not available, returning content as-is');
    return llmContent;
}

function humanToLlm(humanContent: string): string {
    if (!humanContent.trim()) return '';
    
    console.log('dx-markdown: humanToLlm called, WASM available:', !!wasmMarkdown);
    console.log('dx-markdown: human_to_llm function exists:', !!(wasmMarkdown && wasmMarkdown.human_to_llm));
    
    if (wasmMarkdown && wasmMarkdown.human_to_llm) {
        try {
            console.log('dx-markdown: Calling WASM human_to_llm...');
            const result = wasmMarkdown.human_to_llm(humanContent);
            console.log('dx-markdown: WASM conversion successful, input length:', humanContent.length, 'output length:', result.length);
            return result;
        } catch (e) {
            console.error('dx-markdown: WASM human_to_llm failed:', e);
            console.error('dx-markdown: Returning content unchanged due to error');
        }
    } else {
        console.warn('dx-markdown: WASM not available, returning content unchanged');
    }
    
    // Fallback: return as-is if WASM not available
    return humanContent;
}

// ============================================================================
// Markdown Human File System Provider
// ============================================================================

export class MarkdownHumanFileSystem implements vscode.FileSystemProvider {
    private _onDidChangeFile = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
    readonly onDidChangeFile = this._onDidChangeFile.event;

    watch(): vscode.Disposable {
        return new vscode.Disposable(() => {});
    }

    async stat(uri: vscode.Uri): Promise<vscode.FileStat> {
        const realPath = this.getRealPath(uri);
        
        try {
            const stats = await fs.promises.stat(realPath);
            return {
                type: vscode.FileType.File,
                ctime: stats.ctimeMs,
                mtime: stats.mtimeMs,
                size: stats.size,
            };
        } catch (error: any) {
            console.error('dx-markdown: Failed to stat file:', realPath, error);
            
            // NEVER throw - always return a valid stat
            // This prevents "Try Again" errors
            return {
                type: vscode.FileType.File,
                ctime: Date.now(),
                mtime: Date.now(),
                size: 0,
            };
        }
    }

    async readDirectory(): Promise<[string, vscode.FileType][]> {
        return [];
    }

    async createDirectory(): Promise<void> {}

    async readFile(uri: vscode.Uri): Promise<Uint8Array> {
        const realPath = this.getRealPath(uri);
        const key = realPath.toLowerCase();
        
        try {
            // Check if file is currently being written - wait with retries
            let waitAttempts = 0;
            while (writingMarkdownFiles.has(key) && waitAttempts < 10) {
                console.log(`dx-markdown: File is being written (attempt ${waitAttempts + 1}/10), waiting...`);
                await new Promise(resolve => setTimeout(resolve, 200));
                waitAttempts++;
            }
            
            if (writingMarkdownFiles.has(key)) {
                console.warn('dx-markdown: File still being written after 2 seconds');
                // Return cached content if available
                const cached = documentCache.get(key);
                if (cached) {
                    console.log('dx-markdown: Returning cached content (file being written)');
                    return new TextEncoder().encode(cached.content);
                }
            }
            
            // Check cache first
            const cached = documentCache.get(key);
            if (cached && (Date.now() - cached.timestamp) < CACHE_TTL) {
                console.log('dx-markdown: Using cached content');
                return new TextEncoder().encode(cached.content);
            }
            
            // Get workspace paths
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                console.warn('dx-markdown: No workspace folder found');
                // Return cached or empty
                if (cached) {
                    return new TextEncoder().encode(cached.content);
                }
                return new TextEncoder().encode('# No workspace folder\n\nPlease open a workspace folder.');
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            const relativePath = path.relative(workspaceRoot, realPath);
            const relativeDir = path.dirname(relativePath);
            const baseName = path.basename(realPath, '.md');
            const humanPath = path.join(workspaceRoot, '.dx', 'markdown', relativeDir, `${baseName}.human`);
            const machinePath = path.join(workspaceRoot, '.dx', 'markdown', relativeDir, `${baseName}.machine`);
            
            console.log('dx-markdown: Human path:', humanPath);
            console.log('dx-markdown: Machine path:', machinePath);
            
            // Check if we can use cached machine format
            try {
                const mdStat = await fs.promises.stat(realPath);
                const machineStat = await fs.promises.stat(machinePath);
                
                // If machine file is newer than source, use cached human format
                if (machineStat.mtimeMs > mdStat.mtimeMs && fs.existsSync(humanPath)) {
                    console.log('dx-markdown: ⚡ Using cached .human (machine is fresh)');
                    const humanContent = await fs.promises.readFile(humanPath, 'utf-8');
                    documentCache.set(key, { content: humanContent, timestamp: Date.now() });
                    return new TextEncoder().encode(humanContent);
                }
            } catch (e) {
                // Machine or human file doesn't exist, continue with normal flow
                console.log('dx-markdown: Cache miss, parsing from source');
            }
            
            // Always read from .md file (source of truth on disk)
            let mdContent: string;
            try {
                await fs.promises.access(realPath, fs.constants.R_OK);
                mdContent = await fs.promises.readFile(realPath, 'utf-8');
            } catch (e) {
                console.error('dx-markdown: Failed to read .md file:', e);
                // Return cached or empty
                if (cached) {
                    return new TextEncoder().encode(cached.content);
                }
                return new TextEncoder().encode('# File not found\n\nThe markdown file could not be read.');
            }
            
            console.log('dx-markdown: Read .md file, length:', mdContent.length);
            console.log('dx-markdown: First 200 chars:', mdContent.substring(0, 200));
            
            // DX-Markdown LLM format uses:
            // - # headers (markdown headings)
            // - t:N(headers)[data] for tables (DX serializer table format)
            // - Compact spacing
            //
            // This is DIFFERENT from DX-Serializer .sr format which uses:
            // - name:count(headers)[data] for objects
            // - N|value for arrays
            // - No # headers
            //
            // We should NEVER see pure DX-Serializer format in .md files
            const isDxSerializerFormat = /^[a-zA-Z_][a-zA-Z0-9_]*:\d+\([^)]+\)\[/.test(mdContent.trim()) &&
                                        !mdContent.trim().startsWith('#') &&
                                        !mdContent.includes('# ');
            
            console.log('dx-markdown: Is pure DX-Serializer format (WRONG!):', isDxSerializerFormat);
            
            let humanContent: string;
            if (isDxSerializerFormat) {
                // Wrong format! This is DX-Serializer format in a .md file
                console.error('dx-markdown: ⚠️ ERROR: Detected DX-Serializer format in .md file!');
                console.error('dx-markdown: .md files should use DX-Markdown LLM format with # headers');
                console.error('dx-markdown: Showing content as-is for user to fix');
                
                // Show as-is so user can fix it
                humanContent = mdContent;
            } else {
                // Check if .md file is already in LLM format or still in human format
                // If it's in human format (has lots of whitespace, full words), convert it to LLM
                const looksLikeHumanFormat = mdContent.includes('  =') || // padded key = value
                                            /\n\n+/.test(mdContent) || // multiple blank lines
                                            mdContent.length > 1000 && mdContent.split('\n').length < 50; // verbose
                
                console.log('dx-markdown: Looks like human format:', looksLikeHumanFormat);
                
                if (looksLikeHumanFormat) {
                    // File is in human format - convert to LLM and save back to disk
                    console.log('dx-markdown: Converting human format to LLM format and saving to .md file...');
                    humanContent = mdContent;
                    
                    // Convert to LLM format
                    const llmContent = humanToLlm(mdContent);
                    console.log('dx-markdown: Converted to LLM format, length:', llmContent.length);
                    console.log('dx-markdown: First 200 chars of LLM:', llmContent.substring(0, 200));
                    
                    // Save LLM format back to .md file (async, non-blocking)
                    const trimmedLlmContent = llmContent.replace(/\n+$/, '');
                    fs.promises.writeFile(realPath, trimmedLlmContent, 'utf-8').catch(e => {
                        console.error('dx-markdown: Failed to save LLM format to .md file:', e);
                    });
                } else {
                    // .md file contains DX-Markdown LLM format - convert to human using WASM
                    console.log('dx-markdown: Converting DX-Markdown LLM format to Human format...');
                    humanContent = llmToHuman(mdContent);
                    console.log('dx-markdown: Converted to human format, length:', humanContent.length);
                }
            }
            
            // Save cache files asynchronously (non-blocking)
            await fs.promises.mkdir(path.dirname(humanPath), { recursive: true });
            await fs.promises.writeFile(humanPath, humanContent, 'utf-8');
            console.log('dx-markdown: Saved .human file');
            
            // Generate and save machine format using dx-markdown WASM
            if (wasmMarkdown && wasmMarkdown.human_to_machine) {
                try {
                    const machineBytes = wasmMarkdown.human_to_machine(humanContent);
                    await fs.promises.writeFile(machinePath, Buffer.from(machineBytes));
                    console.log('dx-markdown: Saved .machine file');
                } catch (e) {
                    console.error('dx-markdown: Failed to generate machine format:', e);
                }
            }
            
            // Update cache
            documentCache.set(key, { content: humanContent, timestamp: Date.now() });
            
            return new TextEncoder().encode(humanContent);
            
        } catch (error: any) {
            console.error('dx-markdown: ❌ CRITICAL readFile error for', realPath, ':', error);
            console.error('dx-markdown: Error stack:', error.stack);
            
            // NEVER throw - always return something
            const cached = documentCache.get(key);
            if (cached) {
                console.warn('dx-markdown: Returning cached content due to critical error');
                return new TextEncoder().encode(cached.content);
            }
            
            // Last resort: return error message as content
            console.warn('dx-markdown: Returning error message as content');
            return new TextEncoder().encode(`# Error Reading File\n\nAn error occurred while reading this file:\n\n\`\`\`\n${error.message}\n\`\`\`\n\nPlease check the console for more details.`);
        }
    }

    async writeFile(uri: vscode.Uri, content: Uint8Array): Promise<void> {
        const realPath = this.getRealPath(uri);
        const humanContent = new TextDecoder().decode(content);
        
        console.log('dx-markdown: writeFile called for:', realPath);
        console.log('dx-markdown: Human content length:', humanContent.length);
        
        // Get workspace root and build .human file path
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            console.warn('dx-markdown: No workspace folder found');
            return;
        }
        
        const workspaceRoot = workspaceFolders[0].uri.fsPath;
        const relativePath = path.relative(workspaceRoot, realPath);
        const relativeDir = path.dirname(relativePath);
        const baseName = path.basename(realPath, '.md');
        const humanPath = path.join(workspaceRoot, '.dx', 'markdown', relativeDir, `${baseName}.human`);
        
        // Read current .human file content to compare
        let currentHumanContent = '';
        try {
            if (fs.existsSync(humanPath)) {
                currentHumanContent = await fs.promises.readFile(humanPath, 'utf-8');
            }
        } catch (e) {
            // File doesn't exist yet, that's fine
        }
        
        // If the human content hasn't changed, don't write anything
        if (humanContent.trim() === currentHumanContent.trim()) {
            console.log('dx-markdown: Content unchanged, skipping write');
            return;
        }
        
        console.log('dx-markdown: Content changed, saving...');
        
        // Mark file as being written to prevent watcher from re-processing
        const key = realPath.toLowerCase();
        writingMarkdownFiles.add(key);
        
        try {
            // Content has changed, save to .human file (source of truth)
            await fs.promises.mkdir(path.dirname(humanPath), { recursive: true });
            await fs.promises.writeFile(humanPath, humanContent, 'utf-8');
            console.log('dx-markdown: Saved .human file');
            
            // Generate and save machine format using dx-markdown WASM
            const machinePath = path.join(workspaceRoot, '.dx', 'markdown', relativeDir, `${baseName}.machine`);
            if (wasmMarkdown && wasmMarkdown.human_to_machine) {
                try {
                    const machineBytes = wasmMarkdown.human_to_machine(humanContent);
                    await fs.promises.writeFile(machinePath, Buffer.from(machineBytes));
                    console.log('dx-markdown: Saved .machine file');
                } catch (e) {
                    console.error('dx-markdown: Failed to generate machine format:', e);
                }
            }
            
            // Convert human content to DX-Markdown LLM format using dx-markdown WASM
            // DX-Markdown LLM format: Token-optimized Markdown (still uses # headers)
            // NOT DX-Serializer format (which uses N| or name:count syntax)
            console.log('dx-markdown: Converting human to LLM format...');
            console.log('dx-markdown: WASM initialized:', wasmMarkdownInitialized);
            console.log('dx-markdown: WASM module exists:', !!wasmMarkdown);
            console.log('dx-markdown: human_to_llm exists:', !!(wasmMarkdown && wasmMarkdown.human_to_llm));
            
            const llmContent = humanToLlm(humanContent);
            console.log('dx-markdown: LLM content length:', llmContent.length);
            console.log('dx-markdown: First 200 chars of LLM:', llmContent.substring(0, 200));
            
            const trimmedLlmContent = llmContent.replace(/\n+$/, '');
            await fs.promises.writeFile(realPath, trimmedLlmContent, 'utf-8');
            console.log('dx-markdown: Saved DX-Markdown LLM format to .md file');
            
            // Update cache
            documentCache.set(key, { content: humanContent, timestamp: Date.now() });
            
            this._onDidChangeFile.fire([{ type: vscode.FileChangeType.Changed, uri }]);
        } finally {
            // Remove from writing set after a delay to prevent immediate re-trigger
            setTimeout(() => writingMarkdownFiles.delete(key), 500);
        }
    }

    async delete(): Promise<void> {}
    async rename(): Promise<void> {}
    
    private getRealPath(uri: vscode.Uri): string {
        // Handle Windows paths: /C:/path -> C:/path or /c%3A/path -> c:/path
        let p = decodeURIComponent(uri.path);
        if (p.match(/^\/[a-zA-Z]:\//)) {
            p = p.substring(1);
        }
        return p;
    }
    
    private async saveHumanFile(mdPath: string, humanContent: string): Promise<void> {
        try {
            // Get workspace root
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                console.warn('No workspace folder found');
                return;
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            
            // Get relative path from workspace root
            const relativePath = path.relative(workspaceRoot, mdPath);
            
            // Build path: .dx/markdown/{relative-dir}/{filename-without-ext}.human
            const relativeDir = path.dirname(relativePath);
            const baseName = path.basename(mdPath);
            // Remove .md extension
            const nameWithoutExt = baseName.replace(/\.md$/, '');
            
            const humanDir = path.join(workspaceRoot, '.dx', 'markdown', relativeDir);
            const humanPath = path.join(humanDir, `${nameWithoutExt}.human`);
            const machinePath = path.join(humanDir, `${nameWithoutExt}.machine`);
            
            await fs.promises.mkdir(humanDir, { recursive: true });
            
            // Save human format
            await fs.promises.writeFile(humanPath, humanContent, 'utf-8');
            
            // Generate and save machine format
            if (wasmMarkdown && wasmMarkdown.human_to_machine) {
                try {
                    const machineBytes = wasmMarkdown.human_to_machine(humanContent);
                    await fs.promises.writeFile(machinePath, Buffer.from(machineBytes));
                } catch (e) {
                    console.error('Failed to generate machine format:', e);
                }
            }
        } catch (e) {
            console.error('Failed to save human file:', e);
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

export function isMarkdownFile(filePath: string): boolean {
    return filePath.endsWith('.md');
}

export function getMarkdownHumanUri(mdUri: vscode.Uri): vscode.Uri {
    // Create URI with markdownhuman scheme, preserving the exact path
    // This ensures the virtual file system receives the correct disk path
    return vscode.Uri.file(mdUri.fsPath).with({ scheme: MARKDOWN_HUMAN_SCHEME });
}
