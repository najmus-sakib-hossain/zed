/**
 * Universal Token Counter Status Bar
 * 
 * Displays token count for ANY open file in the VS Code status bar.
 * Click to see detailed token counts and costs from all major LLM models.
 * 
 * Supports all major models with accurate pricing (January 2026):
 * - OpenAI/Azure: GPT-5.2, GPT-5.2 pro, GPT-5 mini
 * - Anthropic: Claude Opus 4.5, Opus 4, Sonnet 4.5, Sonnet 4, Haiku 4.5, Haiku 3.5
 * - Google: Gemini 3 Pro/Flash, Gemini 2.5 Pro/Flash/Flash-Lite, Gemini 2.0 Flash/Flash-Lite
 */

import * as vscode from 'vscode';
import {
    LLM_MODELS,
    LlmModel,
    estimateTokens,
    countTokensAsync,
    calculateCost,
    formatCost,
    formatTokenCount,
    getModelsByProvider,
} from './llmModels';
import { isTiktokenAvailable } from './tiktoken';

/**
 * Token count result for a single model
 */
export interface ModelTokenResult {
    model: LlmModel;
    tokens: number;
    inputCost: number;
    inputCachedCost: number;
    outputCost: number;
    isEstimate: boolean;
}

export class UniversalTokenCounterStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];
    private currentResults: ModelTokenResult[] = [];
    private currentFileName: string = '';
    private currentFilePath: string = '';
    private currentCharCount: number = 0;
    private currentLineCount: number = 0;
    private updateTimeout: NodeJS.Timeout | null = null;
    private panel: vscode.WebviewPanel | null = null;
    private tiktokenAvailable: boolean = false;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            1000
        );
        this.statusBarItem.command = 'dx.showUniversalTokenPanel';
        this.statusBarItem.text = '$(symbol-number) tokens';
        this.statusBarItem.tooltip = 'Click to see token counts';
        this.statusBarItem.show();

        // Check tiktoken availability
        this.checkTiktoken();

        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                this.updateForEditor(editor);
            })
        );

        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                const activeEditor = vscode.window.activeTextEditor;
                if (activeEditor && event.document === activeEditor.document) {
                    this.debouncedUpdate(activeEditor);
                }
            })
        );

        this.updateForEditor(vscode.window.activeTextEditor);
    }

    private async checkTiktoken(): Promise<void> {
        this.tiktokenAvailable = await isTiktokenAvailable();
        if (this.tiktokenAvailable) {
            console.log('DX Token Counter: Using real tiktoken for accurate counts');
        } else {
            console.log('DX Token Counter: Using character-based estimation (install js-tiktoken for accuracy)');
        }
    }

    private debouncedUpdate(editor: vscode.TextEditor): void {
        if (this.updateTimeout) clearTimeout(this.updateTimeout);
        this.updateTimeout = setTimeout(() => this.updateForEditor(editor), 300);
    }

    private async updateForEditor(editor: vscode.TextEditor | undefined): Promise<void> {
        if (!editor) {
            this.hide();
            return;
        }

        const content = editor.document.getText();
        this.currentFileName = editor.document.fileName.split(/[/\\]/).pop() || 'file';
        this.currentFilePath = editor.document.fileName;
        this.currentCharCount = content.length;
        this.currentLineCount = editor.document.lineCount;

        // Calculate tokens and costs for all models (async for real tiktoken)
        const results: ModelTokenResult[] = [];
        
        for (const model of LLM_MODELS) {
            let tokens: number;
            let isEstimate = true;
            
            try {
                tokens = await countTokensAsync(content, model);
                isEstimate = !this.tiktokenAvailable;
            } catch {
                tokens = estimateTokens(content, model);
            }
            
            results.push({
                model,
                tokens,
                inputCost: calculateCost(tokens, model, 'input'),
                inputCachedCost: calculateCost(tokens, model, 'inputCached'),
                outputCost: calculateCost(tokens, model, 'output'),
                isEstimate,
            });
        }
        
        this.currentResults = results;
        this.updateStatusBar();
        if (this.panel) this.updatePanelContent();
    }

    private updateStatusBar(): void {
        if (this.currentResults.length === 0) {
            this.hide();
            return;
        }

        // Use GPT-5 mini as the default display (most common/affordable)
        const gpt5Mini = this.currentResults.find(r => r.model.name === 'GPT-5 mini');
        const count = gpt5Mini?.tokens || this.currentResults[0].tokens;
        const isEstimate = gpt5Mini?.isEstimate ?? true;

        // Show indicator if using real tiktoken
        const indicator = isEstimate ? '~' : '';
        this.statusBarItem.text = `$(symbol-number) ${indicator}${formatTokenCount(count)} tokens`;
        this.statusBarItem.tooltip = this.createTooltip();
        this.statusBarItem.show();
    }

    private createTooltip(): vscode.MarkdownString {
        const tooltip = new vscode.MarkdownString();
        tooltip.isTrusted = true;
        tooltip.supportHtml = true;

        const accuracyNote = this.tiktokenAvailable 
            ? '✓ Using real tiktoken' 
            : '~ Estimated (install js-tiktoken for accuracy)';
        
        tooltip.appendMarkdown(`**Token Counts** _(click for details)_\n\n`);
        tooltip.appendMarkdown(`_${accuracyNote}_\n\n`);
        tooltip.appendMarkdown(`| Model | Tokens | Input Cost |\n`);
        tooltip.appendMarkdown(`|:------|-------:|----------:|\n`);

        // Show a subset of popular models in tooltip
        const popularModels = ['GPT-5 mini', 'Claude Sonnet 4', 'Gemini 2.5 Flash'];
        for (const result of this.currentResults) {
            if (popularModels.includes(result.model.name)) {
                const prefix = result.isEstimate ? '~' : '';
                tooltip.appendMarkdown(
                    `| ${result.model.name} | ${prefix}${result.tokens.toLocaleString()} | ${formatCost(result.inputCost)} |\n`
                );
            }
        }

        return tooltip;
    }

    showDetailPanel(): void {
        if (this.currentResults.length === 0) {
            vscode.window.showWarningMessage('No file open to analyze');
            return;
        }

        if (this.panel) {
            this.panel.reveal();
            this.updatePanelContent();
            return;
        }

        this.panel = vscode.window.createWebviewPanel(
            'dxUniversalTokens',
            'Token Analysis',
            vscode.ViewColumn.Beside,
            { enableScripts: true, retainContextWhenHidden: true }
        );

        this.panel.onDidDispose(() => { this.panel = null; });
        this.updatePanelContent();
    }

    private updatePanelContent(): void {
        if (!this.panel || this.currentResults.length === 0) return;
        this.panel.webview.html = this.getPanelHtml();
    }

    private getPanelHtml(): string {
        const modelsByProvider = getModelsByProvider();
        
        // DX Savings calculation (73.3% for serializer, 42.9% for markdown)
        const isDxFile = this.currentFileName.endsWith('.sr') || 
                         this.currentFileName.endsWith('dx');
        const isMarkdownMachine = this.currentFileName.endsWith('.machine');
        
        // Estimate what the equivalent JSON/Markdown would cost
        const dxSavingsPercent = isDxFile ? 73.3 : isMarkdownMachine ? 42.9 : 0;
        const originalMultiplier = dxSavingsPercent > 0 ? 1 / (1 - dxSavingsPercent / 100) : 1;
        
        // Generate table rows grouped by provider
        let tableRows = '';
        for (const [provider, models] of modelsByProvider) {
            // Provider header row
            tableRows += `<tr class="provider-header"><td colspan="8">${provider}</td></tr>`;
            
            for (const model of models) {
                const result = this.currentResults.find(r => r.model.name === model.name);
                if (!result) continue;

                const originalCost = result.inputCost * originalMultiplier;
                const savings = originalCost - result.inputCost;
                const savingsDisplay = dxSavingsPercent > 0 
                    ? `<span class="savings">-${formatCost(savings)}</span>` 
                    : '<span class="no-savings">-</span>';

                tableRows += `
                    <tr>
                        <td class="model-name">${model.name}</td>
                        <td class="context">${model.contextWindow}</td>
                        <td class="price">$${model.inputPer1M}</td>
                        <td class="price">$${model.inputCachedPer1M}</td>
                        <td class="price">$${model.outputPer1M}</td>
                        <td class="count">${result.tokens.toLocaleString()}</td>
                        <td class="cost">${formatCost(result.inputCost)}</td>
                        <td class="dx-savings">${savingsDisplay}</td>
                    </tr>
                `;
            }
        }

        // Calculate averages
        const avgTokens = Math.round(
            this.currentResults.reduce((sum, r) => sum + r.tokens, 0) / this.currentResults.length
        );
        const avgInputCost = this.currentResults.reduce((sum, r) => sum + r.inputCost, 0) / this.currentResults.length;
        const avgOriginalCost = avgInputCost * originalMultiplier;
        const totalSavings = avgOriginalCost - avgInputCost;

        // DX Savings banner (only show for DX files)
        const dxSavingsBanner = dxSavingsPercent > 0 ? `
            <div class="dx-banner">
                <div class="dx-banner-icon">⚡</div>
                <div class="dx-banner-content">
                    <div class="dx-banner-title">DX ${isDxFile ? 'Serializer' : 'Markdown'} Savings</div>
                    <div class="dx-banner-stats">
                        <span class="dx-percent">${dxSavingsPercent}%</span> fewer tokens
                        <span class="dx-divider">•</span>
                        <span class="dx-money">${formatCost(totalSavings)}</span> saved per request
                    </div>
                </div>
            </div>
        ` : '';

        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Token Analysis</title>
    <style>
        :root {
            --bg-primary: #000000;
            --bg-secondary: #0a0a0a;
            --bg-tertiary: #111111;
            --bg-hover: #1a1a1a;
            --border-color: #333333;
            --text-primary: #ededed;
            --text-secondary: #888888;
            --text-muted: #666666;
            --accent-blue: #0070f3;
            --accent-green: #50e3c2;
            --accent-green-dark: #0d9373;
            --gradient-green: linear-gradient(135deg, #0d9373 0%, #50e3c2 100%);
        }
        
        * { margin: 0; padding: 0; box-sizing: border-box; }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            padding: 24px;
            line-height: 1.5;
        }
        
        .header { margin-bottom: 24px; }
        .header h1 { font-size: 24px; font-weight: 600; margin-bottom: 8px; }
        .header .filepath {
            font-size: 13px;
            color: var(--text-secondary);
            font-family: 'SF Mono', Monaco, monospace;
            background: var(--bg-tertiary);
            padding: 8px 12px;
            border-radius: 6px;
            border: 1px solid var(--border-color);
            word-break: break-all;
        }
        
        .dx-banner {
            background: var(--gradient-green);
            border-radius: 12px;
            padding: 20px 24px;
            margin-bottom: 24px;
            display: flex;
            align-items: center;
            gap: 16px;
            box-shadow: 0 4px 20px rgba(80, 227, 194, 0.2);
        }
        
        .dx-banner-icon {
            font-size: 32px;
            filter: drop-shadow(0 2px 4px rgba(0,0,0,0.3));
        }
        
        .dx-banner-content { flex: 1; }
        
        .dx-banner-title {
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: rgba(0,0,0,0.7);
            margin-bottom: 4px;
        }
        
        .dx-banner-stats {
            font-size: 18px;
            font-weight: 500;
            color: #000;
        }
        
        .dx-percent {
            font-size: 28px;
            font-weight: 700;
        }
        
        .dx-money {
            font-family: 'SF Mono', Monaco, monospace;
            font-weight: 700;
            font-size: 20px;
        }
        
        .dx-divider {
            margin: 0 12px;
            opacity: 0.5;
        }
        
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 16px;
            margin-bottom: 24px;
        }
        
        .stat-card {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 16px;
            text-align: center;
        }
        
        .stat-value {
            font-size: 24px;
            font-weight: 700;
            color: var(--text-primary);
        }
        
        .stat-value.accent { color: var(--accent-blue); }
        .stat-value.green { color: var(--accent-green); }
        
        .stat-label {
            font-size: 11px;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-top: 4px;
        }
        
        .section-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 16px;
            padding-bottom: 12px;
            border-bottom: 1px solid var(--border-color);
        }
        
        .section-title {
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-secondary);
        }
        
        .badge {
            font-size: 11px;
            padding: 4px 8px;
            border-radius: 4px;
            font-weight: 500;
            background: rgba(0, 112, 243, 0.1);
            color: var(--accent-blue);
        }
        
        table { width: 100%; border-collapse: collapse; }
        
        th {
            text-align: left;
            font-size: 11px;
            font-weight: 500;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            padding: 12px 8px;
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border-color);
        }
        
        th:nth-child(n+3) { text-align: right; }
        
        td {
            padding: 12px 8px;
            border-bottom: 1px solid var(--border-color);
            font-size: 13px;
        }
        
        tr:hover { background: var(--bg-hover); }
        
        .provider-header {
            background: var(--bg-tertiary) !important;
        }
        
        .provider-header td {
            font-weight: 600;
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--accent-blue);
            padding: 8px;
        }
        
        .model-name { font-weight: 500; }
        .context { color: var(--text-secondary); font-size: 12px; }
        .price { text-align: right; color: var(--text-muted); font-family: 'SF Mono', Monaco, monospace; font-size: 12px; }
        .count { text-align: right; font-weight: 600; font-family: 'SF Mono', Monaco, monospace; }
        .cost { text-align: right; color: var(--text-primary); font-family: 'SF Mono', Monaco, monospace; }
        
        .dx-savings { text-align: right; }
        .savings {
            color: var(--accent-green);
            font-family: 'SF Mono', Monaco, monospace;
            font-weight: 600;
            background: rgba(80, 227, 194, 0.1);
            padding: 2px 6px;
            border-radius: 4px;
        }
        .no-savings { color: var(--text-muted); }
        
        .footer {
            margin-top: 24px;
            padding-top: 16px;
            border-top: 1px solid var(--border-color);
            font-size: 12px;
            color: var(--text-muted);
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>Token Analysis</h1>
        <div class="filepath">${this.currentFilePath}</div>
    </div>
    
    ${dxSavingsBanner}
    
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value accent">${formatTokenCount(avgTokens)}</div>
            <div class="stat-label">Avg Tokens</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">${formatTokenCount(this.currentCharCount)}</div>
            <div class="stat-label">Characters</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">${this.currentLineCount.toLocaleString()}</div>
            <div class="stat-label">Lines</div>
        </div>
        <div class="stat-card">
            <div class="stat-value green">${dxSavingsPercent > 0 ? '-' + formatCost(totalSavings) : formatCost(avgInputCost)}</div>
            <div class="stat-label">${dxSavingsPercent > 0 ? 'DX Savings' : 'Avg Cost'}</div>
        </div>
    </div>

    <div class="section-header">
        <span class="section-title">Token Counts & Pricing by Model</span>
        <span class="badge">${LLM_MODELS.length} Models</span>
    </div>
    
    <table>
        <thead>
            <tr>
                <th>Model</th>
                <th>Context</th>
                <th>Input/1M</th>
                <th>Cached/1M</th>
                <th>Output/1M</th>
                <th>Tokens</th>
                <th>Cost</th>
                <th>DX Saves</th>
            </tr>
        </thead>
        <tbody>${tableRows}</tbody>
    </table>

    <div class="footer">
        Token counts are estimates. Pricing as of January 2026. DX Serializer saves 73.3% vs JSON, DX Markdown saves 42.9% vs standard Markdown.
    </div>
</body>
</html>`;
    }

    hide(): void {
        this.statusBarItem.text = '$(symbol-number) --';
        this.statusBarItem.tooltip = 'No file open';
        this.currentResults = [];
    }

    dispose(): void {
        if (this.updateTimeout) clearTimeout(this.updateTimeout);
        if (this.panel) this.panel.dispose();
        this.statusBarItem.dispose();
        for (const d of this.disposables) d.dispose();
        this.disposables = [];
    }
}

export function registerUniversalTokenCounterCommands(
    context: vscode.ExtensionContext,
    tokenCounter: UniversalTokenCounterStatusBar
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.showUniversalTokenPanel', () => {
            tokenCounter.showDetailPanel();
        })
    );
}
