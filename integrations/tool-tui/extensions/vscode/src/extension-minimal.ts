/**
 * Minimal DX Extension - Token Counter Only
 */

import * as vscode from 'vscode';

class TokenCounter implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];

    constructor() {
        console.log('TokenCounter: Creating status bar item...');

        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            1000
        );
        this.statusBarItem.text = '$(symbol-number) 0 tokens';
        this.statusBarItem.tooltip = 'Click to see token counts';
        this.statusBarItem.command = 'dx.showTokens';
        this.statusBarItem.show();

        console.log('TokenCounter: Status bar item created and shown');

        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                this.update(editor);
            })
        );

        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                const editor = vscode.window.activeTextEditor;
                if (editor && event.document === editor.document) {
                    this.update(editor);
                }
            })
        );

        this.update(vscode.window.activeTextEditor);
    }

    private update(editor: vscode.TextEditor | undefined): void {
        if (!editor) {
            this.statusBarItem.text = '$(symbol-number) --';
            return;
        }

        const text = editor.document.getText();
        const tokens = Math.ceil(text.length / 4);
        const formatted = tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : tokens.toString();

        this.statusBarItem.text = `$(symbol-number) ${formatted} tokens`;
    }

    dispose(): void {
        this.statusBarItem.dispose();
        for (const d of this.disposables) d.dispose();
    }
}

let tokenCounter: TokenCounter | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    console.log('DX Extension: Activating...');

    try {
        tokenCounter = new TokenCounter();
        context.subscriptions.push(tokenCounter);

        context.subscriptions.push(
            vscode.commands.registerCommand('dx.showTokens', () => {
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                    const text = editor.document.getText();
                    const tokens = Math.ceil(text.length / 4);
                    vscode.window.showInformationMessage(`Token count: ${tokens}`);
                }
            })
        );

        console.log('DX Extension: Activated successfully!');
        vscode.window.showInformationMessage('DX Token Counter activated!');
    } catch (error) {
        console.error('DX Extension: Activation failed:', error);
        vscode.window.showErrorMessage(`DX Extension failed: ${error}`);
    }
}

export async function deactivate(): Promise<void> {
    console.log('DX Extension: Deactivating...');
}
