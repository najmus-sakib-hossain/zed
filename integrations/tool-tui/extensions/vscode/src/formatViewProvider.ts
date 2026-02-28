/**
 * Format View Provider for DX Serializer Files
 * 
 * Provides icons in the file tab top-right to switch between:
 * - Human format (default, readable)
 * - LLM format (token-optimized)
 * - Machine format (binary)
 * 
 * Applies to: dx, .sr files
 */

import * as vscode from 'vscode';
import { parseLlm, DxDocument } from './llmParser';
import { formatDocument } from './humanFormatter';
import { parseHuman, serializeToLlm } from './humanParser';
import { detectFormat } from './formatDetector';

export type SerializerFormat = 'human' | 'llm' | 'machine';

interface FormatState {
    originalContent: string;
    currentFormat: SerializerFormat;
    document: DxDocument | null;
}

// Track format state per file
const formatStates = new Map<string, FormatState>();

/**
 * Get current format for a file
 */
export function getCurrentFormat(uri: vscode.Uri): SerializerFormat {
    const state = formatStates.get(uri.toString());
    return state?.currentFormat || 'human';
}

/**
 * Check if a file is a DX serializer file or markdown file
 */
export function isDxSerializerFile(uri: vscode.Uri): boolean {
    // Check both fsPath and uri.path to handle virtual file systems
    const fsPath = uri.fsPath.toLowerCase();
    const uriPath = uri.path.toLowerCase();
    
    // Check if it's a markdown file (including virtual markdown human scheme)
    if (uri.scheme === 'markdownhuman' || fsPath.endsWith('.md') || uriPath.endsWith('.md')) {
        return true;
    }
    
    // Check if it's a DX serializer file
    if (uri.scheme === 'dxhuman' || fsPath.endsWith('.sr') || uriPath.endsWith('.sr')) {
        return true;
    }
    
    // Check if it's a .llm or .machine cache file
    if (fsPath.endsWith('.llm') || uriPath.endsWith('.llm') || 
        fsPath.endsWith('.machine') || uriPath.endsWith('.machine')) {
        return true;
    }
    
    // Check if it's a 'dx' file without extension (check basename)
    const basename = uriPath.split('/').pop() || '';
    if (basename === 'dx') {
        return true;
    }
    
    return false;
}

/**
 * Convert content to specified format
 */
export function convertToFormat(content: string, targetFormat: SerializerFormat): string {
    const detection = detectFormat(content);
    
    // Parse the content first
    let doc: DxDocument | null = null;
    
    if (detection.format === 'llm') {
        const result = parseLlm(content);
        if (result.success && result.document) {
            doc = result.document;
        }
    } else {
        const result = parseHuman(content);
        if (result.success && result.document) {
            doc = result.document;
        }
    }
    
    if (!doc) {
        return content; // Return original if parsing fails
    }
    
    switch (targetFormat) {
        case 'human':
            return formatDocument(doc);
        case 'llm':
            return serializeToLlm(doc);
        case 'machine':
            // Machine format is binary - for now return LLM with header
            return `DXMB\x00\x01${serializeToLlm(doc)}`;
        default:
            return content;
    }
}

/**
 * Format View Status Bar Provider
 */
export class FormatViewStatusBar implements vscode.Disposable {
    // private humanButton: vscode.StatusBarItem; // DEPRECATED: 2026 architecture
    private llmButton: vscode.StatusBarItem;
    private machineButton: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];

    constructor() {
        // Create status bar items (right side, high priority to appear together)
        /* DEPRECATED: Human format button (2026 architecture - human format on disk)
        this.humanButton = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            200
        );
        this.humanButton.command = 'dx.format.showHuman';
        this.humanButton.tooltip = 'Show Human Format (readable)';
        */

        this.llmButton = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            199
        );
        this.llmButton.command = 'dx.format.showLlm';
        this.llmButton.tooltip = 'Show LLM Format (token-optimized)';

        this.machineButton = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            198
        );
        this.machineButton.command = 'dx.format.showMachine';
        this.machineButton.tooltip = 'Show Machine Format (binary)';

        // Listen for editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                this.updateVisibility(editor);
            })
        );

        // Initial update
        this.updateVisibility(vscode.window.activeTextEditor);
    }

    private updateVisibility(editor: vscode.TextEditor | undefined): void {
        if (!editor || !isDxSerializerFile(editor.document.uri)) {
            this.hide();
            return;
        }

        const currentFormat = getCurrentFormat(editor.document.uri);
        this.updateButtons(currentFormat);
        this.show();
    }

    private updateButtons(currentFormat: SerializerFormat): void {
        /* DEPRECATED: Human button (2026 architecture - human format on disk)
        // Human button
        if (currentFormat === 'human') {
            this.humanButton.text = '$(file-text) Human';
            this.humanButton.backgroundColor = new vscode.ThemeColor(
                'statusBarItem.prominentBackground'
            );
        } else {
            this.humanButton.text = '$(file-text)';
            this.humanButton.backgroundColor = undefined;
        }
        */

        // LLM button
        if (currentFormat === 'llm') {
            this.llmButton.text = '$(symbol-number) LLM';
            this.llmButton.backgroundColor = new vscode.ThemeColor(
                'statusBarItem.prominentBackground'
            );
        } else {
            this.llmButton.text = '$(symbol-number)';
            this.llmButton.backgroundColor = undefined;
        }

        // Machine button
        if (currentFormat === 'machine') {
            this.machineButton.text = '$(file-binary) Machine';
            this.machineButton.backgroundColor = new vscode.ThemeColor(
                'statusBarItem.prominentBackground'
            );
        } else {
            this.machineButton.text = '$(file-binary)';
            this.machineButton.backgroundColor = undefined;
        }
    }

    show(): void {
        // this.humanButton.show(); // DEPRECATED: 2026 architecture
        this.llmButton.show();
        this.machineButton.show();
    }

    hide(): void {
        // this.humanButton.hide(); // DEPRECATED: 2026 architecture
        this.llmButton.hide();
        this.machineButton.hide();
    }

    dispose(): void {
        // this.humanButton.dispose(); // DEPRECATED: 2026 architecture
        this.llmButton.dispose();
        this.machineButton.dispose();
        for (const d of this.disposables) d.dispose();
    }
}

/**
 * Register format view commands
 */
export function registerFormatViewCommands(
    context: vscode.ExtensionContext,
    formatViewStatusBar: FormatViewStatusBar
): void {
    // Commands registered in extension.ts to avoid duplicate registration
    console.log('DX: Format commands already registered in extension.ts');
}


/**
 * Switch the current editor to a different format
 */
async function switchFormat(targetFormat: SerializerFormat): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || !isDxSerializerFile(editor.document.uri)) {
        return;
    }

    const uri = editor.document.uri;
    const content = editor.document.getText();
    
    // Store original if not already stored
    if (!formatStates.has(uri.toString())) {
        formatStates.set(uri.toString(), {
            originalContent: content,
            currentFormat: 'human',
            document: null,
        });
    }

    const state = formatStates.get(uri.toString())!;
    
    // Convert to target format
    const converted = convertToFormat(content, targetFormat);
    
    // Update state
    state.currentFormat = targetFormat;
    
    // Replace editor content
    const fullRange = new vscode.Range(
        editor.document.positionAt(0),
        editor.document.positionAt(content.length)
    );
    
    await editor.edit((editBuilder) => {
        editBuilder.replace(fullRange, converted);
    });
}

