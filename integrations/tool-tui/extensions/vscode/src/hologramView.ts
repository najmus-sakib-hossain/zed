/**
 * Hologram View: Enhanced Markdown editing experience
 * 
 * Provides a live preview pane showing rendered HTML for Markdown documents,
 * along with syntax highlighting and code folding support.
 * 
 * Requirements: 6.1, 6.2, 6.5, 6.6, 11.5, 11.6
 */

import * as vscode from 'vscode';
import { BinaryCacheManager } from './binaryCache';

/**
 * Hologram View display mode
 */
export type HologramViewMode = 'preview' | 'split' | 'off';

/**
 * Hologram View Provider
 * 
 * Provides a webview panel for live preview of Markdown documents.
 */
export class HologramViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'dx.hologramView';

    private view?: vscode.WebviewView;
    private currentDocument?: vscode.TextDocument;
    private disposables: vscode.Disposable[] = [];
    private mode: HologramViewMode = 'preview';

    constructor(
        private readonly extensionUri: vscode.Uri,
        private readonly cacheManager: BinaryCacheManager
    ) {
        // Load initial mode from settings
        const config = vscode.workspace.getConfiguration('dx');
        this.mode = config.get<HologramViewMode>('hologramView.mode', 'preview');

        // Listen for configuration changes (Requirements: 11.6)
        this.disposables.push(
            vscode.workspace.onDidChangeConfiguration((event) => {
                if (event.affectsConfiguration('dx.hologramView.mode')) {
                    this.handleSettingsChange();
                }
            })
        );
    }

    /**
     * Handle settings change - apply changes without restart
     * Requirements: 11.6
     */
    private handleSettingsChange(): void {
        const config = vscode.workspace.getConfiguration('dx');
        const newMode = config.get<HologramViewMode>('hologramView.mode', 'preview');

        if (newMode !== this.mode) {
            this.mode = newMode;

            if (this.mode === 'off' && this.view) {
                // Clear the view when disabled
                this.view.webview.html = this.getHtmlContent('<div class="empty-state">Hologram View is disabled</div>');
            } else if (this.currentDocument) {
                // Refresh the preview
                this.updatePreview(this.currentDocument);
            }
        }
    }

    /**
     * Get the current display mode
     */
    getMode(): HologramViewMode {
        return this.mode;
    }

    /**
     * Resolve the webview view
     */
    resolveWebviewView(
        webviewView: vscode.WebviewView,
        _context: vscode.WebviewViewResolveContext,
        _token: vscode.CancellationToken
    ): void {
        this.view = webviewView;

        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.extensionUri],
        };

        // Set initial content
        webviewView.webview.html = this.getHtmlContent('');

        // Update when active editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                if (editor && this.isMarkdownFile(editor.document)) {
                    this.updatePreview(editor.document);
                }
            })
        );

        // Update when document changes
        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                if (this.currentDocument && event.document === this.currentDocument) {
                    this.updatePreview(event.document);
                }
            })
        );

        // Initial update if there's an active Markdown file
        const activeEditor = vscode.window.activeTextEditor;
        if (activeEditor && this.isMarkdownFile(activeEditor.document)) {
            this.updatePreview(activeEditor.document);
        }
    }

    /**
     * Update the preview with the current document
     */
    updatePreview(document: vscode.TextDocument): void {
        if (!this.view) {
            return;
        }

        // Check if hologram view is disabled
        if (this.mode === 'off') {
            this.view.webview.html = this.getHtmlContent('<div class="empty-state">Hologram View is disabled. Enable it in settings.</div>');
            return;
        }

        this.currentDocument = document;
        const content = document.getText();
        const html = this.renderMarkdownToHtml(content);
        this.view.webview.html = this.getHtmlContent(html);
    }

    /**
     * Check if a document is a Markdown file
     */
    private isMarkdownFile(document: vscode.TextDocument): boolean {
        return document.uri.fsPath.endsWith('.md');
    }

    /**
     * Render Markdown content to HTML
     */
    private renderMarkdownToHtml(content: string): string {
        // Simple Markdown to HTML conversion
        // This is a basic implementation - could be enhanced with full parser
        let html = '';
        const lines = content.split('\n');

        for (const line of lines) {
            // Headers: N|Title
            const headerMatch = line.match(/^(\d)\|(.+)$/);
            if (headerMatch) {
                const level = parseInt(headerMatch[1], 10);
                const text = this.escapeHtml(headerMatch[2]);
                html += `<h${level}>${text}</h${level}>\n`;
                continue;
            }

            // Code blocks: @lang ... @
            if (line.startsWith('@') && !line.startsWith('@markdown')) {
                const lang = line.slice(1).trim();
                html += `<pre><code class="language-${lang}">`;
                continue;
            }
            if (line === '@') {
                html += '</code></pre>\n';
                continue;
            }

            // Paragraphs
            if (line.trim()) {
                const processed = this.processInlineStyles(line);
                html += `<p>${processed}</p>\n`;
            }
        }

        return html;
    }

    /**
     * Process inline styles in text
     */
    private processInlineStyles(text: string): string {
        let result = this.escapeHtml(text);

        // Bold: text! -> <strong>text</strong>
        result = result.replace(/(\w+)!/g, '<strong>$1</strong>');

        // Italic: text/ -> <em>text</em>
        result = result.replace(/(\w+)\//g, '<em>$1</em>');

        // Code: `text` -> <code>text</code>
        result = result.replace(/`([^`]+)`/g, '<code>$1</code>');

        // Links: [text](url) -> <a href="url">text</a>
        result = result.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');

        return result;
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
            .replace(/'/g, '&#39;');
    }

    /**
     * Get the HTML content for the webview
     */
    private getHtmlContent(bodyContent: string): string {
        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Markdown Preview</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            font-size: var(--vscode-font-size);
            color: var(--vscode-foreground);
            background-color: var(--vscode-editor-background);
            padding: 16px;
            line-height: 1.6;
        }
        h1, h2, h3, h4, h5, h6 {
            color: var(--vscode-editor-foreground);
            margin-top: 1.5em;
            margin-bottom: 0.5em;
        }
        h1 { font-size: 2em; border-bottom: 1px solid var(--vscode-panel-border); }
        h2 { font-size: 1.5em; }
        h3 { font-size: 1.25em; }
        p {
            margin: 0.5em 0;
        }
        pre {
            background-color: var(--vscode-textCodeBlock-background);
            padding: 12px;
            border-radius: 4px;
            overflow-x: auto;
        }
        code {
            font-family: var(--vscode-editor-font-family);
            background-color: var(--vscode-textCodeBlock-background);
            padding: 2px 4px;
            border-radius: 2px;
        }
        pre code {
            background: none;
            padding: 0;
        }
        a {
            color: var(--vscode-textLink-foreground);
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        strong {
            font-weight: bold;
        }
        em {
            font-style: italic;
        }
        table {
            border-collapse: collapse;
            width: 100%;
            margin: 1em 0;
        }
        th, td {
            border: 1px solid var(--vscode-panel-border);
            padding: 8px;
            text-align: left;
        }
        th {
            background-color: var(--vscode-editor-lineHighlightBackground);
        }
        blockquote {
            border-left: 4px solid var(--vscode-textBlockQuote-border);
            margin: 1em 0;
            padding-left: 1em;
            color: var(--vscode-textBlockQuote-foreground);
        }
        .empty-state {
            text-align: center;
            color: var(--vscode-descriptionForeground);
            padding: 2em;
        }
    </style>
</head>
<body>
    ${bodyContent || '<div class="empty-state">Open a .md file to see preview</div>'}
</body>
</html>`;
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
 * Register the Hologram View provider
 */
export function registerHologramView(
    context: vscode.ExtensionContext,
    cacheManager: BinaryCacheManager
): HologramViewProvider {
    const provider = new HologramViewProvider(context.extensionUri, cacheManager);

    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            HologramViewProvider.viewType,
            provider
        )
    );

    context.subscriptions.push(provider);

    return provider;
}
