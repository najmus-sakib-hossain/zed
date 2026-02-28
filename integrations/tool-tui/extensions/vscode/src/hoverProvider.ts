/**
 * Hover Provider for DX Serializer VS Code Extension
 * 
 * Provides hover information for:
 * - Reference usages ([^key] patterns) - shows resolved value
 * - Stack section keys - shows the full value
 * 
 * Requirements: 5.6, 7.2
 */

import * as vscode from 'vscode';
import { parseHuman } from './humanParser';
import { parseLlm } from './llmParser';
import { detectFormat } from './formatDetector';

/**
 * Reference information extracted from document
 */
interface ReferenceInfo {
    key: string;
    value: string;
    line: number;
}

/**
 * Extract all reference definitions from document content
 * Supports both Human V3 format ([stack] section) and LLM format (#: definitions)
 */
function extractReferences(content: string): Map<string, ReferenceInfo> {
    const refs = new Map<string, ReferenceInfo>();
    const detection = detectFormat(content);

    if (detection.format === 'llm') {
        // Parse LLM format
        const result = parseLlm(content);
        if (result.success && result.document) {
            for (const [key, value] of result.document.refs) {
                refs.set(key, { key, value, line: -1 });
            }
        }
    } else {
        // Parse Human format
        const result = parseHuman(content);
        if (result.document) {
            for (const [key, value] of result.document.refs) {
                refs.set(key, { key, value, line: -1 });
            }
        }
    }

    return refs;
}

/**
 * Find reference usage at position
 * Looks for [^key] pattern at the given position
 */
function findReferenceAtPosition(
    document: vscode.TextDocument,
    position: vscode.Position
): { key: string; range: vscode.Range } | null {
    const line = document.lineAt(position.line).text;

    // Find all [^key] patterns in the line
    const refPattern = /\[\^([^\]]+)\]/g;
    let match;

    while ((match = refPattern.exec(line)) !== null) {
        const startCol = match.index;
        const endCol = match.index + match[0].length;

        // Check if position is within this match
        if (position.character >= startCol && position.character <= endCol) {
            const range = new vscode.Range(
                position.line, startCol,
                position.line, endCol
            );
            return { key: match[1], range };
        }
    }

    return null;
}

/**
 * Find stack key at position
 * Looks for key = value pattern in [stack] section
 */
function findStackKeyAtPosition(
    document: vscode.TextDocument,
    position: vscode.Position,
    refs: Map<string, ReferenceInfo>
): { key: string; value: string; range: vscode.Range } | null {
    const line = document.lineAt(position.line).text;
    const trimmed = line.trim();

    // Check if we're in a key = value line
    const eqIndex = trimmed.indexOf('=');
    if (eqIndex === -1) {
        return null;
    }

    const key = trimmed.substring(0, eqIndex).trim();

    // Check if this key exists in refs
    const refInfo = refs.get(key);
    if (!refInfo) {
        return null;
    }

    // Find the key position in the line
    const keyStart = line.indexOf(key);
    if (keyStart === -1) {
        return null;
    }

    // Check if position is on the key
    if (position.character >= keyStart && position.character <= keyStart + key.length) {
        const range = new vscode.Range(
            position.line, keyStart,
            position.line, keyStart + key.length
        );
        return { key, value: refInfo.value, range };
    }

    return null;
}

/**
 * Format reference value for hover display
 * Splits pipe-separated values into readable format
 */
function formatReferenceValue(key: string, value: string): vscode.MarkdownString {
    const md = new vscode.MarkdownString();
    md.isTrusted = true;

    // Check if value contains pipe separators (stack format)
    if (value.includes('|')) {
        const parts = value.split('|').map(p => p.trim());
        md.appendMarkdown(`**${key}**\n\n`);
        md.appendMarkdown('| Value |\n|---|\n');
        for (const part of parts) {
            md.appendMarkdown(`| ${part} |\n`);
        }
    } else {
        md.appendMarkdown(`**${key}**: \`${value}\``);
    }

    return md;
}

/**
 * DX Hover Provider
 * Provides hover information for references and stack keys
 * 
 * Requirements: 5.6, 7.2
 */
export class DxHoverProvider implements vscode.HoverProvider {
    provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.Hover> {
        const content = document.getText();
        const refs = extractReferences(content);

        // Check for reference usage [^key]
        const refUsage = findReferenceAtPosition(document, position);
        if (refUsage) {
            const refInfo = refs.get(refUsage.key);
            if (refInfo) {
                const md = formatReferenceValue(refUsage.key, refInfo.value);
                return new vscode.Hover(md, refUsage.range);
            } else {
                // Reference not found - show error
                const md = new vscode.MarkdownString();
                md.appendMarkdown(`⚠️ **Undefined reference**: \`${refUsage.key}\`\n\n`);
                if (refs.size > 0) {
                    md.appendMarkdown('**Defined references:**\n');
                    for (const key of refs.keys()) {
                        md.appendMarkdown(`- \`${key}\`\n`);
                    }
                }
                return new vscode.Hover(md, refUsage.range);
            }
        }

        // Check for stack key hover
        const stackKey = findStackKeyAtPosition(document, position, refs);
        if (stackKey) {
            const md = formatReferenceValue(stackKey.key, stackKey.value);
            return new vscode.Hover(md, stackKey.range);
        }

        return null;
    }
}

/**
 * Register the hover provider for DX files
 */
export function registerHoverProvider(context: vscode.ExtensionContext): void {
    const hoverProvider = new DxHoverProvider();

    // Register for both file and dxlens schemes
    context.subscriptions.push(
        vscode.languages.registerHoverProvider(
            { scheme: 'file', language: 'dx-serializer' },
            hoverProvider
        )
    );

    context.subscriptions.push(
        vscode.languages.registerHoverProvider(
            { scheme: 'dxlens', language: 'dx-serializer' },
            hoverProvider
        )
    );

    console.log('DX: Hover provider registered');
}
