/**
 * DX Generator Trigger Provider
 * 
 * Detects and handles trigger patterns in the editor for auto-generation.
 * Requirements: 2.1, 2.5
 */

import * as vscode from 'vscode';
import {
    TriggerDefinition,
    TriggerMatch,
    TemplateMetadata,
    GenerateResult,
} from './types';
import { TemplateRegistry } from './templateRegistry';
import { ParameterInput } from './parameterInput';

/**
 * Default trigger patterns
 */
const DEFAULT_TRIGGERS: TriggerDefinition[] = [
    {
        pattern: /\/\/gen:(\w+)(?:\s+(.*))?$/,
        templateId: '$1',
        paramExtractor: (match) => {
            const params: Record<string, string> = {};
            if (match[2]) {
                // Parse key=value pairs
                const pairs = match[2].split(/\s+/);
                for (const pair of pairs) {
                    const [key, value] = pair.split('=');
                    if (key && value) {
                        params[key] = value;
                    }
                }
            }
            return params;
        },
    },
    {
        pattern: /#gen:(\w+)(?:\s+(.*))?$/,
        templateId: '$1',
        paramExtractor: (match) => {
            const params: Record<string, string> = {};
            if (match[2]) {
                const pairs = match[2].split(/\s+/);
                for (const pair of pairs) {
                    const [key, value] = pair.split('=');
                    if (key && value) {
                        params[key] = value;
                    }
                }
            }
            return params;
        },
    },
    {
        pattern: /<!--\s*gen:(\w+)(?:\s+(.*))?\s*-->$/,
        templateId: '$1',
        paramExtractor: (match) => {
            const params: Record<string, string> = {};
            if (match[2]) {
                const pairs = match[2].trim().split(/\s+/);
                for (const pair of pairs) {
                    const [key, value] = pair.split('=');
                    if (key && value) {
                        params[key] = value;
                    }
                }
            }
            return params;
        },
    },
];

/**
 * Trigger provider for detecting and executing generation triggers
 */
export class GeneratorTriggerProvider implements vscode.Disposable {
    private triggers: TriggerDefinition[] = [...DEFAULT_TRIGGERS];
    private disposables: vscode.Disposable[] = [];
    private registry: TemplateRegistry;
    private parameterInput: ParameterInput;

    constructor(registry: TemplateRegistry, parameterInput: ParameterInput) {
        this.registry = registry;
        this.parameterInput = parameterInput;
        this.setupEventListeners();
    }

    /**
     * Register a custom trigger pattern
     */
    registerTrigger(trigger: TriggerDefinition): void {
        this.triggers.push(trigger);
    }

    /**
     * Check if a line contains a trigger
     */
    detectTrigger(line: string): TriggerMatch | null {
        for (const trigger of this.triggers) {
            const match = line.match(trigger.pattern);
            if (match) {
                // Resolve template ID (handle $1 placeholder)
                let templateId = trigger.templateId;
                if (templateId === '$1' && match[1]) {
                    templateId = match[1];
                }

                // Extract parameters
                const params = trigger.paramExtractor
                    ? trigger.paramExtractor(match)
                    : {};

                return {
                    trigger: { ...trigger, templateId },
                    match,
                    params,
                    startIndex: match.index || 0,
                    endIndex: (match.index || 0) + match[0].length,
                };
            }
        }
        return null;
    }

    /**
     * Execute generation for a trigger
     */
    async executeTrigger(
        document: vscode.TextDocument,
        position: vscode.Position,
        triggerMatch: TriggerMatch
    ): Promise<void> {
        const templateId = triggerMatch.trigger.templateId;

        // Get template metadata
        const template = await this.registry.getTemplate(templateId);
        if (!template) {
            vscode.window.showErrorMessage(
                `Template '${templateId}' not found. Run 'dx gen --list' to see available templates.`
            );
            return;
        }

        // Collect missing parameters
        const missingParams = template.parameters.filter(
            (p) => p.required && !(p.name in triggerMatch.params)
        );

        let finalParams = { ...triggerMatch.params };

        if (missingParams.length > 0) {
            const inputParams = await this.parameterInput.promptForParameters(
                template,
                triggerMatch.params
            );
            if (!inputParams) {
                // User cancelled
                return;
            }
            finalParams = { ...finalParams, ...inputParams };
        }

        // Generate content
        const result = await this.registry.generate(templateId, finalParams);

        if (!result.success || !result.content) {
            vscode.window.showErrorMessage(
                `Generation failed: ${result.error || 'Unknown error'}`
            );
            return;
        }

        // Replace trigger with generated content
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document !== document) {
            return;
        }

        const line = document.lineAt(position.line);
        const range = new vscode.Range(
            line.range.start,
            line.range.end
        );

        await editor.edit((editBuilder) => {
            editBuilder.replace(range, result.content!);
        });

        // Position cursor at first editable placeholder
        await this.positionCursorAtPlaceholder(editor, position.line, result.content!);

        // Show token savings
        if (result.tokensSaved && result.tokensSaved > 0) {
            vscode.window.setStatusBarMessage(
                `$(zap) Saved ~${result.tokensSaved} tokens`,
                5000
            );
        }
    }

    /**
     * Position cursor at the first editable placeholder in generated content
     * Supports multiple placeholder formats:
     * - ${1:placeholder} - VS Code snippet style
     * - $0 - Final cursor position
     * - __CURSOR__ - Simple marker
     */
    private async positionCursorAtPlaceholder(
        editor: vscode.TextEditor,
        startLine: number,
        content: string
    ): Promise<void> {
        const lines = content.split('\n');

        // Look for placeholder patterns in order of priority
        const patterns = [
            /\$\{1:([^}]*)\}/,      // ${1:placeholder}
            /\$\{0\}/,              // ${0} - final position
            /\$0/,                  // $0 - final position
            /__CURSOR__/,           // __CURSOR__
        ];

        for (let lineOffset = 0; lineOffset < lines.length; lineOffset++) {
            const line = lines[lineOffset];

            for (const pattern of patterns) {
                const match = line.match(pattern);
                if (match && match.index !== undefined) {
                    const lineNumber = startLine + lineOffset;
                    const charPosition = match.index;

                    // If it's a snippet placeholder with default text, select it
                    if (match[1]) {
                        const startPos = new vscode.Position(lineNumber, charPosition);
                        const endPos = new vscode.Position(
                            lineNumber,
                            charPosition + match[0].length
                        );

                        // Replace the placeholder marker with just the default text
                        const defaultText = match[1];
                        await editor.edit((editBuilder) => {
                            editBuilder.replace(
                                new vscode.Range(startPos, endPos),
                                defaultText
                            );
                        });

                        // Select the default text
                        const selectStart = new vscode.Position(lineNumber, charPosition);
                        const selectEnd = new vscode.Position(
                            lineNumber,
                            charPosition + defaultText.length
                        );
                        editor.selection = new vscode.Selection(selectStart, selectEnd);
                    } else {
                        // Just position cursor and remove marker
                        const startPos = new vscode.Position(lineNumber, charPosition);
                        const endPos = new vscode.Position(
                            lineNumber,
                            charPosition + match[0].length
                        );

                        await editor.edit((editBuilder) => {
                            editBuilder.delete(new vscode.Range(startPos, endPos));
                        });

                        editor.selection = new vscode.Selection(startPos, startPos);
                    }

                    return;
                }
            }
        }

        // No placeholder found, position at end of first line
        const endOfFirstLine = new vscode.Position(
            startLine,
            lines[0].length
        );
        editor.selection = new vscode.Selection(endOfFirstLine, endOfFirstLine);
    }

    /**
     * Set up event listeners for trigger detection
     */
    private setupEventListeners(): void {
        // Listen for Enter key to trigger generation
        this.disposables.push(
            vscode.commands.registerCommand(
                'dx.generator.checkTrigger',
                async () => {
                    const editor = vscode.window.activeTextEditor;
                    if (!editor) {
                        return;
                    }

                    const position = editor.selection.active;
                    const line = editor.document.lineAt(position.line);
                    const triggerMatch = this.detectTrigger(line.text);

                    if (triggerMatch) {
                        await this.executeTrigger(
                            editor.document,
                            position,
                            triggerMatch
                        );
                    } else {
                        // No trigger found, execute default Enter behavior
                        await vscode.commands.executeCommand(
                            'default:type',
                            { text: '\n' }
                        );
                    }
                }
            )
        );
    }

    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}
