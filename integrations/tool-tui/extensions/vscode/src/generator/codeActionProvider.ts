/**
 * DX Generator Code Action Provider
 * 
 * Provides inline code actions for trigger patterns.
 * Requirements: 10.8
 */

import * as vscode from 'vscode';
import { TemplateRegistry } from './templateRegistry';
import { GeneratorTriggerProvider } from './triggerProvider';

/**
 * Code action provider for generator triggers
 */
export class GeneratorCodeActionProvider implements vscode.CodeActionProvider {
    private registry: TemplateRegistry;
    private triggerProvider: GeneratorTriggerProvider;

    static readonly providedCodeActionKinds = [
        vscode.CodeActionKind.QuickFix,
        vscode.CodeActionKind.Refactor,
    ];

    constructor(
        registry: TemplateRegistry,
        triggerProvider: GeneratorTriggerProvider
    ) {
        this.registry = registry;
        this.triggerProvider = triggerProvider;
    }

    async provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range | vscode.Selection,
        _context: vscode.CodeActionContext,
        _token: vscode.CancellationToken
    ): Promise<vscode.CodeAction[]> {
        const actions: vscode.CodeAction[] = [];
        const line = document.lineAt(range.start.line);
        const triggerMatch = this.triggerProvider.detectTrigger(line.text);

        if (triggerMatch) {
            // Generate action
            const generateAction = new vscode.CodeAction(
                `Generate from ${triggerMatch.trigger.templateId}`,
                vscode.CodeActionKind.QuickFix
            );
            generateAction.command = {
                command: 'dx.generator.generateById',
                title: 'Generate',
                arguments: [triggerMatch.trigger.templateId, triggerMatch.params],
            };
            generateAction.isPreferred = true;
            actions.push(generateAction);

            // Preview action
            const previewAction = new vscode.CodeAction(
                `Preview ${triggerMatch.trigger.templateId} output`,
                vscode.CodeActionKind.Refactor
            );
            previewAction.command = {
                command: 'dx.generator.previewTemplate',
                title: 'Preview',
                arguments: [triggerMatch.trigger.templateId, triggerMatch.params],
            };
            actions.push(previewAction);

            // Edit parameters action
            const editAction = new vscode.CodeAction(
                `Edit parameters for ${triggerMatch.trigger.templateId}`,
                vscode.CodeActionKind.Refactor
            );
            editAction.command = {
                command: 'dx.generator.editTriggerParams',
                title: 'Edit Parameters',
                arguments: [document, range.start, triggerMatch],
            };
            actions.push(editAction);
        }

        // Check if line could be a trigger pattern
        const potentialTrigger = this.detectPotentialTrigger(line.text);
        if (potentialTrigger && !triggerMatch) {
            const suggestAction = new vscode.CodeAction(
                `Convert to generator trigger`,
                vscode.CodeActionKind.Refactor
            );
            suggestAction.command = {
                command: 'dx.generator.suggestTrigger',
                title: 'Suggest Trigger',
                arguments: [document, range.start, potentialTrigger],
            };
            actions.push(suggestAction);
        }

        return actions;
    }

    /**
     * Detect if a line could be converted to a trigger
     */
    private detectPotentialTrigger(line: string): string | null {
        // Look for comments that mention generation
        const patterns = [
            /\/\/\s*TODO:\s*generate\s+(\w+)/i,
            /\/\/\s*create\s+(\w+)/i,
            /\/\*\s*generate\s+(\w+)\s*\*\//i,
            /#\s*TODO:\s*generate\s+(\w+)/i,
        ];

        for (const pattern of patterns) {
            const match = line.match(pattern);
            if (match) {
                return match[1];
            }
        }

        return null;
    }
}

/**
 * Register the code action provider and related commands
 */
export function registerGeneratorCodeActions(
    context: vscode.ExtensionContext,
    registry: TemplateRegistry,
    triggerProvider: GeneratorTriggerProvider
): void {
    const codeActionProvider = new GeneratorCodeActionProvider(registry, triggerProvider);

    // Register for common file types
    const languages = [
        'typescript',
        'typescriptreact',
        'javascript',
        'javascriptreact',
        'rust',
        'python',
        'go',
        'java',
        'csharp',
        'cpp',
        'c',
        'html',
        'css',
        'scss',
        'json',
        'yaml',
        'markdown',
        'plaintext',
    ];

    for (const language of languages) {
        context.subscriptions.push(
            vscode.languages.registerCodeActionsProvider(
                { language },
                codeActionProvider,
                {
                    providedCodeActionKinds: GeneratorCodeActionProvider.providedCodeActionKinds,
                }
            )
        );
    }

    // Register preview command
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.previewTemplate',
            async (templateId: string, params: Record<string, string>) => {
                const template = await registry.getTemplate(templateId);
                if (!template) {
                    vscode.window.showErrorMessage(`Template '${templateId}' not found`);
                    return;
                }

                // Generate preview with placeholder values
                const previewParams = { ...params };
                for (const param of template.parameters) {
                    if (!(param.name in previewParams)) {
                        previewParams[param.name] = param.default?.toString() || `<${param.name}>`;
                    }
                }

                const result = await registry.generate(templateId, previewParams);
                if (!result.success || !result.content) {
                    vscode.window.showErrorMessage(`Preview failed: ${result.error}`);
                    return;
                }

                // Show preview in new document
                const doc = await vscode.workspace.openTextDocument({
                    content: result.content,
                    language: detectLanguageFromTemplate(template.outputPattern),
                });
                await vscode.window.showTextDocument(doc, {
                    viewColumn: vscode.ViewColumn.Beside,
                    preview: true,
                    preserveFocus: true,
                });
            }
        )
    );

    // Register edit parameters command
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.editTriggerParams',
            async (document: vscode.TextDocument, position: vscode.Position, triggerMatch: any) => {
                const template = await registry.getTemplate(triggerMatch.trigger.templateId);
                if (!template) {
                    return;
                }

                // Show quick pick for each parameter
                const newParams: Record<string, string> = { ...triggerMatch.params };

                for (const param of template.parameters) {
                    const currentValue = newParams[param.name] || param.default?.toString() || '';
                    const value = await vscode.window.showInputBox({
                        prompt: `${param.name} (${param.valueType})`,
                        value: currentValue,
                        placeHolder: param.description || param.name,
                    });

                    if (value === undefined) {
                        return; // User cancelled
                    }

                    if (value) {
                        newParams[param.name] = value;
                    }
                }

                // Rebuild trigger line with new params
                const paramStr = Object.entries(newParams)
                    .map(([k, v]) => `${k}=${v}`)
                    .join(' ');

                const newTrigger = `//gen:${triggerMatch.trigger.templateId} ${paramStr}`;

                const editor = vscode.window.activeTextEditor;
                if (editor && editor.document === document) {
                    const line = document.lineAt(position.line);
                    await editor.edit(editBuilder => {
                        editBuilder.replace(line.range, newTrigger);
                    });
                }
            }
        )
    );

    // Register suggest trigger command
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.suggestTrigger',
            async (document: vscode.TextDocument, position: vscode.Position, suggestion: string) => {
                const templates = await registry.searchTemplates(suggestion);

                if (templates.length === 0) {
                    vscode.window.showInformationMessage(
                        `No templates found matching '${suggestion}'`
                    );
                    return;
                }

                const items = templates.map(t => ({
                    label: t.name,
                    description: t.description,
                    template: t,
                }));

                const selected = await vscode.window.showQuickPick(items, {
                    placeHolder: 'Select a template',
                });

                if (selected) {
                    const editor = vscode.window.activeTextEditor;
                    if (editor && editor.document === document) {
                        const line = document.lineAt(position.line);
                        const newTrigger = `//gen:${selected.template.id}`;
                        await editor.edit(editBuilder => {
                            editBuilder.replace(line.range, newTrigger);
                        });
                    }
                }
            }
        )
    );
}

/**
 * Detect language from output pattern
 */
function detectLanguageFromTemplate(outputPattern: string): string {
    const ext = outputPattern.split('.').pop()?.toLowerCase();
    const languageMap: Record<string, string> = {
        'ts': 'typescript',
        'tsx': 'typescriptreact',
        'js': 'javascript',
        'jsx': 'javascriptreact',
        'rs': 'rust',
        'py': 'python',
        'go': 'go',
        'java': 'java',
        'cs': 'csharp',
        'cpp': 'cpp',
        'c': 'c',
        'html': 'html',
        'css': 'css',
        'scss': 'scss',
        'json': 'json',
        'yaml': 'yaml',
        'yml': 'yaml',
        'md': 'markdown',
        'sql': 'sql',
    };
    return languageMap[ext || ''] || 'plaintext';
}
