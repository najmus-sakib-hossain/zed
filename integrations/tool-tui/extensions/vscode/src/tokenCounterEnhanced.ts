/**
 * Enhanced Token Counter Status Bar
 * 
 * Displays comprehensive token efficiency metrics in the VS Code status bar.
 * Shows savings percentage and provides detailed panel on click.
 * 
 * Requirements: 3.1, 3.2, 3.3, 7.1-7.6
 */

import * as vscode from 'vscode';
import { TokenEfficiencyService, EfficiencyReport } from './tokenEfficiencyService';
import { FormatConverterService } from './formatConverterService';

/**
 * Enhanced Token Counter Status Bar
 * 
 * Provides:
 * - Status bar showing token count and savings percentage
 * - Detailed panel with all model counts and format comparisons
 * - Copy buttons for all 5 formats
 * - Debounced updates on document change
 */
export class TokenCounterEnhancedStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];
    private tokenService: TokenEfficiencyService;
    private formatConverter: FormatConverterService;
    private currentReport: EfficiencyReport | null = null;
    private updateTimeout: NodeJS.Timeout | null = null;
    private panel: vscode.WebviewPanel | null = null;

    // Debounce delay in milliseconds
    private static readonly DEBOUNCE_MS = 500;

    constructor() {
        this.formatConverter = new FormatConverterService();
        this.tokenService = new TokenEfficiencyService(this.formatConverter);

        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            98
        );
        this.statusBarItem.command = 'dx.showTokenEfficiencyPanel';

        // Listen for active editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                this.updateForEditor(editor);
            })
        );

        // Listen for document changes with debouncing
        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                const activeEditor = vscode.window.activeTextEditor;
                if (activeEditor && event.document === activeEditor.document) {
                    this.debouncedUpdate(activeEditor);
                }
            })
        );

        // Initial update
        this.updateForEditor(vscode.window.activeTextEditor);
    }

    /**
     * Initialize WASM bindings if available
     */
    async initWasm(wasmModule: any): Promise<void> {
        await this.tokenService.initWasm(wasmModule);
        await this.formatConverter.initWasm(wasmModule);
    }

    /**
     * Debounced update to avoid excessive recalculation
     */
    private debouncedUpdate(editor: vscode.TextEditor): void {
        if (this.updateTimeout) {
            clearTimeout(this.updateTimeout);
        }

        this.updateTimeout = setTimeout(() => {
            this.updateForEditor(editor);
        }, TokenCounterEnhancedStatusBar.DEBOUNCE_MS);
    }

    /**
     * Update for the current editor
     */
    private async updateForEditor(editor: vscode.TextEditor | undefined): Promise<void> {
        if (!editor || !this.isDxFile(editor.document)) {
            this.hide();
            return;
        }

        const content = editor.document.getText();

        try {
            this.currentReport = await this.tokenService.getEfficiencyReport(content);
            this.updateStatusBar(this.currentReport);
        } catch (error) {
            console.error('Token efficiency calculation failed:', error);
            this.statusBarItem.text = '$(symbol-number) Error';
            this.statusBarItem.tooltip = 'Failed to calculate token efficiency';
            this.statusBarItem.show();
        }
    }

    /**
     * Update status bar display
     */
    private updateStatusBar(report: EfficiencyReport): void {
        const dxCount = report.dxTokens.openai.count;
        const bestSavings = Math.max(
            report.savings.vsJson,
            report.savings.vsYaml,
            report.savings.vsToml,
            report.savings.vsToon
        );

        const savingsText = bestSavings > 0 ? `+${bestSavings}%` : `${bestSavings}%`;

        this.statusBarItem.text = `$(symbol-number) ${dxCount} tokens (${savingsText})`;
        this.statusBarItem.tooltip = this.createTooltip(report);
        this.statusBarItem.show();
    }

    /**
     * Create tooltip text
     */
    private createTooltip(report: EfficiencyReport): string {
        return `DX Token Efficiency
━━━━━━━━━━━━━━━━━━━━
DX: ${report.dxTokens.openai.count} tokens

Savings vs:
• JSON: ${report.savings.vsJson}%
• YAML: ${report.savings.vsYaml}%
• TOML: ${report.savings.vsToml}%
• TOON: ${report.savings.vsToon}%

Click for detailed panel`;
    }

    /**
     * Check if a document is a DX file
     */
    private isDxFile(document: vscode.TextDocument): boolean {
        const filename = path.basename(document.uri.fsPath);
        return filename === 'dx' || document.uri.fsPath.endsWith('.machine');
    }

    /**
     * Show detailed efficiency panel
     */
    async showDetailPanel(): Promise<void> {
        if (!this.currentReport) {
            vscode.window.showWarningMessage('No token efficiency data available');
            return;
        }

        if (this.panel) {
            this.panel.reveal();
            this.updatePanelContent();
            return;
        }

        this.panel = vscode.window.createWebviewPanel(
            'dxTokenEfficiency',
            'DX Token Efficiency',
            vscode.ViewColumn.Beside,
            {
                enableScripts: true,
                retainContextWhenHidden: true,
            }
        );

        this.panel.onDidDispose(() => {
            this.panel = null;
        });

        // Handle messages from webview
        this.panel.webview.onDidReceiveMessage(async (message) => {
            if (message.command === 'copy') {
                await this.copyFormat(message.format);
            }
        });

        this.updatePanelContent();
    }

    /**
     * Update panel content
     */
    private updatePanelContent(): void {
        if (!this.panel || !this.currentReport) return;

        this.panel.webview.html = this.getPanelHtml(this.currentReport);
    }

    /**
     * Generate panel HTML
     */
    private getPanelHtml(report: EfficiencyReport): string {
        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DX Token Efficiency</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            padding: 20px;
            color: var(--vscode-foreground);
            background-color: var(--vscode-editor-background);
        }
        h1 {
            font-size: 1.5em;
            margin-bottom: 20px;
            border-bottom: 1px solid var(--vscode-panel-border);
            padding-bottom: 10px;
        }
        h2 {
            font-size: 1.2em;
            margin-top: 20px;
            margin-bottom: 10px;
        }
        .section {
            margin-bottom: 20px;
            padding: 15px;
            background-color: var(--vscode-editor-inactiveSelectionBackground);
            border-radius: 4px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 10px;
        }
        th, td {
            padding: 8px;
            text-align: left;
            border-bottom: 1px solid var(--vscode-panel-border);
        }
        th {
            font-weight: bold;
        }
        .savings-positive {
            color: var(--vscode-testing-iconPassed);
        }
        .savings-negative {
            color: var(--vscode-testing-iconFailed);
        }
        .copy-btn {
            background-color: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
            border: none;
            padding: 6px 12px;
            border-radius: 4px;
            cursor: pointer;
            margin-right: 8px;
            margin-bottom: 8px;
        }
        .copy-btn:hover {
            background-color: var(--vscode-button-hoverBackground);
        }
        .format-preview {
            background-color: var(--vscode-textCodeBlock-background);
            padding: 10px;
            border-radius: 4px;
            font-family: var(--vscode-editor-font-family);
            font-size: 12px;
            overflow-x: auto;
            max-height: 200px;
            overflow-y: auto;
            white-space: pre;
        }
    </style>
</head>
<body>
    <h1>DX Token Efficiency Report</h1>
    
    <div class="section">
        <h2>Token Counts by Model</h2>
        <table>
            <tr>
                <th>Model</th>
                <th>DX</th>
                <th>JSON</th>
                <th>YAML</th>
                <th>TOML</th>
                <th>TOON</th>
            </tr>
            <tr>
                <td>GPT-4o (OpenAI)</td>
                <td><strong>${report.dxTokens.openai.count}</strong></td>
                <td>${report.jsonTokens.openai.count}</td>
                <td>${report.yamlTokens.openai.count}</td>
                <td>${report.tomlTokens.openai.count}</td>
                <td>${report.toonTokens.openai.count}</td>
            </tr>
            <tr>
                <td>Claude Sonnet 4</td>
                <td><strong>${report.dxTokens.claude.count}</strong></td>
                <td>${report.jsonTokens.claude.count}</td>
                <td>${report.yamlTokens.claude.count}</td>
                <td>${report.tomlTokens.claude.count}</td>
                <td>${report.toonTokens.claude.count}</td>
            </tr>
            <tr>
                <td>Gemini 3</td>
                <td><strong>${report.dxTokens.gemini.count}</strong></td>
                <td>${report.jsonTokens.gemini.count}</td>
                <td>${report.yamlTokens.gemini.count}</td>
                <td>${report.tomlTokens.gemini.count}</td>
                <td>${report.toonTokens.gemini.count}</td>
            </tr>
            <tr>
                <td>Other</td>
                <td><strong>${report.dxTokens.other.count}</strong></td>
                <td>${report.jsonTokens.other.count}</td>
                <td>${report.yamlTokens.other.count}</td>
                <td>${report.tomlTokens.other.count}</td>
                <td>${report.toonTokens.other.count}</td>
            </tr>
        </table>
    </div>

    <div class="section">
        <h2>Savings vs Other Formats</h2>
        <table>
            <tr>
                <th>Format</th>
                <th>Savings</th>
            </tr>
            <tr>
                <td>vs JSON</td>
                <td class="${report.savings.vsJson >= 0 ? 'savings-positive' : 'savings-negative'}">
                    ${report.savings.vsJson >= 0 ? '+' : ''}${report.savings.vsJson}%
                </td>
            </tr>
            <tr>
                <td>vs YAML</td>
                <td class="${report.savings.vsYaml >= 0 ? 'savings-positive' : 'savings-negative'}">
                    ${report.savings.vsYaml >= 0 ? '+' : ''}${report.savings.vsYaml}%
                </td>
            </tr>
            <tr>
                <td>vs TOML</td>
                <td class="${report.savings.vsToml >= 0 ? 'savings-positive' : 'savings-negative'}">
                    ${report.savings.vsToml >= 0 ? '+' : ''}${report.savings.vsToml}%
                </td>
            </tr>
            <tr>
                <td>vs TOON</td>
                <td class="${report.savings.vsToon >= 0 ? 'savings-positive' : 'savings-negative'}">
                    ${report.savings.vsToon >= 0 ? '+' : ''}${report.savings.vsToon}%
                </td>
            </tr>
        </table>
    </div>

    <div class="section">
        <h2>Copy Format</h2>
        <p>Click to copy the equivalent content in each format:</p>
        <button class="copy-btn" onclick="copyFormat('dx')">📋 Copy DX</button>
        <button class="copy-btn" onclick="copyFormat('json')">📋 Copy JSON</button>
        <button class="copy-btn" onclick="copyFormat('yaml')">📋 Copy YAML</button>
        <button class="copy-btn" onclick="copyFormat('toml')">📋 Copy TOML</button>
        <button class="copy-btn" onclick="copyFormat('toon')">📋 Copy TOON</button>
    </div>

    <script>
        const vscode = acquireVsCodeApi();
        
        function copyFormat(format) {
            vscode.postMessage({ command: 'copy', format: format });
        }
    </script>
</body>
</html>`;
    }

    /**
     * Copy format to clipboard
     */
    async copyFormat(format: 'dx' | 'json' | 'yaml' | 'toml' | 'toon'): Promise<void> {
        if (!this.currentReport) {
            vscode.window.showWarningMessage('No content available to copy');
            return;
        }

        const content = this.currentReport.equivalents[format];
        await vscode.env.clipboard.writeText(content);
        vscode.window.showInformationMessage(`${format.toUpperCase()} copied to clipboard`);
    }

    /**
     * Show the status bar item
     */
    show(): void {
        this.statusBarItem.show();
    }

    /**
     * Hide the status bar item
     */
    hide(): void {
        this.statusBarItem.hide();
        this.currentReport = null;
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        if (this.updateTimeout) {
            clearTimeout(this.updateTimeout);
        }
        if (this.panel) {
            this.panel.dispose();
        }
        this.statusBarItem.dispose();
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}

/**
 * Register enhanced token counter commands
 */
export function registerEnhancedTokenCounterCommands(
    context: vscode.ExtensionContext,
    tokenCounter: TokenCounterEnhancedStatusBar
): void {
    // Show token efficiency panel command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.showTokenEfficiencyPanel', async () => {
            await tokenCounter.showDetailPanel();
        })
    );

    // Copy format commands
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.copyAsDx', async () => {
            await tokenCounter.copyFormat('dx');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.copyAsJson', async () => {
            await tokenCounter.copyFormat('json');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.copyAsYaml', async () => {
            await tokenCounter.copyFormat('yaml');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.copyAsToml', async () => {
            await tokenCounter.copyFormat('toml');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.copyAsToon', async () => {
            await tokenCounter.copyFormat('toon');
        })
    );
}
