/**
 * Inline decoration provider for secret highlighting
 * 
 * Provides inline decorations for detected secrets and vulnerabilities.
 * _Requirements: 10.5, 10.6_
 */

import * as vscode from 'vscode';
import {
    DecorationRange,
    DecorationType,
    getDecorationColor,
    getDecorationBackgroundColor,
} from './types';
import { SecurityClient } from './client';

/**
 * Decoration type cache
 */
interface DecorationTypeCache {
    critical: vscode.TextEditorDecorationType;
    high: vscode.TextEditorDecorationType;
    medium: vscode.TextEditorDecorationType;
    low: vscode.TextEditorDecorationType;
    info: vscode.TextEditorDecorationType;
}

/**
 * Security decoration provider
 * 
 * Manages inline decorations for security findings in the editor.
 */
export class SecurityDecorationProvider implements vscode.Disposable {
    private client: SecurityClient;
    private decorationTypes: DecorationTypeCache;
    private disposables: vscode.Disposable[] = [];

    constructor(client: SecurityClient) {
        this.client = client;
        this.decorationTypes = this.createDecorationTypes();

        // Subscribe to editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                if (editor) {
                    this.updateDecorations(editor);
                }
            })
        );

        // Subscribe to document changes
        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                const editor = vscode.window.activeTextEditor;
                if (editor && editor.document === event.document) {
                    this.updateDecorations(editor);
                }
            })
        );

        // Subscribe to finding updates
        this.client.subscribe(() => {
            const editor = vscode.window.activeTextEditor;
            if (editor) {
                this.updateDecorations(editor);
            }
        });

        // Initial update
        if (vscode.window.activeTextEditor) {
            this.updateDecorations(vscode.window.activeTextEditor);
        }
    }

    /**
     * Create decoration types for each severity level
     */
    private createDecorationTypes(): DecorationTypeCache {
        return {
            critical: vscode.window.createTextEditorDecorationType({
                backgroundColor: getDecorationBackgroundColor(DecorationType.Critical),
                border: `1px solid ${getDecorationColor(DecorationType.Critical)}`,
                borderRadius: '3px',
                overviewRulerColor: getDecorationColor(DecorationType.Critical),
                overviewRulerLane: vscode.OverviewRulerLane.Right,
                after: {
                    contentText: ' 🔐',
                    color: getDecorationColor(DecorationType.Critical),
                },
            }),
            high: vscode.window.createTextEditorDecorationType({
                backgroundColor: getDecorationBackgroundColor(DecorationType.High),
                border: `1px solid ${getDecorationColor(DecorationType.High)}`,
                borderRadius: '3px',
                overviewRulerColor: getDecorationColor(DecorationType.High),
                overviewRulerLane: vscode.OverviewRulerLane.Right,
                after: {
                    contentText: ' ⚠️',
                    color: getDecorationColor(DecorationType.High),
                },
            }),
            medium: vscode.window.createTextEditorDecorationType({
                backgroundColor: getDecorationBackgroundColor(DecorationType.Medium),
                border: `1px solid ${getDecorationColor(DecorationType.Medium)}`,
                borderRadius: '3px',
                overviewRulerColor: getDecorationColor(DecorationType.Medium),
                overviewRulerLane: vscode.OverviewRulerLane.Right,
            }),
            low: vscode.window.createTextEditorDecorationType({
                backgroundColor: getDecorationBackgroundColor(DecorationType.Low),
                border: `1px solid ${getDecorationColor(DecorationType.Low)}`,
                borderRadius: '3px',
                overviewRulerColor: getDecorationColor(DecorationType.Low),
                overviewRulerLane: vscode.OverviewRulerLane.Right,
            }),
            info: vscode.window.createTextEditorDecorationType({
                backgroundColor: getDecorationBackgroundColor(DecorationType.Info),
                borderRadius: '3px',
            }),
        };
    }

    /**
     * Update decorations for an editor
     */
    public updateDecorations(editor: vscode.TextEditor): void {
        const filePath = editor.document.uri.fsPath;
        const decorations = this.client.getDecorations(filePath);

        // Group decorations by type
        const critical: vscode.DecorationOptions[] = [];
        const high: vscode.DecorationOptions[] = [];
        const medium: vscode.DecorationOptions[] = [];
        const low: vscode.DecorationOptions[] = [];
        const info: vscode.DecorationOptions[] = [];

        for (const decoration of decorations) {
            const range = new vscode.Range(
                decoration.startLine,
                decoration.startColumn,
                decoration.endLine,
                decoration.endColumn
            );

            const decorationOption: vscode.DecorationOptions = {
                range,
                hoverMessage: new vscode.MarkdownString(decoration.message),
            };

            switch (decoration.decorationType) {
                case DecorationType.Critical:
                    critical.push(decorationOption);
                    break;
                case DecorationType.High:
                    high.push(decorationOption);
                    break;
                case DecorationType.Medium:
                    medium.push(decorationOption);
                    break;
                case DecorationType.Low:
                    low.push(decorationOption);
                    break;
                case DecorationType.Info:
                    info.push(decorationOption);
                    break;
            }
        }

        // Apply decorations
        editor.setDecorations(this.decorationTypes.critical, critical);
        editor.setDecorations(this.decorationTypes.high, high);
        editor.setDecorations(this.decorationTypes.medium, medium);
        editor.setDecorations(this.decorationTypes.low, low);
        editor.setDecorations(this.decorationTypes.info, info);
    }

    /**
     * Clear all decorations
     */
    public clearDecorations(editor: vscode.TextEditor): void {
        editor.setDecorations(this.decorationTypes.critical, []);
        editor.setDecorations(this.decorationTypes.high, []);
        editor.setDecorations(this.decorationTypes.medium, []);
        editor.setDecorations(this.decorationTypes.low, []);
        editor.setDecorations(this.decorationTypes.info, []);
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.decorationTypes.critical.dispose();
        this.decorationTypes.high.dispose();
        this.decorationTypes.medium.dispose();
        this.decorationTypes.low.dispose();
        this.decorationTypes.info.dispose();

        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }
}

/**
 * Code action provider for security quick-fixes
 * _Requirements: 10.6_
 */
export class SecurityCodeActionProvider implements vscode.CodeActionProvider {
    private client: SecurityClient;

    constructor(client: SecurityClient) {
        this.client = client;
    }

    /**
     * Provide code actions for a document
     */
    public provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range | vscode.Selection,
        _context: vscode.CodeActionContext,
        _token: vscode.CancellationToken
    ): vscode.CodeAction[] {
        const filePath = document.uri.fsPath;
        const line = range.start.line;
        const column = range.start.character;

        const actions = this.client.getCodeActions(filePath, line, column);
        
        return actions.map((action) => {
            const codeAction = new vscode.CodeAction(
                action.title,
                this.mapCodeActionKind(action.kind)
            );

            codeAction.isPreferred = action.isPreferred;

            // Create workspace edit
            const edit = new vscode.WorkspaceEdit();
            for (const textEdit of action.edits) {
                const editRange = new vscode.Range(
                    textEdit.startLine,
                    textEdit.startColumn,
                    textEdit.endLine,
                    textEdit.endColumn
                );
                edit.replace(document.uri, editRange, textEdit.newText);
            }
            codeAction.edit = edit;

            return codeAction;
        });
    }

    /**
     * Map our code action kind to VS Code's
     */
    private mapCodeActionKind(kind: string): vscode.CodeActionKind {
        switch (kind) {
            case 'quickfix':
                return vscode.CodeActionKind.QuickFix;
            case 'refactor':
                return vscode.CodeActionKind.Refactor;
            case 'source':
                return vscode.CodeActionKind.Source;
            default:
                return vscode.CodeActionKind.QuickFix;
        }
    }
}

/**
 * Diagnostics provider for security findings
 * 
 * Reports security findings as VS Code diagnostics in the Problems panel.
 */
export class SecurityDiagnosticsProvider implements vscode.Disposable {
    private client: SecurityClient;
    private diagnosticCollection: vscode.DiagnosticCollection;
    private disposables: vscode.Disposable[] = [];

    constructor(client: SecurityClient) {
        this.client = client;
        this.diagnosticCollection = vscode.languages.createDiagnosticCollection('dx-security');

        // Subscribe to finding updates
        this.client.subscribe((finding) => {
            if (finding.filePath) {
                this.updateDiagnostics(finding.filePath);
            }
        });

        // Subscribe to document changes
        this.disposables.push(
            vscode.workspace.onDidOpenTextDocument((document) => {
                this.updateDiagnostics(document.uri.fsPath);
            })
        );

        this.disposables.push(
            vscode.workspace.onDidCloseTextDocument((document) => {
                this.diagnosticCollection.delete(document.uri);
            })
        );
    }

    /**
     * Update diagnostics for a file
     */
    public updateDiagnostics(filePath: string): void {
        const uri = vscode.Uri.file(filePath);
        const findings = this.client.getFileFindings(filePath);
        const decorations = this.client.getDecorations(filePath);

        const diagnostics: vscode.Diagnostic[] = [];

        // Add findings as diagnostics
        for (const finding of findings) {
            const range = new vscode.Range(
                finding.lineNumber - 1,
                finding.column,
                finding.lineNumber - 1,
                finding.column + 10
            );

            const diagnostic = new vscode.Diagnostic(
                range,
                finding.message,
                this.mapSeverity(finding.severity)
            );

            diagnostic.source = 'dx-security';
            if (finding.cveId) {
                diagnostic.code = finding.cveId;
            }

            diagnostics.push(diagnostic);
        }

        // Add decorations as diagnostics (for secrets)
        for (const decoration of decorations) {
            const range = new vscode.Range(
                decoration.startLine,
                decoration.startColumn,
                decoration.endLine,
                decoration.endColumn
            );

            const diagnostic = new vscode.Diagnostic(
                range,
                decoration.message,
                this.mapDecorationType(decoration.decorationType)
            );

            diagnostic.source = 'dx-security';
            diagnostics.push(diagnostic);
        }

        this.diagnosticCollection.set(uri, diagnostics);
    }

    /**
     * Clear all diagnostics
     */
    public clearDiagnostics(): void {
        this.diagnosticCollection.clear();
    }

    /**
     * Map severity to VS Code diagnostic severity
     */
    private mapSeverity(severity: number): vscode.DiagnosticSeverity {
        switch (severity) {
            case 4: // Critical
            case 3: // High
                return vscode.DiagnosticSeverity.Error;
            case 2: // Medium
                return vscode.DiagnosticSeverity.Warning;
            case 1: // Low
                return vscode.DiagnosticSeverity.Information;
            default:
                return vscode.DiagnosticSeverity.Hint;
        }
    }

    /**
     * Map decoration type to VS Code diagnostic severity
     */
    private mapDecorationType(type: DecorationType): vscode.DiagnosticSeverity {
        switch (type) {
            case DecorationType.Critical:
            case DecorationType.High:
                return vscode.DiagnosticSeverity.Error;
            case DecorationType.Medium:
                return vscode.DiagnosticSeverity.Warning;
            case DecorationType.Low:
                return vscode.DiagnosticSeverity.Information;
            default:
                return vscode.DiagnosticSeverity.Hint;
        }
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.diagnosticCollection.dispose();
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }
}
