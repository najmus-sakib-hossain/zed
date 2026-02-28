/**
 * Output File Mapping Module
 * 
 * Tracks the relationship between dx-style classnames and their generated CSS
 * in the output file. Provides line number mapping for navigation and display.
 * 
 * **Validates: Requirements 3.3, 3.4**
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Information about a CSS rule's location in the output file
 */
export interface LineInfo {
    /** Starting line number (1-indexed) */
    startLine: number;
    /** Ending line number (1-indexed) */
    endLine: number;
    /** The CSS content for this classname */
    css: string;
}

/**
 * Output file mapping data
 */
export interface OutputFileMapping {
    /** Path to the output CSS file */
    outputPath: string;
    /** Map from classname to line information */
    classToLine: Map<string, LineInfo>;
    /** Last modified timestamp */
    lastModified: number;
}

/**
 * Regex to match CSS class selectors
 * Matches: .classname { ... }
 */
const CSS_CLASS_PATTERN = /^\s*\.([a-zA-Z0-9_-]+(?:\\:[a-zA-Z0-9_-]+)*)\s*\{/;

/**
 * Parse a CSS file and build a mapping from classnames to line numbers
 */
function parseCSSFile(content: string): Map<string, LineInfo> {
    const mapping = new Map<string, LineInfo>();
    const lines = content.split('\n');

    let currentClassname: string | null = null;
    let currentStartLine = 0;
    let currentCSSLines: string[] = [];
    let braceDepth = 0;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNumber = i + 1; // 1-indexed

        // Check for class selector start
        const classMatch = line.match(CSS_CLASS_PATTERN);
        if (classMatch && braceDepth === 0) {
            // Unescape the classname (e.g., hover\:bg-blue-500 -> hover:bg-blue-500)
            currentClassname = classMatch[1].replace(/\\/g, '');
            currentStartLine = lineNumber;
            currentCSSLines = [line];
            braceDepth = (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;

            // Check if rule is on single line
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

        // Track brace depth for multi-line rules
        if (currentClassname !== null) {
            currentCSSLines.push(line);
            braceDepth += (line.match(/\{/g) || []).length;
            braceDepth -= (line.match(/\}/g) || []).length;

            // Rule complete
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


/**
 * Output Mapping Manager
 * 
 * Manages the mapping between classnames and their CSS output locations.
 * Watches for file changes and updates the mapping automatically.
 * 
 * **Validates: Requirements 3.1, 3.3, 3.4**
 */
export class OutputMappingManager implements vscode.Disposable {
    private mapping: OutputFileMapping | null = null;
    private fileWatcher: vscode.FileSystemWatcher | null = null;
    private disposables: vscode.Disposable[] = [];

    constructor() { }

    /**
     * Initialize the mapping manager with an output file path
     */
    async initialize(outputPath: string): Promise<void> {
        await this.loadMapping(outputPath);
        this.setupFileWatcher(outputPath);
    }

    /**
     * Load or reload the mapping from the output file
     */
    async loadMapping(outputPath: string): Promise<void> {
        try {
            const absolutePath = this.resolveOutputPath(outputPath);

            if (!fs.existsSync(absolutePath)) {
                this.mapping = null;
                return;
            }

            const content = await fs.promises.readFile(absolutePath, 'utf-8');
            const stats = await fs.promises.stat(absolutePath);

            this.mapping = {
                outputPath: absolutePath,
                classToLine: parseCSSFile(content),
                lastModified: stats.mtimeMs
            };

            console.log(`DX Style: Loaded mapping for ${this.mapping.classToLine.size} classnames`);
        } catch (error) {
            console.error('DX Style: Failed to load output mapping:', error);
            this.mapping = null;
        }
    }

    /**
     * Set up file watcher for the output CSS file
     * 
     * **Validates: Requirements 3.1**
     */
    private setupFileWatcher(outputPath: string): void {
        // Dispose existing watcher
        if (this.fileWatcher) {
            this.fileWatcher.dispose();
        }

        const absolutePath = this.resolveOutputPath(outputPath);
        const pattern = new vscode.RelativePattern(
            path.dirname(absolutePath),
            path.basename(absolutePath)
        );

        this.fileWatcher = vscode.workspace.createFileSystemWatcher(pattern);

        this.fileWatcher.onDidChange(async () => {
            console.log('DX Style: Output file changed, reloading mapping...');
            await this.loadMapping(outputPath);
        });

        this.fileWatcher.onDidCreate(async () => {
            console.log('DX Style: Output file created, loading mapping...');
            await this.loadMapping(outputPath);
        });

        this.fileWatcher.onDidDelete(() => {
            console.log('DX Style: Output file deleted, clearing mapping...');
            this.mapping = null;
        });

        this.disposables.push(this.fileWatcher);
    }

    /**
     * Resolve the output path to an absolute path
     */
    private resolveOutputPath(outputPath: string): string {
        if (path.isAbsolute(outputPath)) {
            return outputPath;
        }

        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            return path.join(workspaceFolders[0].uri.fsPath, outputPath);
        }

        return outputPath;
    }

    /**
     * Get line information for a classname
     */
    getLineInfo(classname: string): LineInfo | null {
        if (!this.mapping) {
            return null;
        }
        return this.mapping.classToLine.get(classname) || null;
    }

    /**
     * Get the output file path
     */
    getOutputPath(): string | null {
        return this.mapping?.outputPath || null;
    }

    /**
     * Check if the mapping is loaded
     */
    isLoaded(): boolean {
        return this.mapping !== null;
    }

    /**
     * Get all mapped classnames
     */
    getClassnames(): string[] {
        if (!this.mapping) {
            return [];
        }
        return Array.from(this.mapping.classToLine.keys());
    }

    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
        this.mapping = null;
    }
}

// Singleton instance
let outputMappingManager: OutputMappingManager | null = null;

/**
 * Get the output mapping manager instance
 */
export function getOutputMappingManager(): OutputMappingManager {
    if (!outputMappingManager) {
        outputMappingManager = new OutputMappingManager();
    }
    return outputMappingManager;
}

/**
 * Initialize the output mapping manager
 */
export async function initializeOutputMapping(context: vscode.ExtensionContext): Promise<void> {
    const manager = getOutputMappingManager();
    context.subscriptions.push(manager);

    // Try to find the output CSS file from configuration
    const config = vscode.workspace.getConfiguration('dx.style');
    const outputPath = config.get<string>('outputPath', 'dist/styles.css');

    await manager.initialize(outputPath);
}
