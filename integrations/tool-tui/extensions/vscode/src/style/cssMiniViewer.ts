/**
 * Mini CSS Viewer Component
 * 
 * Displays generated CSS with syntax highlighting and inline editing capabilities.
 * Shows CSS code in a webview panel with line numbers and file path.
 * 
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10**
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { getOutputMappingManager, LineInfo } from './outputMapping';

/**
 * Options for the mini CSS viewer
 */
export interface MiniViewerOptions {
    /** Enable inline editing */
    editable: boolean;
    /** Show line numbers */
    showLineNumbers: boolean;
    /** Show file path */
    showFilePath: boolean;
    /** Enable copy button */
    enableCopy: boolean;
}

const DEFAULT_OPTIONS: MiniViewerOptions = {
    editable: true,
    showLineNumbers: true,
    showFilePath: true,
    enableCopy: true
};

/**
 * CSS Mini Viewer
 * 
 * Provides a webview-based component for displaying and editing CSS.
 * 
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
 */
export class CSSMiniViewer implements vscode.Disposable {
    private panel: vscode.WebviewPanel | null = null;
    private currentClassname: string = '';
    private currentCSS: string = '';
    private currentLineInfo: LineInfo | null = null;
    private options: MiniViewerOptions;
    private disposables: vscode.Disposable[] = [];

    constructor(options: Partial<MiniViewerOptions> = {}) {
        this.options = { ...DEFAULT_OPTIONS, ...options };
    }

    /**
     * Show the mini viewer for a classname
     * 
     * **Validates: Requirements 3.1, 3.3, 3.4**
     */
    async show(classname: string): Promise<void> {
        this.currentClassname = classname;

        // Get CSS from output mapping
        const mappingManager = getOutputMappingManager();
        const lineInfo = mappingManager.getLineInfo(classname);
        const outputPath = mappingManager.getOutputPath();

        if (!lineInfo || !outputPath) {
            this.showNotCompiled();
            return;
        }

        this.currentLineInfo = lineInfo;
        this.currentCSS = lineInfo.css;

        // Create or reveal panel
        if (this.panel) {
            this.panel.reveal(vscode.ViewColumn.Beside);
        } else {
            this.createPanel();
        }

        this.updateContent(outputPath);
    }

    /**
     * Create the webview panel
     */
    private createPanel(): void {
        this.panel = vscode.window.createWebviewPanel(
            'cssMiniViewer',
            `CSS: ${this.currentClassname}`,
            vscode.ViewColumn.Beside,
            {
                enableScripts: true,
                retainContextWhenHidden: true
            }
        );

        // Handle messages from webview
        this.panel.webview.onDidReceiveMessage(
            async (message) => {
                switch (message.command) {
                    case 'save':
                        await this.saveChanges(message.css);
                        break;
                    case 'copy':
                        await this.copyToClipboard();
                        break;
                    case 'navigate':
                        await this.navigateToLine();
                        break;
                }
            },
            null,
            this.disposables
        );

        // Handle panel disposal
        this.panel.onDidDispose(() => {
            this.panel = null;
        }, null, this.disposables);
    }


    /**
     * Update the webview content
     */
    private updateContent(outputPath: string): void {
        if (!this.panel) {
            return;
        }

        this.panel.title = `CSS: ${this.currentClassname}`;
        this.panel.webview.html = this.getWebviewContent(outputPath);
    }

    /**
     * Show "not compiled" message
     * 
     * **Validates: Requirements 3.9**
     */
    private showNotCompiled(): void {
        if (!this.panel) {
            this.createPanel();
        }

        if (this.panel) {
            this.panel.title = `CSS: ${this.currentClassname}`;
            this.panel.webview.html = this.getNotCompiledContent();
        }
    }

    /**
     * Get webview HTML content
     * 
     * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.6, 3.7**
     */
    private getWebviewContent(outputPath: string): string {
        const lineNumbers = this.options.showLineNumbers && this.currentLineInfo
            ? this.generateLineNumbers(this.currentLineInfo.startLine, this.currentLineInfo.endLine)
            : '';

        const filePathDisplay = this.options.showFilePath
            ? `<div class="file-path">${path.basename(outputPath)}:${this.currentLineInfo?.startLine || 0}</div>`
            : '';

        const copyButton = this.options.enableCopy
            ? `<button class="copy-btn" onclick="copyCSS()">📋 Copy</button>`
            : '';

        const editableAttr = this.options.editable ? 'contenteditable="true"' : '';

        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CSS Mini Viewer</title>
    <style>
        body {
            font-family: var(--vscode-editor-font-family, 'Consolas', monospace);
            font-size: var(--vscode-editor-font-size, 14px);
            background-color: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
            padding: 10px;
            margin: 0;
        }
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 10px;
            padding-bottom: 8px;
            border-bottom: 1px solid var(--vscode-panel-border);
        }
        .classname {
            font-weight: bold;
            color: var(--vscode-symbolIcon-classForeground);
        }
        .file-path {
            font-size: 0.85em;
            color: var(--vscode-descriptionForeground);
            cursor: pointer;
        }
        .file-path:hover {
            text-decoration: underline;
        }
        .toolbar {
            display: flex;
            gap: 8px;
        }
        .copy-btn, .navigate-btn {
            background: var(--vscode-button-secondaryBackground);
            color: var(--vscode-button-secondaryForeground);
            border: none;
            padding: 4px 8px;
            cursor: pointer;
            border-radius: 3px;
            font-size: 0.85em;
        }
        .copy-btn:hover, .navigate-btn:hover {
            background: var(--vscode-button-secondaryHoverBackground);
        }
        .code-container {
            display: flex;
            background: var(--vscode-editor-background);
            border: 1px solid var(--vscode-panel-border);
            border-radius: 4px;
            overflow: hidden;
        }
        .line-numbers {
            background: var(--vscode-editorLineNumber-background, var(--vscode-editor-background));
            color: var(--vscode-editorLineNumber-foreground);
            padding: 10px 8px;
            text-align: right;
            user-select: none;
            border-right: 1px solid var(--vscode-panel-border);
            font-size: 0.9em;
        }
        .line-numbers div {
            line-height: 1.5;
        }
        .css-content {
            flex: 1;
            padding: 10px;
            white-space: pre-wrap;
            word-wrap: break-word;
            line-height: 1.5;
            outline: none;
        }
        .css-content:focus {
            background: var(--vscode-editor-selectionBackground);
        }
        /* CSS Syntax Highlighting */
        .css-selector { color: var(--vscode-symbolIcon-classForeground, #d7ba7d); }
        .css-property { color: var(--vscode-symbolIcon-propertyForeground, #9cdcfe); }
        .css-value { color: var(--vscode-symbolIcon-stringForeground, #ce9178); }
        .css-punctuation { color: var(--vscode-editor-foreground); }
        
        .status {
            margin-top: 8px;
            font-size: 0.85em;
            color: var(--vscode-descriptionForeground);
        }
        .status.saved {
            color: var(--vscode-testing-iconPassed);
        }
        .status.error {
            color: var(--vscode-testing-iconFailed);
        }
    </style>
</head>
<body>
    <div class="header">
        <span class="classname">.${this.escapeHtml(this.currentClassname)}</span>
        <div class="toolbar">
            ${copyButton}
            <button class="navigate-btn" onclick="navigateToFile()">📄 Open File</button>
        </div>
    </div>
    ${filePathDisplay}
    <div class="code-container">
        ${lineNumbers ? `<div class="line-numbers">${lineNumbers}</div>` : ''}
        <div class="css-content" ${editableAttr} id="cssContent" onblur="saveCSS()" onkeydown="handleKeydown(event)">${this.highlightCSS(this.currentCSS)}</div>
    </div>
    <div class="status" id="status"></div>
    
    <script>
        const vscode = acquireVsCodeApi();
        
        function copyCSS() {
            vscode.postMessage({ command: 'copy' });
            showStatus('Copied to clipboard!', 'saved');
        }
        
        function navigateToFile() {
            vscode.postMessage({ command: 'navigate' });
        }
        
        function saveCSS() {
            const content = document.getElementById('cssContent');
            const css = content.innerText;
            vscode.postMessage({ command: 'save', css: css });
        }
        
        function handleKeydown(event) {
            if (event.ctrlKey && event.key === 's') {
                event.preventDefault();
                saveCSS();
                showStatus('Saved!', 'saved');
            }
        }
        
        function showStatus(message, type) {
            const status = document.getElementById('status');
            status.textContent = message;
            status.className = 'status ' + type;
            setTimeout(() => {
                status.textContent = '';
                status.className = 'status';
            }, 2000);
        }
    </script>
</body>
</html>`;
    }

    /**
     * Get "not compiled" HTML content
     * 
     * **Validates: Requirements 3.9**
     */
    private getNotCompiledContent(): string {
        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CSS Mini Viewer</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            background-color: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
            padding: 20px;
            text-align: center;
        }
        .message {
            margin-top: 40px;
        }
        .icon {
            font-size: 48px;
            margin-bottom: 16px;
        }
        .title {
            font-size: 1.2em;
            margin-bottom: 8px;
        }
        .description {
            color: var(--vscode-descriptionForeground);
        }
    </style>
</head>
<body>
    <div class="message">
        <div class="icon">⚠️</div>
        <div class="title">Styles Not Compiled</div>
        <div class="description">
            No generated CSS found for <strong>.${this.escapeHtml(this.currentClassname)}</strong>.<br>
            Run your build process to generate the output CSS file.
        </div>
    </div>
</body>
</html>`;
    }

    /**
     * Generate line numbers HTML
     */
    private generateLineNumbers(start: number, end: number): string {
        const lines: string[] = [];
        for (let i = start; i <= end; i++) {
            lines.push(`<div>${i}</div>`);
        }
        return lines.join('');
    }

    /**
     * Basic CSS syntax highlighting
     */
    private highlightCSS(css: string): string {
        // Escape HTML first
        let highlighted = this.escapeHtml(css);

        // Highlight selectors (class names)
        highlighted = highlighted.replace(
            /(\.[a-zA-Z0-9_-]+(?:\\:[a-zA-Z0-9_-]+)*)/g,
            '<span class="css-selector">$1</span>'
        );

        // Highlight properties
        highlighted = highlighted.replace(
            /([a-z-]+)(\s*:)/g,
            '<span class="css-property">$1</span>$2'
        );

        // Highlight values (after colon, before semicolon)
        highlighted = highlighted.replace(
            /(:\s*)([^;{}]+)(;)/g,
            '$1<span class="css-value">$2</span>$3'
        );

        return highlighted;
    }

    /**
     * Escape HTML special characters
     */
    private escapeHtml(text: string): string {
        return text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#039;');
    }

    /**
     * Save changes to the output file
     * 
     * **Validates: Requirements 3.8**
     */
    async saveChanges(newCSS: string): Promise<void> {
        if (!this.currentLineInfo) {
            return;
        }

        const mappingManager = getOutputMappingManager();
        const outputPath = mappingManager.getOutputPath();

        if (!outputPath) {
            vscode.window.showErrorMessage('DX Style: Output file not found');
            return;
        }

        try {
            // Read the current file
            const content = await fs.promises.readFile(outputPath, 'utf-8');
            const lines = content.split('\n');

            // Replace the CSS at the specified lines
            const newLines = newCSS.split('\n');
            lines.splice(
                this.currentLineInfo.startLine - 1,
                this.currentLineInfo.endLine - this.currentLineInfo.startLine + 1,
                ...newLines
            );

            // Write back
            await fs.promises.writeFile(outputPath, lines.join('\n'), 'utf-8');

            // Update current state
            this.currentCSS = newCSS;
            this.currentLineInfo.css = newCSS;
            this.currentLineInfo.endLine = this.currentLineInfo.startLine + newLines.length - 1;

            console.log('DX Style: CSS saved successfully');
        } catch (error) {
            vscode.window.showErrorMessage(`DX Style: Failed to save CSS: ${error}`);
        }
    }

    /**
     * Copy CSS to clipboard
     * 
     * **Validates: Requirements 3.10**
     */
    async copyToClipboard(): Promise<void> {
        await vscode.env.clipboard.writeText(this.currentCSS);
    }

    /**
     * Navigate to the line in the output file
     * 
     * **Validates: Requirements 3.5**
     */
    async navigateToLine(): Promise<void> {
        if (!this.currentLineInfo) {
            return;
        }

        const mappingManager = getOutputMappingManager();
        const outputPath = mappingManager.getOutputPath();

        if (!outputPath) {
            return;
        }

        const uri = vscode.Uri.file(outputPath);
        const document = await vscode.workspace.openTextDocument(uri);
        const editor = await vscode.window.showTextDocument(document, vscode.ViewColumn.One);

        // Navigate to the line
        const position = new vscode.Position(this.currentLineInfo.startLine - 1, 0);
        editor.selection = new vscode.Selection(position, position);
        editor.revealRange(
            new vscode.Range(position, position),
            vscode.TextEditorRevealType.InCenter
        );
    }

    dispose(): void {
        if (this.panel) {
            this.panel.dispose();
        }
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }
}

// Singleton instance
let cssMiniViewer: CSSMiniViewer | null = null;

/**
 * Get the CSS mini viewer instance
 */
export function getCSSMiniViewer(): CSSMiniViewer {
    if (!cssMiniViewer) {
        cssMiniViewer = new CSSMiniViewer();
    }
    return cssMiniViewer;
}

/**
 * Register the CSS mini viewer command
 */
export function registerCSSMiniViewerCommand(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.style.showCSSViewer', async (classname?: string) => {
            if (!classname) {
                // Try to get classname from current selection
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                    const selection = editor.selection;
                    classname = editor.document.getText(selection);
                }
            }

            if (classname) {
                const viewer = getCSSMiniViewer();
                await viewer.show(classname);
            } else {
                vscode.window.showWarningMessage('DX Style: No classname selected');
            }
        })
    );

    console.log('DX Style: CSS Mini Viewer command registered');
}
