/**
 * DX Generator Status Bar
 * 
 * Shows token savings in the VS Code status bar.
 * Requirements: 10.4
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Token savings statistics
 */
interface TokenStats {
    sessionTokens: number;
    totalTokens: number;
    generationCount: number;
    lastUpdated: string;
}

/**
 * Status bar for displaying token savings
 */
export class GeneratorStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private stats: TokenStats;
    private statsFilePath: string | undefined;
    private disposables: vscode.Disposable[] = [];

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            99
        );
        this.statusBarItem.command = 'dx.generator.showStats';

        this.stats = {
            sessionTokens: 0,
            totalTokens: 0,
            generationCount: 0,
            lastUpdated: new Date().toISOString(),
        };

        // Initialize stats file path
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            this.statsFilePath = path.join(
                workspaceFolders[0].uri.fsPath,
                '.dx',
                'generator-stats.json'
            );
            this.loadStats();
        }

        this.updateDisplay();
    }

    /**
     * Record token savings from a generation
     */
    recordSavings(tokensSaved: number): void {
        this.stats.sessionTokens += tokensSaved;
        this.stats.totalTokens += tokensSaved;
        this.stats.generationCount++;
        this.stats.lastUpdated = new Date().toISOString();

        this.updateDisplay();
        this.saveStats();

        // Show temporary message
        vscode.window.setStatusBarMessage(
            `$(zap) +${tokensSaved} tokens saved`,
            3000
        );
    }

    /**
     * Get current statistics
     */
    getStats(): TokenStats {
        return { ...this.stats };
    }

    /**
     * Reset session statistics
     */
    resetSession(): void {
        this.stats.sessionTokens = 0;
        this.updateDisplay();
    }


    /**
     * Reset all statistics
     */
    resetAll(): void {
        this.stats = {
            sessionTokens: 0,
            totalTokens: 0,
            generationCount: 0,
            lastUpdated: new Date().toISOString(),
        };
        this.updateDisplay();
        this.saveStats();
    }

    /**
     * Update the status bar display
     */
    private updateDisplay(): void {
        if (this.stats.sessionTokens > 0) {
            this.statusBarItem.text = `$(zap) ${this.formatNumber(this.stats.sessionTokens)} tokens`;
            this.statusBarItem.tooltip = this.buildTooltip();
            this.statusBarItem.show();
        } else if (this.stats.totalTokens > 0) {
            this.statusBarItem.text = `$(zap) DX Gen`;
            this.statusBarItem.tooltip = this.buildTooltip();
            this.statusBarItem.show();
        } else {
            this.statusBarItem.hide();
        }
    }

    /**
     * Build tooltip content
     */
    private buildTooltip(): string {
        const lines = [
            'DX Generator Token Savings',
            '',
            `Session: ${this.formatNumber(this.stats.sessionTokens)} tokens`,
            `Total: ${this.formatNumber(this.stats.totalTokens)} tokens`,
            `Generations: ${this.stats.generationCount}`,
            '',
            'Click for details',
        ];
        return lines.join('\n');
    }

    /**
     * Format number with K/M suffix
     */
    private formatNumber(num: number): string {
        if (num >= 1000000) {
            return (num / 1000000).toFixed(1) + 'M';
        }
        if (num >= 1000) {
            return (num / 1000).toFixed(1) + 'K';
        }
        return num.toString();
    }

    /**
     * Load statistics from file
     */
    private loadStats(): void {
        if (!this.statsFilePath) {
            return;
        }

        try {
            if (fs.existsSync(this.statsFilePath)) {
                const content = fs.readFileSync(this.statsFilePath, 'utf-8');
                const saved = JSON.parse(content) as TokenStats;
                this.stats.totalTokens = saved.totalTokens || 0;
                this.stats.generationCount = saved.generationCount || 0;
                this.stats.lastUpdated = saved.lastUpdated || new Date().toISOString();
            }
        } catch (error) {
            console.error('Failed to load generator stats:', error);
        }
    }

    /**
     * Save statistics to file
     */
    private saveStats(): void {
        if (!this.statsFilePath) {
            return;
        }

        try {
            const dir = path.dirname(this.statsFilePath);
            if (!fs.existsSync(dir)) {
                fs.mkdirSync(dir, { recursive: true });
            }
            fs.writeFileSync(
                this.statsFilePath,
                JSON.stringify(this.stats, null, 2),
                'utf-8'
            );
        } catch (error) {
            console.error('Failed to save generator stats:', error);
        }
    }

    /**
     * Show detailed statistics
     */
    async showDetails(): Promise<void> {
        const items = [
            {
                label: '$(graph) Session Tokens Saved',
                description: this.formatNumber(this.stats.sessionTokens),
            },
            {
                label: '$(database) Total Tokens Saved',
                description: this.formatNumber(this.stats.totalTokens),
            },
            {
                label: '$(file-code) Total Generations',
                description: this.stats.generationCount.toString(),
            },
            {
                label: '$(clock) Last Updated',
                description: new Date(this.stats.lastUpdated).toLocaleString(),
            },
            { label: '', kind: vscode.QuickPickItemKind.Separator },
            {
                label: '$(refresh) Reset Session Stats',
                description: 'Clear session token count',
            },
            {
                label: '$(trash) Reset All Stats',
                description: 'Clear all statistics',
            },
        ];

        const selected = await vscode.window.showQuickPick(items, {
            placeHolder: 'DX Generator Statistics',
        });

        if (selected?.label === '$(refresh) Reset Session Stats') {
            this.resetSession();
            vscode.window.showInformationMessage('Session stats reset');
        } else if (selected?.label === '$(trash) Reset All Stats') {
            const confirm = await vscode.window.showWarningMessage(
                'Reset all generator statistics?',
                'Yes',
                'No'
            );
            if (confirm === 'Yes') {
                this.resetAll();
                vscode.window.showInformationMessage('All stats reset');
            }
        }
    }

    dispose(): void {
        this.statusBarItem.dispose();
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }
}

/**
 * Register status bar commands
 */
export function registerStatusBarCommands(
    context: vscode.ExtensionContext,
    statusBar: GeneratorStatusBar
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.showStats', async () => {
            await statusBar.showDetails();
        })
    );
}
