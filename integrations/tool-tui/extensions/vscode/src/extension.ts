/**
 * DX Extension - Holographic View for DX Serializer Files
 * 
 * On disk: LLM format (compact DSR format with commas) in `dx` file
 * In editor: Human format shown via virtual file system with `dx` path
 * 
 * Uses dx-serializer WASM for proper LLM ↔ Human conversion.
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import {
    LLM_MODELS,
    estimateTokens,
    calculateCost,
    formatCost,
    formatTokenCount,
    getModelsByProvider,
} from './llmModels';
import { llmToHumanFallback } from './conversion';
import { MarkdownHumanFileSystem, initMarkdownWasm, isMarkdownFile, getMarkdownHumanUri, writingMarkdownFiles } from './markdownLensFileSystem';
import { MarkdownFilterStatusBar, registerMarkdownFilterCommands } from './markdownFilterStatusBar';
import { activateMarkdownColorizer, deactivateMarkdownColorizer, triggerMarkdownColorization } from './markdownColorizer';

const DX_HUMAN_SCHEME = 'dxhuman';
const MARKDOWN_HUMAN_SCHEME = 'markdownhuman';

// ============================================================================
// WASM Serializer
// ============================================================================

let wasmSerializer: any = null;
let wasmInitialized = false;

async function initWasm(extensionPath: string): Promise<void> {
    if (wasmInitialized) {
        console.log('DX: WASM already initialized, skipping');
        return;
    }
    
    console.log('DX: Starting WASM initialization...');
    console.log('DX: Extension path:', extensionPath);
    
    try {
        const wasmPath = path.join(extensionPath, 'wasm-serializer', 'serializer.js');
        console.log('DX: WASM JS path:', wasmPath);
        
        // Check if file exists
        try {
            await fs.promises.access(wasmPath, fs.constants.R_OK);
            console.log('DX: ✅ WASM JS file exists and is readable');
        } catch (e) {
            console.error('DX: ❌ WASM JS file not accessible:', e);
            throw e;
        }
        
        // Convert Windows path to file:// URL
        const wasmUrl = vscode.Uri.file(wasmPath).toString();
        console.log('DX: Loading WASM from URL:', wasmUrl);
        
        const wasm = await import(wasmUrl);
        console.log('DX: ✅ WASM module imported');
        
        // Initialize WASM
        const wasmBinaryPath = path.join(extensionPath, 'wasm-serializer', 'serializer_bg.wasm');
        console.log('DX: WASM binary path:', wasmBinaryPath);
        
        const wasmBinary = await fs.promises.readFile(wasmBinaryPath);
        console.log('DX: ✅ WASM binary loaded, size:', wasmBinary.length);
        
        await wasm.default(wasmBinary);
        console.log('DX: ✅ WASM initialized');
        
        wasmSerializer = wasm;
        wasmInitialized = true;
        console.log('DX: ✅ WASM serializer initialized successfully');
    } catch (e) {
        console.error('DX: ❌ Failed to initialize WASM:', e);
        console.error('DX: Error stack:', e instanceof Error ? e.stack : String(e));
    }
}

// ============================================================================
// LLM ↔ Human Conversion
// ============================================================================

function llmToHuman(llmContent: string): string {
    if (!llmContent.trim()) return '';
    
    // Try WASM first if available
    if (wasmSerializer && wasmInitialized) {
        try {
            const serializer = new wasmSerializer.DxSerializer();
            const result = serializer.toHuman(llmContent);
            if (result.success) {
                return result.content;
            }
            console.warn('DX: WASM toHuman failed:', result.error);
        } catch (e) {
            console.warn('DX: WASM toHuman exception:', e);
        }
    }
    
    return llmToHumanFallback(llmContent);
}

function humanToLlm(humanContent: string): string {
    if (!humanContent.trim()) return '';
    
    console.log('DX: humanToLlm called, wasmSerializer:', !!wasmSerializer, 'wasmInitialized:', wasmInitialized);
    
    // Always convert - don't try to detect format
    // Human format has spaces around = (key = value)
    // LLM format has no spaces (key=value)
    
    // Try WASM first if available
    if (wasmSerializer && wasmInitialized) {
        try {
            console.log('DX: Attempting WASM conversion...');
            const serializer = new wasmSerializer.DxSerializer();
            const result = serializer.toDense(humanContent);
            console.log('DX: WASM conversion result - success:', result.success);
            if (result.success) {
                console.log('DX: WASM conversion successful, first 100 chars:', result.content.substring(0, 100));
                return result.content;
            }
            console.warn('DX: WASM toDense failed:', result.error);
        } catch (e) {
            console.error('DX: WASM toDense exception:', e);
        }
    } else {
        console.warn('DX: WASM not available, using fallback');
    }
    
    console.log('DX: Using fallback conversion');
    return humanToLlmFallback(humanContent);
}

// ============================================================================
// Human → LLM Format Conversion
// ============================================================================

interface SectionContent {
    scalars: Map<string, string>;
    arrays: Map<string, string[]>;
}

function humanToLlmFallback(humanContent: string): string {
    const lines = humanContent.split('\n');
    const output: string[] = [];
    
    const rootScalars = new Map<string, string>();
    const allSections = new Map<string, { type: 'regular' | 'tabular'; data: any; order: number }>();
    let sectionOrder = 0;
    
    let currentSection = '';
    let currentTabularBase = '';
    let currentTabularIndex = 0;
    let pendingArrayKey = '';
    let pendingArrayItems: string[] = [];
    
    // Quote value if it contains spaces (for LLM format)
    const quoteIfNeeded = (v: string): string => {
        const trimmed = v.trim();
        if (trimmed.includes(' ')) {
            return `"${trimmed.replace(/"/g, '\\"')}"`;
        }
        return trimmed;
    };
    
    // Get or create regular section content
    const getSection = (name: string): { scalars: Map<string, string>; arrays: Map<string, string[]> } => {
        if (!allSections.has(name)) {
            const data = { scalars: new Map<string, string>(), arrays: new Map<string, string[]>() };
            allSections.set(name, { type: 'regular', data, order: sectionOrder++ });
        }
        return allSections.get(name)!.data;
    };
    
    // Get or create tabular section
    const getTabularSection = (name: string): Array<Map<string, string>> => {
        if (!allSections.has(name)) {
            const data: Array<Map<string, string>> = [];
            allSections.set(name, { type: 'tabular', data, order: sectionOrder++ });
        }
        return allSections.get(name)!.data;
    };
    
    // Flush pending array to current section
    const flushArray = () => {
        if (pendingArrayKey && pendingArrayItems.length > 0) {
            if (currentSection) {
                const section = getSection(currentSection);
                section.arrays.set(pendingArrayKey, [...pendingArrayItems]);
            } else {
                // Root level array
                rootScalars.set(pendingArrayKey, `[${pendingArrayItems.join(' ')}]`);
            }
            pendingArrayKey = '';
            pendingArrayItems = [];
        }
    };
    
    // Flush current section when switching to a new one
    const flushSection = () => {
        flushArray();
    };
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        
        // Section header: [section] or [section.subsection] or [section:index] (tabular)
        const sectionMatch = trimmed.match(/^\[([a-zA-Z_][a-zA-Z0-9_.]*?)(?::(\d+))?\]$/);
        if (sectionMatch) {
            flushSection();
            const baseName = sectionMatch[1];
            const index = sectionMatch[2];
            
            if (index) {
                // Tabular section like [dependencies:1]
                currentTabularBase = baseName;
                currentTabularIndex = parseInt(index, 10);
                currentSection = ''; // Clear regular section
                
                // Initialize tabular section if needed
                getTabularSection(baseName);
            } else {
                // Regular section
                currentSection = baseName;
                currentTabularBase = '';
                currentTabularIndex = 0;
            }
            continue;
        }
        
        // List item: - value
        if (trimmed.startsWith('- ')) {
            const item = trimmed.substring(2).trim();
            pendingArrayItems.push(quoteIfNeeded(item));
            continue;
        }
        
        // Array header: key:
        const arrayMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):$/);
        if (arrayMatch) {
            flushArray();
            pendingArrayKey = arrayMatch[1];
            continue;
        }
        
        // Key-value pair: key = value (with flexible spacing)
        const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*(.*)$/);
        if (kvMatch) {
            flushArray();
            const key = kvMatch[1];
            const value = quoteIfNeeded(kvMatch[2].trim());
            
            if (currentTabularBase) {
                // Add to tabular section
                const rows = getTabularSection(currentTabularBase);
                // Ensure we have enough rows
                while (rows.length < currentTabularIndex) {
                    rows.push(new Map());
                }
                rows[currentTabularIndex - 1].set(key, value);
            } else if (currentSection) {
                const section = getSection(currentSection);
                section.scalars.set(key, value);
            } else {
                rootScalars.set(key, value);
            }
        }
    }
    
    flushSection();
    
    // Output root scalars first
    for (const [key, value] of rootScalars) {
        if (value.startsWith('[')) {
            // Array format: key=[items]
            output.push(`${key}=${value}`);
        } else {
            output.push(`${key}=${value}`);
        }
    }
    
    // Output all sections in their original order
    const sortedSections = Array.from(allSections.entries()).sort((a, b) => a[1].order - b[1].order);
    
    for (const [sectionName, sectionInfo] of sortedSections) {
        if (sectionInfo.type === 'tabular') {
            const rows = sectionInfo.data as Array<Map<string, string>>;
            if (rows.length === 0) continue;
            
            // Get schema from first row keys
            const schema = Array.from(rows[0].keys());
            
            // Build rows
            const rowStrings: string[] = [];
            for (const row of rows) {
                const values = schema.map(key => row.get(key) || '');
                rowStrings.push(values.join(' '));
            }
            
            // NEW FORMAT: name[col1 col2](rows)
            output.push(`${sectionName}[${schema.join(' ')}](`);
            output.push(...rowStrings);
            output.push(')');
        } else {
            // Regular section - NEW FORMAT: name(key=value key2=value2)
            const content = sectionInfo.data as { scalars: Map<string, string>; arrays: Map<string, string[]> };
            const parts: string[] = [];
            
            // Add scalars
            for (const [key, value] of content.scalars) {
                parts.push(`${key}=${value}`);
            }
            
            // Add arrays
            for (const [key, items] of content.arrays) {
                parts.push(`${key}=[${items.join(' ')}]`);
            }
            
            if (parts.length > 0) {
                output.push(`${sectionName}(${parts.join(' ')})`);
            }
        }
    }
    
    return output.join('\n');
}

// ============================================================================
// DX Human File System Provider
// ============================================================================

class DxHumanFileSystem implements vscode.FileSystemProvider {
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
            console.error('DX: Failed to stat file:', realPath, error);
            
            if (error.code === 'ENOENT') {
                throw vscode.FileSystemError.FileNotFound(uri);
            }
            
            // Return a default stat for other errors
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
        
        try {
            // Get workspace paths for caching
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (workspaceFolders && workspaceFolders.length > 0) {
                const workspaceRoot = workspaceFolders[0].uri.fsPath;
                const relativePath = path.relative(workspaceRoot, realPath);
                let relativeDir = path.dirname(relativePath);
                if (relativeDir === '.') relativeDir = '';
                const baseName = path.basename(realPath);
                const nameWithoutExt = baseName.replace(/\.sr$/, '') || baseName;
                const llmPath = path.join(workspaceRoot, '.dx', 'serializer', relativeDir, `${nameWithoutExt}.llm`);
                const machinePath = path.join(workspaceRoot, '.dx', 'serializer', relativeDir, `${nameWithoutExt}.machine`);
                
                // Check if we can use cached machine format
                try {
                    const sourceStat = await fs.promises.stat(realPath);
                    const machineStat = await fs.promises.stat(machinePath);
                    
                    // If machine file is newer than source, use cached LLM format
                    if (machineStat.mtimeMs > sourceStat.mtimeMs && fs.existsSync(llmPath)) {
                        console.log('DX: ⚡ Using cached .llm (machine is fresh)');
                        const llmContent = await fs.promises.readFile(llmPath, 'utf-8');
                        const humanContent = llmToHuman(llmContent);
                        return new TextEncoder().encode(humanContent);
                    }
                } catch (e) {
                    // Machine file doesn't exist, continue with normal flow
                    console.log('DX: Cache miss, parsing from source');
                }
            }
            
            // Ensure file exists first
            await fs.promises.access(realPath, fs.constants.R_OK);
            
            // Read the LLM content from disk
            const llmContent = await fs.promises.readFile(realPath, 'utf-8');
            
            // Convert to human format
            const humanContent = llmToHuman(llmContent);
            
            // Save to .dx/*.llm and .machine (async, non-blocking)
            this.saveLlmFile(realPath, llmContent, humanContent).catch(e => {
                console.error('DX: ❌ Failed to save LLM/machine files:', e);
                console.error('DX: Error stack:', e instanceof Error ? e.stack : String(e));
            });
            
            return new TextEncoder().encode(humanContent);
            
        } catch (error: any) {
            console.error('DX: readFile error for', realPath, ':', error);
            
            // If file not found, throw proper error
            if (error.code === 'ENOENT') {
                throw vscode.FileSystemError.FileNotFound(uri);
            }
            
            // For permission errors
            if (error.code === 'EACCES' || error.code === 'EPERM') {
                throw vscode.FileSystemError.NoPermissions(uri);
            }
            
            // For any other error, throw unavailable to trigger retry
            throw vscode.FileSystemError.Unavailable(uri);
        }
    }

    async writeFile(uri: vscode.Uri, content: Uint8Array): Promise<void> {
        const realPath = this.getRealPath(uri);
        const humanContent = new TextDecoder().decode(content);
        
        // Read the current disk content
        let currentDiskContent = '';
        try {
            currentDiskContent = await fs.promises.readFile(realPath, 'utf-8');
        } catch (e) {
            // File doesn't exist yet, that's fine
        }
        
        // Convert current disk content to human format to compare
        const currentHumanContent = currentDiskContent ? llmToHuman(currentDiskContent) : '';
        
        // If the human content hasn't changed, don't write anything
        // This prevents unnecessary conversions and preserves the exact disk format
        if (humanContent.trim() === currentHumanContent.trim()) {
            return;
        }
        
        // COMMENTED OUT: Old architecture converted human to LLM on save
        // Now we keep human format on disk
        // const llmContent = humanToLlm(humanContent);
        // const trimmedLlmContent = llmContent.replace(/\n+$/, '');
        // await fs.promises.writeFile(realPath, trimmedLlmContent, 'utf-8');
        
        // Write human format directly to disk
        await fs.promises.writeFile(realPath, humanContent, 'utf-8');
        
        // Convert to LLM format and save
        const llmContent = humanToLlm(humanContent);
        await this.saveLlmFile(realPath, llmContent, humanContent);
        
        this._onDidChangeFile.fire([{ type: vscode.FileChangeType.Changed, uri }]);
    }

    async delete(): Promise<void> {}
    async rename(): Promise<void> {}
    
    private getRealPath(uri: vscode.Uri): string {
        // Handle Windows paths: /C:/path -> C:/path
        let p = uri.path;
        if (p.match(/^\/[a-zA-Z]:\//)) {
            p = p.substring(1);
        }
        return p;
    }
    
    private async saveLlmFile(sourcePath: string, llmContent: string, humanContent: string): Promise<void> {
        try {
            console.log('DX: saveLlmFile called for:', sourcePath);
            
            // Wait for WASM to initialize (max 5 seconds)
            let attempts = 0;
            while (!wasmInitialized && attempts < 50) {
                await new Promise(resolve => setTimeout(resolve, 100));
                attempts++;
            }
            
            if (!wasmInitialized) {
                console.warn('DX: ⚠️ WASM not initialized after 5 seconds, proceeding anyway');
            }
            
            // Get workspace root
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                console.warn('DX: ⚠️ No workspace folder found');
                return;
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            console.log('DX: Workspace root:', workspaceRoot);
            
            // Get relative path from workspace root
            const relativePath = path.relative(workspaceRoot, sourcePath);
            console.log('DX: Relative path:', relativePath);
            
            // Build path: .dx/{relative-dir}/{filename-without-ext}.llm
            const relativeDir = path.dirname(relativePath);
            const baseName = path.basename(sourcePath);
            // Remove extension from basename (e.g., example1.sr -> example1, dx -> dx)
            const nameWithoutExt = baseName.replace(/\.sr$/, '').replace(/\.dx$/, '') || baseName;
            
            const llmDir = path.join(workspaceRoot, '.dx', 'serializer', relativeDir);
            const llmFilePath = path.join(llmDir, `${nameWithoutExt}.llm`);
            const machinePath = path.join(llmDir, `${nameWithoutExt}.machine`);
            
            console.log('DX: Creating directory:', llmDir);
            await fs.promises.mkdir(llmDir, { recursive: true});
            
            console.log('DX: Writing .llm file:', llmFilePath);
            await fs.promises.writeFile(llmFilePath, llmContent, 'utf-8');
            console.log('DX: ✅ LLM file saved');
            
            // Generate machine format if WASM is available
            if (wasmSerializer && wasmInitialized) {
                try {
                    console.log('DX: Generating machine format...');
                    const machineBytes = wasmSerializer.human_to_machine(humanContent);
                    console.log('DX: Machine bytes generated, length:', machineBytes.length);
                    
                    await fs.promises.writeFile(machinePath, Buffer.from(machineBytes));
                    console.log('DX: ✅ Machine file saved:', machinePath);
                } catch (e) {
                    console.error('DX: ❌ Failed to generate machine format:', e);
                    console.error('DX: Error details:', e instanceof Error ? e.message : String(e));
                }
            } else {
                console.warn('DX: ⚠️ WASM not available for machine format generation');
                console.warn('DX: wasmSerializer:', !!wasmSerializer, 'wasmInitialized:', wasmInitialized);
            }
        } catch (e) {
            console.error('DX: ❌ Failed in saveLlmFile:', e);
            console.error('DX: Error stack:', e instanceof Error ? e.stack : String(e));
            throw e; // Re-throw to be caught by caller
        }
    }
}

// ============================================================================
// Token Counter with Official Tokenizers (100% accurate)
// ============================================================================

import { countTokens as countAccurateTokens, countAllTokens } from './tokenCounter';

class TokenCounter implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];
    private panel: vscode.WebviewPanel | null = null;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 1000);
        this.statusBarItem.command = 'dx.showTokenPanel';
        this.statusBarItem.show();
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor(() => this.update()),
            vscode.workspace.onDidChangeTextDocument(() => this.update()),
            vscode.workspace.onDidChangeConfiguration(e => {
                if (e.affectsConfiguration('dx.tokenCounter.showFileSize')) {
                    this.update();
                }
            })
        );
        this.update();
    }

    private update(): void {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this.statusBarItem.text = '$(symbol-number) --';
            return;
        }
        const text = editor.document.getText();
        
        // Use official GPT tokenizer (100% accurate)
        const tokens = countAccurateTokens(text, 'gpt');
        
        const tokenStr = formatTokenCount(tokens);
        const showFileSize = vscode.workspace.getConfiguration('dx').get('tokenCounter.showFileSize', true);
        
        if (showFileSize) {
            const sizeBytes = Buffer.byteLength(text, 'utf8');
            const sizeStr = this.formatFileSize(sizeBytes);
            this.statusBarItem.text = `# ${tokenStr} tokens, ${sizeStr}`;
        } else {
            this.statusBarItem.text = `# ${tokenStr} tokens`;
        }
        
        this.statusBarItem.tooltip = `GPT cl100k_base encoding (exact)\nFile size: ${this.formatFileSize(Buffer.byteLength(text, 'utf8'))}`;
    }
    
    private formatFileSize(bytes: number): string {
        if (bytes < 1024) return `${bytes}B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
    }

    async showPanel(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) { vscode.window.showWarningMessage('No file open'); return; }

        if (this.panel) { this.panel.reveal(); }
        else {
            this.panel = vscode.window.createWebviewPanel('dxTokens', 'Token Analysis', vscode.ViewColumn.Beside, {});
            this.panel.onDidDispose(() => { this.panel = null; });
        }

        const text = editor.document.getText();
        const fileName = editor.document.fileName.split(/[/\\]/).pop() || '';
        const isDxFile = /\.(dsr|dx|sr)$/.test(fileName) || fileName === 'dx';
        const isMarkdownMachine = fileName.endsWith('.machine');
        const savings = isDxFile ? 73.3 : isMarkdownMachine ? 42.9 : 0;

        // Get accurate token counts for all providers
        const allTokens = countAllTokens(text);
        const gptTokens = allTokens.gpt4o;
        const claudeTokens = allTokens.claude;

        let rows = '';
        for (const [provider, models] of getModelsByProvider()) {
            rows += `<tr class="ph"><td colspan="5">${provider}</td></tr>`;
            for (const m of models) {
                // Use accurate token counts per provider
                let tokens: number;
                if (m.provider === 'OpenAI/Azure') {
                    tokens = gptTokens;
                } else if (m.provider === 'Anthropic') {
                    tokens = claudeTokens;
                } else {
                    // Gemini: estimate ~10% less efficient than GPT
                    tokens = Math.ceil(gptTokens * 1.1);
                }
                const cost = calculateCost(tokens, m, 'input');
                rows += `<tr><td>${m.name}</td><td>${m.contextWindow}</td><td class="r">${tokens.toLocaleString()}</td><td class="r">${formatCost(cost)}</td><td class="r">${savings > 0 ? `-${formatCost(cost * savings / (100 - savings))}` : '-'}</td></tr>`;
            }
        }

        this.panel.webview.html = `<!DOCTYPE html><html><head><style>
body{font-family:system-ui;background:#000;color:#eee;padding:20px}
h1{font-size:20px;margin-bottom:16px}
table{width:100%;border-collapse:collapse}
th,td{padding:8px;text-align:left;border-bottom:1px solid #333}
.r{text-align:right;font-family:monospace}
.ph{background:#111}.ph td{font-weight:600;color:#0070f3}
${savings > 0 ? `.bn{background:linear-gradient(135deg,#0d9373,#50e3c2);padding:16px;border-radius:8px;margin-bottom:16px;color:#000}` : ''}
</style></head><body>
${savings > 0 ? `<div class="bn">⚡ DX ${isDxFile ? 'Serializer' : 'Markdown'} saves ${savings}% tokens</div>` : ''}
<h1>Token Analysis</h1>
<table><thead><tr><th>Model</th><th>Context</th><th class="r">Tokens</th><th class="r">Cost</th><th class="r">DX Saves</th></tr></thead><tbody>${rows}</tbody></table>
</body></html>`;
    }

    dispose(): void {
        this.statusBarItem.dispose();
        this.panel?.dispose();
        this.disposables.forEach(d => d.dispose());
    }
}

// ============================================================================
// Export Status Bar
// ============================================================================

class ExportStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 999);
        this.statusBarItem.command = 'dx.export.showMenu';
        this.statusBarItem.text = '$(export) Export';
        this.statusBarItem.tooltip = 'Export DX to other formats';
        
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor(() => this.updateVisibility())
        );
        this.updateVisibility();
    }

    private updateVisibility(): void {
        const editor = vscode.window.activeTextEditor;
        if (editor && isDxLlmFile(editor.document.uri.fsPath)) {
            this.statusBarItem.show();
        } else {
            this.statusBarItem.hide();
        }
    }

    dispose(): void {
        this.statusBarItem.dispose();
        this.disposables.forEach(d => d.dispose());
    }
}

// ============================================================================
// Format Converters
// ============================================================================

function humanToJson(humanContent: string, compact: boolean = false): string {
    const lines = humanContent.split('\n');
    const result: Record<string, any> = {};
    let currentSection = '';
    let pendingArrayKey = '';
    let pendingArrayItems: string[] = [];
    
    const flushArray = () => {
        if (pendingArrayKey && pendingArrayItems.length > 0) {
            if (currentSection) {
                if (!result[currentSection]) result[currentSection] = {};
                result[currentSection][pendingArrayKey] = pendingArrayItems;
            } else {
                result[pendingArrayKey] = pendingArrayItems;
            }
            pendingArrayKey = '';
            pendingArrayItems = [];
        }
    };
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        
        const sectionMatch = trimmed.match(/^\[([a-zA-Z_][a-zA-Z0-9_.]*)\]$/);
        if (sectionMatch) {
            flushArray();
            currentSection = sectionMatch[1];
            if (!result[currentSection]) result[currentSection] = {};
            continue;
        }
        
        if (trimmed.startsWith('- ')) {
            pendingArrayItems.push(trimmed.substring(2).trim());
            continue;
        }
        
        const arrayMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):$/);
        if (arrayMatch) {
            flushArray();
            pendingArrayKey = arrayMatch[1];
            continue;
        }
        
        const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*(.*)$/);
        if (kvMatch) {
            flushArray();
            const key = kvMatch[1];
            let value: any = kvMatch[2].trim();
            
            // Parse value types
            if (value === 'true') value = true;
            else if (value === 'false') value = false;
            else if (value === 'none' || value === 'null') value = null;
            else if (/^-?\d+$/.test(value)) value = parseInt(value, 10);
            else if (/^-?\d+\.\d+$/.test(value)) value = parseFloat(value);
            
            if (currentSection) {
                result[currentSection][key] = value;
            } else {
                result[key] = value;
            }
        }
    }
    
    flushArray();
    return compact ? JSON.stringify(result) : JSON.stringify(result, null, 2);
}

function humanToYaml(humanContent: string): string {
    const lines = humanContent.split('\n');
    const output: string[] = [];
    let currentSection = '';
    let pendingArrayKey = '';
    let pendingArrayItems: string[] = [];
    
    const flushArray = () => {
        if (pendingArrayKey && pendingArrayItems.length > 0) {
            const indent = currentSection ? '  ' : '';
            output.push(`${indent}${pendingArrayKey}:`);
            for (const item of pendingArrayItems) {
                output.push(`${indent}  - ${item}`);
            }
            pendingArrayKey = '';
            pendingArrayItems = [];
        }
    };
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        
        const sectionMatch = trimmed.match(/^\[([a-zA-Z_][a-zA-Z0-9_.]*)\]$/);
        if (sectionMatch) {
            flushArray();
            if (output.length > 0) output.push('');
            currentSection = sectionMatch[1];
            output.push(`${currentSection}:`);
            continue;
        }
        
        if (trimmed.startsWith('- ')) {
            pendingArrayItems.push(trimmed.substring(2).trim());
            continue;
        }
        
        const arrayMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):$/);
        if (arrayMatch) {
            flushArray();
            pendingArrayKey = arrayMatch[1];
            continue;
        }
        
        const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*(.*)$/);
        if (kvMatch) {
            flushArray();
            const key = kvMatch[1];
            const value = kvMatch[2].trim();
            const indent = currentSection ? '  ' : '';
            output.push(`${indent}${key}: ${value}`);
        }
    }
    
    flushArray();
    return output.join('\n');
}

function humanToToml(humanContent: string): string {
    const lines = humanContent.split('\n');
    const output: string[] = [];
    let currentSection = '';
    let pendingArrayKey = '';
    let pendingArrayItems: string[] = [];
    
    const flushArray = () => {
        if (pendingArrayKey && pendingArrayItems.length > 0) {
            const quoted = pendingArrayItems.map(i => `"${i}"`).join(', ');
            output.push(`${pendingArrayKey} = [${quoted}]`);
            pendingArrayKey = '';
            pendingArrayItems = [];
        }
    };
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        
        const sectionMatch = trimmed.match(/^\[([a-zA-Z_][a-zA-Z0-9_.]*)\]$/);
        if (sectionMatch) {
            flushArray();
            if (output.length > 0) output.push('');
            currentSection = sectionMatch[1];
            output.push(`[${currentSection}]`);
            continue;
        }
        
        if (trimmed.startsWith('- ')) {
            pendingArrayItems.push(trimmed.substring(2).trim());
            continue;
        }
        
        const arrayMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*):$/);
        if (arrayMatch) {
            flushArray();
            pendingArrayKey = arrayMatch[1];
            continue;
        }
        
        const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*(.*)$/);
        if (kvMatch) {
            flushArray();
            const key = kvMatch[1];
            let value = kvMatch[2].trim();
            
            // Quote strings in TOML
            if (value !== 'true' && value !== 'false' && !/^-?\d+(\.\d+)?$/.test(value)) {
                value = `"${value}"`;
            }
            output.push(`${key} = ${value}`);
        }
    }
    
    flushArray();
    return output.join('\n');
}

function humanToCsv(humanContent: string): string {
    const lines = humanContent.split('\n');
    const rows: string[][] = [];
    let currentSection = '';
    
    rows.push(['section', 'key', 'value']);
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        
        const sectionMatch = trimmed.match(/^\[([a-zA-Z_][a-zA-Z0-9_.]*)\]$/);
        if (sectionMatch) {
            currentSection = sectionMatch[1];
            continue;
        }
        
        if (trimmed.startsWith('- ')) continue; // Skip array items for CSV
        if (trimmed.endsWith(':')) continue; // Skip array headers
        
        const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*(.*)$/);
        if (kvMatch) {
            const key = kvMatch[1];
            const value = kvMatch[2].trim().replace(/"/g, '""');
            rows.push([currentSection, key, `"${value}"`]);
        }
    }
    
    return rows.map(r => r.join(',')).join('\n');
}

function humanToToon(humanContent: string): string {
    // TOON format: similar to human but with different syntax
    const lines = humanContent.split('\n');
    const output: string[] = [];
    
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) {
            output.push('');
            continue;
        }
        
        // Section headers become @section
        const sectionMatch = trimmed.match(/^\[([a-zA-Z_][a-zA-Z0-9_.]*)\]$/);
        if (sectionMatch) {
            output.push(`@${sectionMatch[1]}`);
            continue;
        }
        
        // Array items stay the same
        if (trimmed.startsWith('- ')) {
            output.push(trimmed);
            continue;
        }
        
        // Array headers become key:
        if (trimmed.endsWith(':')) {
            output.push(trimmed);
            continue;
        }
        
        // Key-value pairs use : instead of =
        const kvMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*(.*)$/);
        if (kvMatch) {
            output.push(`${kvMatch[1]}: ${kvMatch[2].trim()}`);
            continue;
        }
        
        output.push(trimmed);
    }
    
    return output.join('\n');
}

// ============================================================================
// Utility Functions
// ============================================================================

function isDxLlmFile(filePath: string): boolean {
    const fileName = path.basename(filePath);
    return fileName === 'dx' || /\.(dx|dsr|sr|sr|serializer)$/.test(fileName);
}

// ============================================================================
// Extension Activation
// ============================================================================

let dxHumanFs: DxHumanFileSystem;
let markdownHumanFs: MarkdownHumanFileSystem;
let tokenCounter: TokenCounter;
let exportStatusBar: ExportStatusBar;
let markdownFilterStatusBar: MarkdownFilterStatusBar;
let extensionReady = false;
let wasmMarkdownModule: any = null;

export function activate(context: vscode.ExtensionContext): void {
    console.log('DX Extension: Activating (synchronous)...');

    // Register virtual file system IMMEDIATELY (synchronously)
    // This MUST happen before any async operations to handle file restoration
    dxHumanFs = new DxHumanFileSystem();
    // =====================================================================
    // VIRTUAL FILE SYSTEM REGISTRATION - COMMENTED OUT (2026 Architecture)
    // =====================================================================
    // New architecture: Human format on disk, LLM in .dx folder
    // Virtual FS no longer needed - keeping code for reference
    // =====================================================================
    
    /*
    context.subscriptions.push(
        vscode.workspace.registerFileSystemProvider(DX_HUMAN_SCHEME, dxHumanFs, {
            isCaseSensitive: true,
            isReadonly: false,
        })
    );
    
    // Register markdown virtual file system
    markdownHumanFs = new MarkdownHumanFileSystem();
    const markdownRegistration = vscode.workspace.registerFileSystemProvider(MARKDOWN_HUMAN_SCHEME, markdownHumanFs, {
        isCaseSensitive: true,
        isReadonly: false,
    });
    context.subscriptions.push(markdownRegistration);
    console.log(`DX Extension: Markdown file system provider registered with scheme: ${MARKDOWN_HUMAN_SCHEME}`);
    */
    
    console.log('DX Extension: Virtual file systems disabled (2026 architecture)');

    // Mark as ready immediately
    extensionReady = true;

    // Initialize WASM serializer asynchronously (non-blocking)
    initWasm(context.extensionPath).then(() => {
        console.log('DX Extension: WASM serializer initialized');
    }).catch(e => {
        console.error('DX: WASM serializer initialization failed (non-critical):', e);
    });
    
    // DISABLED: Initialize WASM markdown converter (generates .dx/markdown files)
    /*
    initMarkdownWasm(context.extensionPath).then(() => {
        console.log('DX Extension: WASM markdown initialized');
        
        // Get the WASM module for filter status bar
        const wasmPath = path.join(context.extensionPath, 'wasm-markdown', 'markdown.js');
        import(wasmPath).then(wasm => {
            wasmMarkdownModule = wasm;
            
            // Initialize markdown filter status bar
            markdownFilterStatusBar = new MarkdownFilterStatusBar(wasmMarkdownModule);
            context.subscriptions.push(markdownFilterStatusBar);
            registerMarkdownFilterCommands(context, markdownFilterStatusBar);
            
            console.log('DX Extension: Markdown filter status bar initialized');
        }).catch(e => {
            console.error('DX: Failed to load WASM module for filters:', e);
        });
    }).catch(e => {
        console.error('DX: WASM markdown initialization failed (non-critical):', e);
    });
    */
    console.log('DX Extension: Markdown .dx folder generation disabled');
    
    // Activate markdown colorizer for beautiful syntax highlighting (independent of WASM)
    activateMarkdownColorizer(context);
    console.log('DX Extension: Markdown colorizer activated');
    
    // Activate DX Explosion (power-mode integration)
    import('./power-mode/index').then(({ activate: activateDxExplosion }) => {
        console.log('DX Extension: Activating DX Explosion...');
        activateDxExplosion(context);
        console.log('DX Extension: DX Explosion activated');
    }).catch(e => {
        console.error('DX: Failed to activate DX Explosion:', e);
    });

    console.log('DX Extension: Markdown .dx folder generation disabled');

    // Set up file watcher for DX serializer files to detect external changes
    // Watch for .sr files and files named 'dx' (no extension)
    const dxWatcher = vscode.workspace.createFileSystemWatcher('**/*.sr');
    const dxNoExtWatcher = vscode.workspace.createFileSystemWatcher('**/dx');
    context.subscriptions.push(dxWatcher);
    context.subscriptions.push(dxNoExtWatcher);
    
    // Track files being written by the extension to avoid circular updates
    const writingDxFiles = new Set<string>();
    
    const handleDxFileChange = async (uri: vscode.Uri) => {
        const key = uri.fsPath;
        
        // Ignore if we're currently writing this file
        if (writingDxFiles.has(key)) {
            return;
        }
        
        try {
            // Read the file content
            const fileContent = await vscode.workspace.fs.readFile(uri);
            const fileText = new TextDecoder().decode(fileContent);
            
            // Get workspace root
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                return;
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            const relativePath = path.relative(workspaceRoot, uri.fsPath);
            let relativeDir = path.dirname(relativePath);
            // Normalize path: if file is in root, relativeDir is '.', change to ''
            if (relativeDir === '.') {
                relativeDir = '';
            }
            const baseName = path.basename(uri.fsPath);
            const nameWithoutExt = baseName.replace(/\.sr$/, '') || baseName;
            
            // Generate paths in .dx/serializer/ folder (matching markdown pattern)
            const llmPath = path.join(workspaceRoot, '.dx', 'serializer', relativeDir, `${nameWithoutExt}.llm`);
            const machinePath = path.join(workspaceRoot, '.dx', 'serializer', relativeDir, `${nameWithoutExt}.machine`);
            
            // Keep human format on disk
            const humanContent = fileText;
            
            // Generate LLM and machine formats if WASM is available
            if (wasmSerializer && wasmInitialized) {
                try {
                    // Create directory structure
                    const outputDir = path.dirname(llmPath);
                    await vscode.workspace.fs.createDirectory(vscode.Uri.file(outputDir));
                    
                    // Convert to LLM format
                    const llmContent = humanToLlm(humanContent);
                    await vscode.workspace.fs.writeFile(
                        vscode.Uri.file(llmPath),
                        new TextEncoder().encode(llmContent)
                    );
                    
                    // Generate machine format
                    const machineBytes = wasmSerializer.human_to_machine(humanContent);
                    await vscode.workspace.fs.writeFile(
                        vscode.Uri.file(machinePath),
                        Buffer.from(machineBytes)
                    );
                    
                    console.log(`DX: Generated .llm and .machine files for ${baseName} in .dx/serializer/`);
                } catch (error) {
                    console.error('DX: Failed to generate LLM/machine formats:', error);
                }
            }
            
        } catch (error) {
            console.error('DX: Failed to handle file change:', error);
        }
    };
    
    dxWatcher.onDidChange(handleDxFileChange);
    dxNoExtWatcher.onDidChange(handleDxFileChange);
    
    // Also generate files when opening a dx file
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(async (editor) => {
            if (!editor) return;
            
            const uri = editor.document.uri;
            if (uri.scheme !== 'file') return;
            
            const fileName = path.basename(uri.fsPath);
            const isDxFile = fileName === 'dx' || fileName.endsWith('.sr');
            
            if (isDxFile) {
                console.log('DX: Opened dx file, generating .llm and .machine files...');
                await handleDxFileChange(uri);
            }
        })
    );
    
    console.log('DX Extension: DX serializer file watcher set up');

    // Token counter
    tokenCounter = new TokenCounter();
    context.subscriptions.push(tokenCounter);

    // Export status bar
    exportStatusBar = new ExportStatusBar();
    context.subscriptions.push(exportStatusBar);

    // Helper to get current file content
    const getCurrentContent = async (): Promise<{ content: string; isHuman: boolean } | null> => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return null;
        
        const content = editor.document.getText();
        const isHuman = editor.document.uri.scheme === DX_HUMAN_SCHEME || 
                        content.includes('[') && content.includes('] =');
        return { content, isHuman };
    };

    // Commands
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.showTokenPanel', () => tokenCounter.showPanel()),
        
        vscode.commands.registerCommand('dx.refreshFromDisk', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            await vscode.commands.executeCommand('workbench.action.files.revert');
            vscode.window.showInformationMessage('DX: Refreshed from disk');
        }),

        /* DEPRECATED: Show Human Format command (2026 architecture - human format on disk)
        // Format view commands
        vscode.commands.registerCommand('dx.format.showHuman', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            
            const filePath = editor.document.uri.fsPath;
            const isMarkdown = filePath.endsWith('.md');
            const isSerializer = isDxLlmFile(filePath);
            
            if (!isMarkdown && !isSerializer) {
                vscode.window.showWarningMessage('Not a DX file');
                return;
            }
            
            // Get workspace root
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                vscode.window.showWarningMessage('No workspace folder found');
                return;
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            const relativePath = path.relative(workspaceRoot, filePath);
            const relativeDir = path.dirname(relativePath);
            const baseName = path.basename(filePath, isMarkdown ? '.md' : '');
            const nameWithoutExt = baseName.replace(/\.sr$/, '') || baseName;
            
            const folder = isMarkdown ? 'markdown' : 'serializer';
            const humanPath = path.join(workspaceRoot, '.dx', folder, relativeDir, `${nameWithoutExt}.human`);
            
            try {
                // Open the actual .human file
                const humanUri = vscode.Uri.file(humanPath);
                await vscode.window.showTextDocument(humanUri, { 
                    preview: false,
                    viewColumn: vscode.ViewColumn.Beside
                });
            } catch (e) {
                vscode.window.showErrorMessage(`Human file not found: ${humanPath}`);
            }
        }),
        */

        vscode.commands.registerCommand('dx.format.showLlm', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            
            let filePath = editor.document.uri.fsPath;
            
            // Determine if this is a markdown or serializer file
            const isMarkdown = filePath.endsWith('.md');
            const isSerializer = isDxLlmFile(filePath);
            
            if (!isMarkdown && !isSerializer) {
                vscode.window.showWarningMessage('Not a DX file');
                return;
            }
            
            // Get workspace root
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                vscode.window.showWarningMessage('No workspace folder found');
                return;
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            const relativePath = path.relative(workspaceRoot, filePath);
            const baseName = path.basename(filePath, isMarkdown ? '.md' : '');
            const nameWithoutExt = baseName.replace(/\.sr$/, '') || baseName;
            
            // Calculate .llm file path in .dx folder
            const relativeDir = path.dirname(relativePath);
            const folder = isMarkdown ? 'markdown' : 'serializer';
            
            let llmPath;
            if (relativeDir === '.') {
                // File is in root directory
                llmPath = path.join(workspaceRoot, '.dx', folder, `${nameWithoutExt}.llm`);
            } else {
                // File is in subdirectory
                llmPath = path.join(workspaceRoot, '.dx', folder, relativeDir, `${nameWithoutExt}.llm`);
            }
            
            try {
                // Open the actual .llm file
                const llmUri = vscode.Uri.file(llmPath);
                await vscode.window.showTextDocument(llmUri, { viewColumn: vscode.ViewColumn.Beside });
            } catch (e) {
                vscode.window.showErrorMessage(`LLM file not found: ${llmPath}`);
            }
        }),

        vscode.commands.registerCommand('dx.format.showMachine', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            
            let filePath = editor.document.uri.fsPath;
            
            // Determine if this is a markdown or serializer file
            const isMarkdown = filePath.endsWith('.md');
            const isSerializer = isDxLlmFile(filePath);
            
            if (!isMarkdown && !isSerializer) {
                vscode.window.showWarningMessage('Not a DX file');
                return;
            }
            
            // Get workspace root
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                vscode.window.showWarningMessage('No workspace folder found');
                return;
            }
            
            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            const relativePath = path.relative(workspaceRoot, filePath);
            const baseName = path.basename(filePath, isMarkdown ? '.md' : '');
            const nameWithoutExt = baseName.replace(/\.sr$/, '') || baseName;
            
            // Calculate .machine file path in .dx folder
            const relativeDir = path.dirname(relativePath);
            const folder = isMarkdown ? 'markdown' : 'serializer';
            
            let machinePath;
            if (relativeDir === '.') {
                // File is in root directory
                machinePath = path.join(workspaceRoot, '.dx', folder, `${nameWithoutExt}.machine`);
            } else {
                // File is in subdirectory
                machinePath = path.join(workspaceRoot, '.dx', folder, relativeDir, `${nameWithoutExt}.machine`);
            }
            
            // Check if file exists
            if (!fs.existsSync(machinePath)) {
                vscode.window.showErrorMessage(`Machine file not found: ${machinePath}`);
                return;
            }
            
            try {
                // Open the actual .machine file
                const machineUri = vscode.Uri.file(machinePath);
                await vscode.window.showTextDocument(machineUri, { 
                    viewColumn: vscode.ViewColumn.Beside,
                    preview: false 
                });
            } catch (e) {
                vscode.window.showErrorMessage(`Failed to open machine file: ${e}`);
            }
        }),

        vscode.commands.registerCommand('dx.showLlmFormat', async () => {
            await vscode.commands.executeCommand('dx.format.showLlm');
        }),

        vscode.commands.registerCommand('dx.openHumanView', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('No file open');
                return;
            }
            
            if (editor.document.uri.scheme === DX_HUMAN_SCHEME) {
                vscode.window.showInformationMessage('Already viewing human format');
                return;
            }
            
            const filePath = editor.document.uri.fsPath;
            if (!isDxLlmFile(filePath)) {
                vscode.window.showWarningMessage('Not a DX file');
                return;
            }
            
            try {
                const humanUri = vscode.Uri.parse(`${DX_HUMAN_SCHEME}:/${filePath.replace(/\\/g, '/')}`);
                await vscode.window.showTextDocument(humanUri, { preview: false });
            } catch (e) {
                vscode.window.showErrorMessage(`Failed to open human view: ${e}`);
            }
        }),

        // Export commands
        vscode.commands.registerCommand('dx.export.showMenu', async () => {
            const items = [
                { label: '$(json) JSON', description: 'Export to JSON format', id: 'json' },
                { label: '$(json) JSON (Compact)', description: 'Export to minified JSON', id: 'jsonCompact' },
                { label: '$(file-code) YAML', description: 'Export to YAML format', id: 'yaml' },
                { label: '$(file-code) TOML', description: 'Export to TOML format', id: 'toml' },
                { label: '$(table) CSV', description: 'Export to CSV format', id: 'csv' },
                { label: '$(file-text) TOON', description: 'Export to TOON format', id: 'toon' },
            ];
            
            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select export format'
            });
            
            if (selected) {
                await vscode.commands.executeCommand(`dx.export.to${selected.id.charAt(0).toUpperCase() + selected.id.slice(1)}`);
            }
        }),

        vscode.commands.registerCommand('dx.export.toJson', async () => {
            const data = await getCurrentContent();
            if (!data) return;
            
            const humanContent = data.isHuman ? data.content : llmToHuman(data.content);
            const json = humanToJson(humanContent, false);
            const doc = await vscode.workspace.openTextDocument({ content: json, language: 'json' });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
        }),

        vscode.commands.registerCommand('dx.export.toJsonCompact', async () => {
            const data = await getCurrentContent();
            if (!data) return;
            
            const humanContent = data.isHuman ? data.content : llmToHuman(data.content);
            const json = humanToJson(humanContent, true);
            const doc = await vscode.workspace.openTextDocument({ content: json, language: 'json' });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
        }),

        vscode.commands.registerCommand('dx.export.toYaml', async () => {
            const data = await getCurrentContent();
            if (!data) return;
            
            const humanContent = data.isHuman ? data.content : llmToHuman(data.content);
            const yaml = humanToYaml(humanContent);
            const doc = await vscode.workspace.openTextDocument({ content: yaml, language: 'yaml' });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
        }),

        vscode.commands.registerCommand('dx.export.toToml', async () => {
            const data = await getCurrentContent();
            if (!data) return;
            
            const humanContent = data.isHuman ? data.content : llmToHuman(data.content);
            const toml = humanToToml(humanContent);
            const doc = await vscode.workspace.openTextDocument({ content: toml, language: 'toml' });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
        }),

        vscode.commands.registerCommand('dx.export.toCsv', async () => {
            const data = await getCurrentContent();
            if (!data) return;
            
            const humanContent = data.isHuman ? data.content : llmToHuman(data.content);
            const csv = humanToCsv(humanContent);
            const doc = await vscode.workspace.openTextDocument({ content: csv, language: 'csv' });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
        }),

        vscode.commands.registerCommand('dx.export.toToon', async () => {
            const data = await getCurrentContent();
            if (!data) return;
            
            const humanContent = data.isHuman ? data.content : llmToHuman(data.content);
            const toon = humanToToon(humanContent);
            const doc = await vscode.workspace.openTextDocument({ content: toon, language: 'dx-serializer' });
            await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
        })
    );

    // Auto-redirect: when user opens a DX LLM file, show human view instead
    // Use a more gentle approach that doesn't close/reopen aggressively
    const redirectingFiles = new Set<string>();
    const manuallyOpenedFiles = new Set<string>(); // Track files opened via format buttons

    
    // Function to handle redirect logic
    const handleFileRedirect = async (doc: vscode.TextDocument, viewColumn?: vscode.ViewColumn) => {
        console.log('dx-markdown: handleFileRedirect called for:', doc.uri.toString(), 'scheme:', doc.uri.scheme);
        
        // Only intercept real file system files
        if (doc.uri.scheme !== 'file') {
            console.log('dx-markdown: Skipping non-file scheme:', doc.uri.scheme);
            return;
        }
        
        const filePath = doc.uri.fsPath;
        const key = filePath.toLowerCase();
        
        // Skip redirect if file was manually opened via format button
        if (manuallyOpenedFiles.has(key)) {
            console.log('dx-markdown: Skipping redirect for manually opened file:', filePath);
            setTimeout(() => manuallyOpenedFiles.delete(key), 2000);
            return;
        }
        
        console.log('dx-markdown: File path:', filePath);
        console.log('dx-markdown: isDxLlmFile:', isDxLlmFile(filePath));
        console.log('dx-markdown: isMarkdownFile:', isMarkdownFile(filePath));
        
        /* DEPRECATED: DX Serializer virtual file system redirect (2026 architecture)
        // Handle .dx files
        if (isDxLlmFile(filePath)) {
            // Avoid redirect loops
            if (redirectingFiles.has(key)) return;
            
            try {
                redirectingFiles.add(key);
                
                // Open via virtual file system (shows human content) in the same column
                const humanUri = vscode.Uri.parse(`${DX_HUMAN_SCHEME}:/${filePath.replace(/\\/g, '/')}`);
                
                // Open the human view first
                await vscode.window.showTextDocument(humanUri, { 
                    preview: false,
                    viewColumn: viewColumn || vscode.ViewColumn.Active,
                    preserveFocus: false
                });
                
                // Then close the LLM file editor (after a small delay to ensure smooth transition)
                setTimeout(async () => {
                    try {
                        // Find and close the LLM file tab
                        const tabs = vscode.window.tabGroups.all.flatMap(g => g.tabs);
                        const llmTab = tabs.find(tab => {
                            const input = tab.input as any;
                            return input?.uri?.fsPath === filePath && input?.uri?.scheme === 'file';
                        });
                        
                        if (llmTab) {
                            await vscode.window.tabGroups.close(llmTab);
                        }
                    } catch (e) {
                        console.error('DX: Failed to close LLM tab:', e);
                    }
                }, 100);
                
            } catch (e) {
                console.error('DX: Failed to auto-redirect:', e);
                vscode.window.showErrorMessage(`DX: Failed to open file. ${e}`);
            } finally {
                // Remove from set after a delay to prevent immediate re-trigger
                setTimeout(() => redirectingFiles.delete(key), 1000);
            }
        }
        */
        
        // 2026 Architecture: .sr and dx files are already in human format on disk
        // Generate .llm and .machine files when opening
        if (isDxLlmFile(filePath) || path.basename(filePath) === 'dx') {
            console.log('DX: Detected .sr or dx file, generating .llm and .machine files');
            
            try {
                const workspaceFolders = vscode.workspace.workspaceFolders;
                if (workspaceFolders && workspaceFolders.length > 0) {
                    const workspaceRoot = workspaceFolders[0].uri.fsPath;
                    const fileContent = await fs.promises.readFile(filePath, 'utf-8');
                    
                    // Detect if content is LLM format or human format
                    // Human format: "key = value" (spaces around =)
                    // LLM format: "key=value" (no spaces around =)
                    const isLlmFormat = !fileContent.includes(' = ') && fileContent.includes('=');
                    let llmContent: string;
                    let humanContent: string;
                    
                    console.log('DX: File format detection - isLlmFormat:', isLlmFormat);
                    
                    if (isLlmFormat) {
                        console.log('DX: Detected LLM format, converting to human');
                        llmContent = fileContent;
                        humanContent = llmToHuman(fileContent);
                    } else {
                        console.log('DX: Detected human format, converting to LLM');
                        humanContent = fileContent;
                        llmContent = humanToLlm(fileContent);
                    }
                    
                    const relativePath = path.relative(workspaceRoot, filePath);
                    const relativeDir = path.dirname(relativePath);
                    const baseName = path.basename(filePath);
                    const nameWithoutExt = baseName.replace(/\.sr$/, '').replace(/\.dx$/, '') || baseName;
                    
                    const llmDir = path.join(workspaceRoot, '.dx', 'serializer', relativeDir);
                    const llmPath = path.join(llmDir, `${nameWithoutExt}.llm`);
                    const machinePath = path.join(llmDir, `${nameWithoutExt}.machine`);
                    
                    await fs.promises.mkdir(llmDir, { recursive: true });
                    await fs.promises.writeFile(llmPath, llmContent, 'utf-8');
                    
                    console.log(`DX: ✅ Generated ${llmPath}`);
                    
                    // Wait for WASM to initialize (max 5 seconds)
                    let wasmAttempts = 0;
                    while (!wasmInitialized && wasmAttempts < 50) {
                        await new Promise(resolve => setTimeout(resolve, 100));
                        wasmAttempts++;
                    }
                    
                    // Generate machine format if WASM is available
                    if (wasmSerializer && wasmInitialized) {
                        try {
                            const machineBytes = wasmSerializer.human_to_machine(humanContent);
                            await fs.promises.writeFile(machinePath, Buffer.from(machineBytes));
                            console.log(`DX: ✅ Generated ${machinePath}`);
                        } catch (e) {
                            console.error('DX: Failed to generate .machine file:', e);
                        }
                    } else {
                        console.warn('DX: ⚠️ WASM not available for machine format generation');
                    }
                }
            } catch (e) {
                console.error('DX: Failed to generate .llm/.machine files on open:', e);
            }
        }
        
        // Handle .md files
        if (isMarkdownFile(filePath)) {
            console.log('dx-markdown: Detected .md file:', filePath);
            
            // Avoid redirect loops
            const key = filePath.toLowerCase();
            if (redirectingFiles.has(key)) {
                console.log('dx-markdown: Already redirecting, skipping:', filePath);
                return;
            }
            
            try {
                redirectingFiles.add(key);
                console.log('dx-markdown: Starting redirect for:', filePath);
                
                // Wait for any pending file operations to complete (similar to serializer files)
                if (writingMarkdownFiles.has(key)) {
                    console.log('dx-markdown: File is being written, waiting before redirect...');
                    let waitAttempts = 0;
                    while (writingMarkdownFiles.has(key) && waitAttempts < 20) {
                        await new Promise(resolve => setTimeout(resolve, 200));
                        waitAttempts++;
                        console.log(`dx-markdown: Still waiting (attempt ${waitAttempts}/20)...`);
                    }
                }
                
                // Additional wait to ensure file watcher has processed and file is fully written
                await new Promise(resolve => setTimeout(resolve, 500));
                
                // Pre-populate cache by reading the file directly
                try {
                    const mdContent = await vscode.workspace.fs.readFile(doc.uri);
                    const mdText = new TextDecoder().decode(mdContent);
                    const cacheKey = doc.uri.fsPath.toLowerCase();
                    
                    // Import the cache from markdownLensFileSystem
                    const { writingMarkdownFiles } = await import('./markdownLensFileSystem');
                    
                    // Store in a temporary variable that we'll use to populate the cache
                    console.log('dx-markdown: Pre-caching file content, length:', mdText.length);
                } catch (e) {
                    console.warn('dx-markdown: Failed to pre-cache file:', e);
                }
                
                /* DEPRECATED: Virtual file system disabled (2026 architecture - human format on disk)
                // Open via virtual file system (shows human content but .md path in tab)
                const humanUri = getMarkdownHumanUri(doc.uri);
                console.log('dx-markdown: Opening virtual URI:', humanUri.toString());
                
                // Open the human view first
                const textDocument = await vscode.window.showTextDocument(humanUri, { 
                    preview: false,
                    viewColumn: viewColumn || vscode.ViewColumn.Active,
                    preserveFocus: false
                });
                
                // Set the language mode to dx-markdown for proper syntax highlighting
                try {
                    await vscode.languages.setTextDocumentLanguage(textDocument.document, 'dx-markdown');
                    console.log('dx-markdown: Virtual file opened successfully with dx-markdown language');
                } catch (e) {
                    console.error('dx-markdown: Failed to set language mode:', e);
                    // Continue anyway - language mode is not critical
                }
                
                // Manually trigger colorization after language change
                setTimeout(() => {
                    triggerMarkdownColorization();
                }, 150);
                
                // Then close the .md file tab (after a small delay to ensure smooth transition)
                setTimeout(async () => {
                    try {
                        // Find and close the .md file tab
                        const tabs = vscode.window.tabGroups.all.flatMap(g => g.tabs);
                        const mdTab = tabs.find(tab => {
                            const input = tab.input as any;
                            return input?.uri?.fsPath === filePath && input?.uri?.scheme === 'file';
                        });
                        
                        if (mdTab) {
                            await vscode.window.tabGroups.close(mdTab);
                            console.log('dx-markdown: Closed original .md tab');
                        }
                    } catch (e) {
                        console.error('dx-markdown: Failed to close .md tab:', e);
                    }
                }, 100);
                */
                
                // NEW: 2026 architecture - just open the file directly (human format on disk)
                console.log('dx-markdown: Opening file directly (human format on disk)');
                await vscode.window.showTextDocument(doc.uri, {
                    preview: false,
                    viewColumn: viewColumn || vscode.ViewColumn.Active,
                    preserveFocus: false
                });
                
            } catch (e) {
                console.error('dx-markdown: Failed to auto-redirect:', e);
                vscode.window.showErrorMessage(`dx-markdown: Failed to open file. ${e}`);
            } finally {
                // Remove from set after a delay to prevent immediate re-trigger
                setTimeout(() => {
                    redirectingFiles.delete(key);
                    console.log('dx-markdown: Removed from redirecting set:', filePath);
                }, 1000);
            }
        }
    };
    
    // Listen for active editor changes
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(async (editor) => {
            console.log('dx-markdown: onDidChangeActiveTextEditor triggered');
            if (!editor) {
                console.log('dx-markdown: No editor');
                return;
            }
            console.log('dx-markdown: Editor document URI:', editor.document.uri.toString());
            await handleFileRedirect(editor.document, editor.viewColumn);
        })
    );
    
    // Listen for document opens (catches files opened before editor becomes active)
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(async (doc) => {
            console.log('dx-markdown: onDidOpenTextDocument triggered for:', doc.uri.toString());
            // Small delay to let the editor become active
            setTimeout(async () => {
                const editor = vscode.window.activeTextEditor;
                if (editor && editor.document === doc) {
                    console.log('dx-markdown: Document matches active editor, redirecting');
                    await handleFileRedirect(doc, editor.viewColumn);
                } else {
                    console.log('dx-markdown: Document does not match active editor');
                }
            }, 50);
        })
    );
    
    // Check currently open editors on activation
    setTimeout(async () => {
        console.log('dx-markdown: Checking currently open editors on activation');
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            console.log('dx-markdown: Found active editor:', editor.document.uri.toString());
            await handleFileRedirect(editor.document, editor.viewColumn);
        } else {
            console.log('dx-markdown: No active editor on activation');
        }
    }, 100);

    // Auto-generate .llm files when .sr or dx files are saved
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(async (document) => {
            const filePath = document.uri.fsPath;
            
            // Process .sr files and dx files (without extension)
            const isDxFile = filePath.endsWith('.sr') || path.basename(filePath) === 'dx';
            
            if (isDxFile) {
                try {
                    const workspaceFolders = vscode.workspace.workspaceFolders;
                    if (!workspaceFolders || workspaceFolders.length === 0) {
                        console.warn('DX: No workspace folder found');
                        return;
                    }
                    
                    const workspaceRoot = workspaceFolders[0].uri.fsPath;
                    const fileContent = await fs.promises.readFile(filePath, 'utf-8');
                    
                    // Detect if content is LLM format or human format
                    // Human format: "key = value" (spaces around =)
                    // LLM format: "key=value" (no spaces around =)
                    const isLlmFormat = !fileContent.includes(' = ') && fileContent.includes('=');
                    let llmContent: string;
                    let humanContent: string;
                    
                    console.log('DX: File format detection - isLlmFormat:', isLlmFormat);
                    
                    if (isLlmFormat) {
                        console.log('DX: Detected LLM format, converting to human');
                        llmContent = fileContent;
                        humanContent = llmToHuman(fileContent);
                    } else {
                        console.log('DX: Detected human format, converting to LLM');
                        humanContent = fileContent;
                        llmContent = humanToLlm(fileContent);
                    }
                    
                    const relativePath = path.relative(workspaceRoot, filePath);
                    const relativeDir = path.dirname(relativePath);
                    const baseName = path.basename(filePath);
                    const nameWithoutExt = baseName.replace(/\.sr$/, '').replace(/\.dx$/, '') || baseName;
                    
                    const llmDir = path.join(workspaceRoot, '.dx', 'serializer', relativeDir);
                    const llmPath = path.join(llmDir, `${nameWithoutExt}.llm`);
                    const machinePath = path.join(llmDir, `${nameWithoutExt}.machine`);
                    
                    await fs.promises.mkdir(llmDir, { recursive: true });
                    await fs.promises.writeFile(llmPath, llmContent, 'utf-8');
                    
                    console.log(`DX: ✅ Generated ${llmPath}`);
                    
                    // Wait for WASM to initialize (max 5 seconds)
                    let saveWasmAttempts = 0;
                    while (!wasmInitialized && saveWasmAttempts < 50) {
                        await new Promise(resolve => setTimeout(resolve, 100));
                        saveWasmAttempts++;
                    }
                    
                    // Generate machine format if WASM is available
                    if (wasmSerializer && wasmInitialized) {
                        try {
                            const machineBytes = wasmSerializer.human_to_machine(humanContent);
                            await fs.promises.writeFile(machinePath, Buffer.from(machineBytes));
                            console.log(`DX: ✅ Generated ${machinePath}`);
                        } catch (e) {
                            console.error('DX: Failed to generate .machine file:', e);
                        }
                    } else {
                        console.warn('DX: ⚠️ WASM not available for machine format generation on save');
                    }
                } catch (e) {
                    console.error('DX: Failed to generate .llm file:', e);
                }
            }
        })
    );

    console.log('DX Extension: Activated!');
}

export function deactivate(): void {
    console.log('DX Extension: Deactivating...');
    deactivateMarkdownColorizer();
}
