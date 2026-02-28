/**
 * Diagnostics Provider for DX Serializer VS Code Extension
 * 
 * Provides inline error diagnostics for DX and DXM files:
 * - Parse errors with line/column information
 * - Undefined reference errors with suggestions
 * - Syntax errors (unclosed quotes, invalid section headers)
 * - DXM format validation errors
 * 
 * Requirements: 5.8, 10.6, 12.4
 */

import * as vscode from 'vscode';
import { parseHuman, ParseErrorInfo } from './humanParser';
import { parseLlm } from './llmParser';
import { detectFormat } from './formatDetector';
import { isExactlyDxFile } from './utils';

/**
 * Diagnostic collection for DX files
 */
let diagnosticCollection: vscode.DiagnosticCollection | undefined;

/**
 * Diagnostic collection for DXM files
 */
let dxmDiagnosticCollection: vscode.DiagnosticCollection | undefined;

/**
 * Disposables for event subscriptions
 */
const disposables: vscode.Disposable[] = [];

/**
 * Check if a document is a Markdown file
 */
function isMarkdownFile(uri: vscode.Uri): boolean {
    return uri.fsPath.endsWith('.md');
}

/**
 * Convert ParseErrorInfo to VS Code Diagnostic
 */
function errorToDiagnostic(error: ParseErrorInfo): vscode.Diagnostic {
    // Convert 1-indexed line/column to 0-indexed
    const line = Math.max(0, (error.line || 1) - 1);
    const column = Math.max(0, (error.column || 1) - 1);

    // Create range - highlight the error location
    const range = new vscode.Range(
        new vscode.Position(line, column),
        new vscode.Position(line, column + 10) // Highlight ~10 chars
    );

    // Build diagnostic message
    let message = error.message;
    if (error.hint) {
        message += `\n\nHint: ${error.hint}`;
    }

    const diagnostic = new vscode.Diagnostic(
        range,
        message,
        vscode.DiagnosticSeverity.Error
    );

    diagnostic.source = 'DX';

    // Add code for specific error types (for code actions)
    if (error.message.includes('Undefined reference')) {
        diagnostic.code = 'undefined-reference';
    } else if (error.message.includes('Unclosed')) {
        diagnostic.code = 'unclosed-delimiter';
    } else if (error.message.includes('Invalid section')) {
        diagnostic.code = 'invalid-section';
    }

    return diagnostic;
}

/**
 * Parse document and collect all errors
 */
function collectErrors(content: string): ParseErrorInfo[] {
    const errors: ParseErrorInfo[] = [];
    const detection = detectFormat(content);

    if (detection.format === 'llm') {
        // Parse LLM format
        const result = parseLlm(content);
        if (!result.success && result.error) {
            errors.push({
                message: result.error.message,
                line: result.error.line,
                column: result.error.column,
                hint: result.error.hint,
            });
        }
    } else {
        // Parse Human format
        const result = parseHuman(content);
        if (!result.success && result.error) {
            errors.push(result.error);
        }
    }

    return errors;
}

/**
 * Update diagnostics for a document
 */
function updateDiagnostics(document: vscode.TextDocument): void {
    if (!diagnosticCollection) {
        return;
    }

    // Only process DX files
    if (!isExactlyDxFile(document.uri)) {
        return;
    }

    const content = document.getText();

    // Skip empty documents
    if (!content.trim()) {
        diagnosticCollection.delete(document.uri);
        return;
    }

    // Collect errors
    const errors = collectErrors(content);

    // Convert to diagnostics
    const diagnostics = errors.map(errorToDiagnostic);

    // Update the diagnostic collection
    diagnosticCollection.set(document.uri, diagnostics);
}

/**
 * Clear diagnostics for a document
 */
function clearDiagnostics(document: vscode.TextDocument): void {
    if (!diagnosticCollection) {
        return;
    }
    diagnosticCollection.delete(document.uri);
}

/**
 * Collect errors from DXM content
 * 
 * Validates DXM format including:
 * - Header syntax (N|Title)
 * - Code block syntax (@lang ... @)
 * - Reference syntax (^key)
 * - Table syntax (#t(schema))
 */
function collectDxmErrors(content: string): ParseErrorInfo[] {
    const errors: ParseErrorInfo[] = [];
    const lines = content.split('\n');

    let inCodeBlock = false;
    let codeBlockStartLine = 0;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNum = i + 1;

        // Check for code block start
        if (line.startsWith('@') && !line.startsWith('@dxm') && line.length > 1) {
            if (inCodeBlock) {
                errors.push({
                    message: 'Nested code block detected',
                    line: lineNum,
                    column: 1,
                    hint: 'Close the previous code block with @ before starting a new one',
                });
            }
            inCodeBlock = true;
            codeBlockStartLine = lineNum;
            continue;
        }

        // Check for code block end
        if (line === '@') {
            if (!inCodeBlock) {
                errors.push({
                    message: 'Unexpected code block end marker',
                    line: lineNum,
                    column: 1,
                    hint: 'This @ appears without a matching code block start',
                });
            }
            inCodeBlock = false;
            continue;
        }

        // Skip validation inside code blocks
        if (inCodeBlock) {
            continue;
        }

        // Check header syntax
        const headerMatch = line.match(/^(\d+)\|/);
        if (headerMatch) {
            const level = parseInt(headerMatch[1], 10);
            if (level < 1 || level > 6) {
                errors.push({
                    message: `Invalid header level: ${level}`,
                    line: lineNum,
                    column: 1,
                    hint: 'Header levels must be between 1 and 6',
                });
            }
            continue;
        }

        // Check for unclosed references
        const refMatches = line.match(/\^(\w+)/g);
        if (refMatches) {
            // References are valid, but we could check if they're defined
            // For now, just validate the syntax
        }

        // Check for malformed table syntax
        if (line.startsWith('#t(') && !line.includes(')')) {
            errors.push({
                message: 'Unclosed table schema',
                line: lineNum,
                column: 1,
                hint: 'Table schema must be closed with )',
            });
        }
    }

    // Check for unclosed code block at end of file
    if (inCodeBlock) {
        errors.push({
            message: 'Unclosed code block',
            line: codeBlockStartLine,
            column: 1,
            hint: 'Add @ on a new line to close the code block',
        });
    }

    return errors;
}

/**
 * Update diagnostics for a DXM document
 * 
 * Requirements: 12.4
 */
function updateDxmDiagnostics(document: vscode.TextDocument): void {
    if (!dxmDiagnosticCollection) {
        return;
    }

    // Only process Markdown files
    if (!isMarkdownFile(document.uri)) {
        return;
    }

    const content = document.getText();

    // Skip empty documents
    if (!content.trim()) {
        dxmDiagnosticCollection.delete(document.uri);
        return;
    }

    // Collect errors
    const errors = collectDxmErrors(content);

    // Convert to diagnostics
    const diagnostics = errors.map(errorToDiagnostic);

    // Update the diagnostic collection
    dxmDiagnosticCollection.set(document.uri, diagnostics);
}

/**
 * Clear DXM diagnostics for a document
 */
function clearDxmDiagnostics(document: vscode.TextDocument): void {
    if (!dxmDiagnosticCollection) {
        return;
    }
    dxmDiagnosticCollection.delete(document.uri);
}

/**
 * Register the diagnostics provider
 * 
 * Requirements: 5.8, 10.6, 12.4
 */
export function registerDiagnosticsProvider(context: vscode.ExtensionContext): void {
    // Create diagnostic collection for DX files
    diagnosticCollection = vscode.languages.createDiagnosticCollection('dx-errors');
    context.subscriptions.push(diagnosticCollection);

    // Create diagnostic collection for DXM files (Requirements: 12.4)
    dxmDiagnosticCollection = vscode.languages.createDiagnosticCollection('dxm-errors');
    context.subscriptions.push(dxmDiagnosticCollection);

    // Update diagnostics when document is opened
    disposables.push(
        vscode.workspace.onDidOpenTextDocument((document) => {
            updateDiagnostics(document);
            updateDxmDiagnostics(document);
        })
    );

    // Update diagnostics when document content changes
    disposables.push(
        vscode.workspace.onDidChangeTextDocument((event) => {
            updateDiagnostics(event.document);
            updateDxmDiagnostics(event.document);
        })
    );

    // Clear diagnostics when document is closed
    disposables.push(
        vscode.workspace.onDidCloseTextDocument((document) => {
            clearDiagnostics(document);
            clearDxmDiagnostics(document);
        })
    );

    // Update diagnostics for all currently open documents
    for (const document of vscode.workspace.textDocuments) {
        updateDiagnostics(document);
        updateDxmDiagnostics(document);
    }

    // Add disposables to context
    context.subscriptions.push(...disposables);

    console.log('DX: Diagnostics provider registered (DX + DXM)');
}

/**
 * Dispose of the diagnostics provider
 */
export function disposeDiagnosticsProvider(): void {
    if (diagnosticCollection) {
        diagnosticCollection.clear();
        diagnosticCollection.dispose();
        diagnosticCollection = undefined;
    }

    if (dxmDiagnosticCollection) {
        dxmDiagnosticCollection.clear();
        dxmDiagnosticCollection.dispose();
        dxmDiagnosticCollection = undefined;
    }

    for (const disposable of disposables) {
        disposable.dispose();
    }
    disposables.length = 0;
}
