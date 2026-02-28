/**
 * Export Converter for DX Serializer
 * 
 * Converts DX Human format to other formats:
 * - TOON (token-optimized notation)
 * - JSON / JSON Compact
 * - YAML
 * - TOML
 * - CSV
 * 
 * Shown in status bar when viewing Human format of DX serializer files.
 */

import * as vscode from 'vscode';
import { parseHuman } from './humanParser';
import { DxDocument, DxValue, DxSection, getFieldName } from './llmParser';
import { isDxSerializerFile, getCurrentFormat } from './formatViewProvider';

// ============================================================================
// Value Conversion Helpers
// ============================================================================

function dxValueToJs(value: DxValue): unknown {
    switch (value.type) {
        case 'string':
            return value.value;
        case 'number':
            return value.value;
        case 'bool':
            return value.value;
        case 'null':
            return null;
        case 'array':
            return (value.value as DxValue[]).map(v => dxValueToJs(v));
        default:
            return String(value.value);
    }
}

function escapeYamlString(s: string): string {
    if (/[:\[\]{}#&*!|>'"%@`]/.test(s) || s.includes('\n') || s.trim() !== s) {
        return `"${s.replace(/"/g, '\\"')}"`;
    }
    return s;
}

function escapeTomlString(s: string): string {
    return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

// ============================================================================
// Format Converters
// ============================================================================

/**
 * Convert DxDocument to TOON format
 */
function toToon(doc: DxDocument): string {
    const lines: string[] = [];
    
    // Root scalars
    for (const [key, value] of doc.context) {
        const v = dxValueToJs(value);
        if (typeof v === 'string') {
            lines.push(`${key}: ${v}`);
        } else if (Array.isArray(v)) {
            lines.push(`${key}:`);
            for (const item of v) {
                lines.push(`  - ${item}`);
            }
        } else {
            lines.push(`${key}: ${v}`);
        }
    }
    
    // Sections
    for (const [sectionId, section] of doc.sections) {
        lines.push('');
        lines.push(`${sectionId}:`);
        
        if (section.rows.length === 1) {
            // Single row - inline object
            const row = section.rows[0];
            for (let i = 0; i < section.schema.length && i < row.length; i++) {
                const key = getFieldName(section.schema[i]);
                const value = dxValueToJs(row[i]);
                if (Array.isArray(value)) {
                    lines.push(`  ${key}:`);
                    for (const item of value) {
                        lines.push(`    - ${item}`);
                    }
                } else {
                    lines.push(`  ${key}: ${value}`);
                }
            }
        } else {
            // Multiple rows - tabular
            for (const row of section.rows) {
                const values = row.map(v => dxValueToJs(v));
                lines.push(`  - ${values.join(', ')}`);
            }
        }
    }
    
    return lines.join('\n');
}

/**
 * Convert DxDocument to JSON
 */
function toJson(doc: DxDocument, compact: boolean = false): string {
    const obj: Record<string, unknown> = {};
    
    // Root scalars
    for (const [key, value] of doc.context) {
        obj[key] = dxValueToJs(value);
    }
    
    // Sections
    for (const [sectionId, section] of doc.sections) {
        if (section.rows.length === 1) {
            // Single row - object
            const row = section.rows[0];
            const sectionObj: Record<string, unknown> = {};
            for (let i = 0; i < section.schema.length && i < row.length; i++) {
                const key = getFieldName(section.schema[i]);
                sectionObj[key] = dxValueToJs(row[i]);
            }
            obj[sectionId] = sectionObj;
        } else {
            // Multiple rows - array of objects
            const rows: Record<string, unknown>[] = [];
            for (const row of section.rows) {
                const rowObj: Record<string, unknown> = {};
                for (let i = 0; i < section.schema.length && i < row.length; i++) {
                    const key = getFieldName(section.schema[i]);
                    rowObj[key] = dxValueToJs(row[i]);
                }
                rows.push(rowObj);
            }
            obj[sectionId] = rows;
        }
    }
    
    return compact ? JSON.stringify(obj) : JSON.stringify(obj, null, 2);
}

/**
 * Convert DxDocument to YAML
 */
function toYaml(doc: DxDocument): string {
    const lines: string[] = [];
    
    // Root scalars
    for (const [key, value] of doc.context) {
        const v = dxValueToJs(value);
        if (typeof v === 'string') {
            lines.push(`${key}: ${escapeYamlString(v)}`);
        } else if (Array.isArray(v)) {
            lines.push(`${key}:`);
            for (const item of v) {
                lines.push(`  - ${escapeYamlString(String(item))}`);
            }
        } else {
            lines.push(`${key}: ${v}`);
        }
    }
    
    // Sections
    for (const [sectionId, section] of doc.sections) {
        lines.push('');
        lines.push(`${sectionId}:`);
        
        if (section.rows.length === 1) {
            const row = section.rows[0];
            for (let i = 0; i < section.schema.length && i < row.length; i++) {
                const key = getFieldName(section.schema[i]);
                const value = dxValueToJs(row[i]);
                if (Array.isArray(value)) {
                    lines.push(`  ${key}:`);
                    for (const item of value) {
                        lines.push(`    - ${escapeYamlString(String(item))}`);
                    }
                } else if (typeof value === 'string') {
                    lines.push(`  ${key}: ${escapeYamlString(value)}`);
                } else {
                    lines.push(`  ${key}: ${value}`);
                }
            }
        } else {
            for (const row of section.rows) {
                const rowObj: string[] = [];
                for (let i = 0; i < section.schema.length && i < row.length; i++) {
                    const key = getFieldName(section.schema[i]);
                    const value = dxValueToJs(row[i]);
                    rowObj.push(`${key}: ${typeof value === 'string' ? escapeYamlString(value) : value}`);
                }
                lines.push(`  - { ${rowObj.join(', ')} }`);
            }
        }
    }
    
    return lines.join('\n');
}

/**
 * Convert DxDocument to TOML
 */
function toToml(doc: DxDocument): string {
    const lines: string[] = [];
    
    // Root scalars
    for (const [key, value] of doc.context) {
        const v = dxValueToJs(value);
        if (typeof v === 'string') {
            lines.push(`${key} = ${escapeTomlString(v)}`);
        } else if (Array.isArray(v)) {
            const items = v.map(item => 
                typeof item === 'string' ? escapeTomlString(String(item)) : item
            );
            lines.push(`${key} = [${items.join(', ')}]`);
        } else if (typeof v === 'boolean') {
            lines.push(`${key} = ${v}`);
        } else {
            lines.push(`${key} = ${v}`);
        }
    }
    
    // Sections
    for (const [sectionId, section] of doc.sections) {
        lines.push('');
        
        if (section.rows.length === 1) {
            lines.push(`[${sectionId}]`);
            const row = section.rows[0];
            for (let i = 0; i < section.schema.length && i < row.length; i++) {
                const key = getFieldName(section.schema[i]);
                const value = dxValueToJs(row[i]);
                if (Array.isArray(value)) {
                    const items = value.map(item => 
                        typeof item === 'string' ? escapeTomlString(String(item)) : item
                    );
                    lines.push(`${key} = [${items.join(', ')}]`);
                } else if (typeof value === 'string') {
                    lines.push(`${key} = ${escapeTomlString(value)}`);
                } else if (typeof value === 'boolean') {
                    lines.push(`${key} = ${value}`);
                } else {
                    lines.push(`${key} = ${value}`);
                }
            }
        } else {
            // Array of tables
            for (const row of section.rows) {
                lines.push(`[[${sectionId}]]`);
                for (let i = 0; i < section.schema.length && i < row.length; i++) {
                    const key = getFieldName(section.schema[i]);
                    const value = dxValueToJs(row[i]);
                    if (typeof value === 'string') {
                        lines.push(`${key} = ${escapeTomlString(value)}`);
                    } else {
                        lines.push(`${key} = ${value}`);
                    }
                }
            }
        }
    }
    
    return lines.join('\n');
}

/**
 * Convert DxDocument to CSV
 */
function toCsv(doc: DxDocument): string {
    const lines: string[] = [];
    
    // Find the first section with multiple rows for CSV
    let targetSection: DxSection | null = null;
    let targetId = '';
    
    for (const [sectionId, section] of doc.sections) {
        if (section.rows.length > 1) {
            targetSection = section;
            targetId = sectionId;
            break;
        }
    }
    
    if (!targetSection) {
        // No tabular data - create CSV from all sections
        const allKeys = new Set<string>();
        const allRows: Record<string, unknown>[] = [];
        
        // Add root context as first row
        const rootRow: Record<string, unknown> = {};
        for (const [key, value] of doc.context) {
            allKeys.add(key);
            rootRow[key] = dxValueToJs(value);
        }
        if (Object.keys(rootRow).length > 0) {
            allRows.push(rootRow);
        }
        
        // Add sections
        for (const [sectionId, section] of doc.sections) {
            if (section.rows.length === 1) {
                const row = section.rows[0];
                const rowObj: Record<string, unknown> = { section: sectionId };
                allKeys.add('section');
                for (let i = 0; i < section.schema.length && i < row.length; i++) {
                    const key = getFieldName(section.schema[i]);
                    allKeys.add(key);
                    rowObj[key] = dxValueToJs(row[i]);
                }
                allRows.push(rowObj);
            }
        }
        
        // Generate CSV
        const headers = Array.from(allKeys);
        lines.push(headers.join(','));
        
        for (const row of allRows) {
            const values = headers.map(h => {
                const v = row[h];
                if (v === undefined || v === null) return '';
                if (typeof v === 'string' && (v.includes(',') || v.includes('"'))) {
                    return `"${v.replace(/"/g, '""')}"`;
                }
                return String(v);
            });
            lines.push(values.join(','));
        }
    } else {
        // Use the tabular section
        const headers = targetSection.schema.map(f => getFieldName(f));
        lines.push(headers.join(','));
        
        for (const row of targetSection.rows) {
            const values = row.map(v => {
                const val = dxValueToJs(v);
                if (val === null || val === undefined) return '';
                if (typeof val === 'string' && (val.includes(',') || val.includes('"'))) {
                    return `"${val.replace(/"/g, '""')}"`;
                }
                return String(val);
            });
            lines.push(values.join(','));
        }
    }
    
    return lines.join('\n');
}

// ============================================================================
// Export Converter Status Bar
// ============================================================================

export class ExportConverterStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            95
        );
        this.statusBarItem.command = 'dx.export.showMenu';
        this.statusBarItem.text = '$(export) Export';
        this.statusBarItem.tooltip = 'Export to TOON, JSON, YAML, TOML, CSV';

        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                this.updateVisibility(editor);
            })
        );

        this.updateVisibility(vscode.window.activeTextEditor);
    }

    private updateVisibility(editor: vscode.TextEditor | undefined): void {
        if (!editor || !isDxSerializerFile(editor.document.uri)) {
            this.hide();
            return;
        }

        const format = getCurrentFormat(editor.document.uri);
        if (format === 'human') {
            this.show();
        } else {
            this.hide();
        }
    }

    show(): void {
        this.statusBarItem.show();
    }

    hide(): void {
        this.statusBarItem.hide();
    }

    dispose(): void {
        this.statusBarItem.dispose();
        for (const d of this.disposables) d.dispose();
    }
}

/**
 * Register export converter commands
 */
export function registerExportConverterCommands(
    context: vscode.ExtensionContext,
    exportStatusBar: ExportConverterStatusBar
): void {
    // Show export menu
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.showMenu', async () => {
            const options = [
                { label: '$(file-code) TOON', description: 'Token-optimized notation', format: 'toon' },
                { label: '$(json) JSON', description: 'Pretty-printed JSON', format: 'json' },
                { label: '$(json) JSON Compact', description: 'Minified JSON', format: 'json-compact' },
                { label: '$(file-code) YAML', description: 'YAML format', format: 'yaml' },
                { label: '$(file-code) TOML', description: 'TOML format', format: 'toml' },
                { label: '$(table) CSV', description: 'Comma-separated values', format: 'csv' },
            ];

            const selected = await vscode.window.showQuickPick(options, {
                placeHolder: 'Select export format',
            });

            if (selected) {
                await exportToFormat(selected.format);
            }
        })
    );

    // Individual export commands
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.toToon', () => exportToFormat('toon'))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.toJson', () => exportToFormat('json'))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.toJsonCompact', () => exportToFormat('json-compact'))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.toYaml', () => exportToFormat('yaml'))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.toToml', () => exportToFormat('toml'))
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.export.toCsv', () => exportToFormat('csv'))
    );
}

/**
 * Export current document to specified format
 */
async function exportToFormat(format: string): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No active editor');
        return;
    }

    const content = editor.document.getText();
    const parseResult = parseHuman(content);

    if (!parseResult.success || !parseResult.document) {
        vscode.window.showErrorMessage('Failed to parse document');
        return;
    }

    const doc = parseResult.document;
    let output: string;
    let extension: string;
    let languageId: string;

    switch (format) {
        case 'toon':
            output = toToon(doc);
            extension = 'toon';
            languageId = 'yaml';
            break;
        case 'json':
            output = toJson(doc, false);
            extension = 'json';
            languageId = 'json';
            break;
        case 'json-compact':
            output = toJson(doc, true);
            extension = 'json';
            languageId = 'json';
            break;
        case 'yaml':
            output = toYaml(doc);
            extension = 'yaml';
            languageId = 'yaml';
            break;
        case 'toml':
            output = toToml(doc);
            extension = 'toml';
            languageId = 'toml';
            break;
        case 'csv':
            output = toCsv(doc);
            extension = 'csv';
            languageId = 'plaintext';
            break;
        default:
            return;
    }

    // Open in new editor
    const newDoc = await vscode.workspace.openTextDocument({
        content: output,
        language: languageId,
    });

    await vscode.window.showTextDocument(newDoc, { preview: true });
}

