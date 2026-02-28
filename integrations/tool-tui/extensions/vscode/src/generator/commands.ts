/**
 * DX Generator Commands
 * 
 * Registers generator commands for VS Code command palette and context menu.
 * Requirements: 2.2, 2.3
 */

import * as vscode from 'vscode';
import { TemplateRegistry } from './templateRegistry';
import { TemplatePicker } from './templatePicker';
import { ParameterInput } from './parameterInput';
import { GeneratorTriggerProvider } from './triggerProvider';

/**
 * Register all generator commands
 */
export function registerGeneratorCommands(
    context: vscode.ExtensionContext,
    registry: TemplateRegistry,
    picker: TemplatePicker,
    parameterInput: ParameterInput,
    triggerProvider: GeneratorTriggerProvider
): void {
    // DX: Generate from Template (command palette)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.generate', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DX Generator: No active editor');
                return;
            }

            // Show template picker
            const template = await picker.pickTemplate();
            if (!template) {
                return; // User cancelled
            }

            // Collect parameters
            const params = await parameterInput.promptForParameters(template, {});
            if (!params) {
                return; // User cancelled
            }

            // Generate content
            const result = await registry.generate(template.id, params);
            if (!result.success || !result.content) {
                vscode.window.showErrorMessage(
                    `Generation failed: ${result.error || 'Unknown error'}`
                );
                return;
            }

            // Insert at cursor position
            const position = editor.selection.active;
            await editor.edit((editBuilder) => {
                editBuilder.insert(position, result.content!);
            });

            // Show success message with token savings
            if (result.tokensSaved && result.tokensSaved > 0) {
                vscode.window.setStatusBarMessage(
                    `$(zap) Generated ${template.name} - Saved ~${result.tokensSaved} tokens`,
                    5000
                );
            } else {
                vscode.window.setStatusBarMessage(
                    `$(check) Generated ${template.name}`,
                    3000
                );
            }
        })
    );


    // DX: Generate from Template (context menu - same as command palette)
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.generateFromContext',
            async () => {
                await vscode.commands.executeCommand('dx.generator.generate');
            }
        )
    );

    // DX: List Templates
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.listTemplates', async () => {
            const templates = await registry.listTemplates();

            if (templates.length === 0) {
                vscode.window.showInformationMessage(
                    'No templates found. Run "dx gen --init" to create example templates.'
                );
                return;
            }

            const items = templates.map((t) => ({
                label: `$(file-code) ${t.name}`,
                description: t.version,
                detail: t.description,
            }));

            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Available templates',
                matchOnDescription: true,
                matchOnDetail: true,
            });

            if (selected) {
                // Find the template and show details
                const template = templates.find(
                    (t) => `$(file-code) ${t.name}` === selected.label
                );
                if (template) {
                    await picker.showTemplateDetails(template);
                }
            }
        })
    );

    // DX: Refresh Templates
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.refreshTemplates',
            async () => {
                await registry.refresh();
                vscode.window.showInformationMessage('DX Generator: Templates refreshed');
            }
        )
    );

    // DX: Check Trigger (for Enter key binding)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.checkTrigger', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                return;
            }

            const position = editor.selection.active;
            const line = editor.document.lineAt(position.line);
            const triggerMatch = triggerProvider.detectTrigger(line.text);

            if (triggerMatch) {
                await triggerProvider.executeTrigger(
                    editor.document,
                    position,
                    triggerMatch
                );
            } else {
                // No trigger found, execute default Enter behavior
                await vscode.commands.executeCommand('default:type', { text: '\n' });
            }
        })
    );

    // DX: Generate by Template ID (for programmatic use)
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.generateById',
            async (templateId: string, params?: Record<string, string>) => {
                const editor = vscode.window.activeTextEditor;
                if (!editor) {
                    vscode.window.showWarningMessage('DX Generator: No active editor');
                    return;
                }

                const template = await registry.getTemplate(templateId);
                if (!template) {
                    vscode.window.showErrorMessage(
                        `Template '${templateId}' not found`
                    );
                    return;
                }

                // Collect missing parameters
                let finalParams = params || {};
                const missingParams = template.parameters.filter(
                    (p) => p.required && !(p.name in finalParams)
                );

                if (missingParams.length > 0) {
                    const inputParams = await parameterInput.promptForParameters(
                        template,
                        finalParams
                    );
                    if (!inputParams) {
                        return;
                    }
                    finalParams = { ...finalParams, ...inputParams };
                }

                // Generate content
                const result = await registry.generate(templateId, finalParams);
                if (!result.success || !result.content) {
                    vscode.window.showErrorMessage(
                        `Generation failed: ${result.error || 'Unknown error'}`
                    );
                    return;
                }

                // Insert at cursor position
                const position = editor.selection.active;
                await editor.edit((editBuilder) => {
                    editBuilder.insert(position, result.content!);
                });

                return result;
            }
        )
    );

    // DX: Search Templates
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'dx.generator.searchTemplates',
            async () => {
                const query = await vscode.window.showInputBox({
                    prompt: 'Search templates',
                    placeHolder: 'Enter search query...',
                });

                if (!query) {
                    return;
                }

                const templates = await registry.searchTemplates(query);

                if (templates.length === 0) {
                    vscode.window.showInformationMessage(
                        `No templates found matching '${query}'`
                    );
                    return;
                }

                const items = templates.map((t) => ({
                    label: `$(file-code) ${t.name}`,
                    description: t.tags.join(', '),
                    detail: t.description,
                    template: t,
                }));

                const selected = await vscode.window.showQuickPick(items, {
                    placeHolder: `Found ${templates.length} template(s)`,
                });

                if (selected) {
                    // Generate from selected template
                    await vscode.commands.executeCommand(
                        'dx.generator.generateById',
                        selected.template.id
                    );
                }
            }
        )
    );
}
