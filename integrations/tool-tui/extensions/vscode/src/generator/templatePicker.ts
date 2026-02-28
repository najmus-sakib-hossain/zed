/**
 * DX Generator Template Picker
 * 
 * Provides a searchable template picker UI for VS Code.
 * Requirements: 2.2, 2.3
 */

import * as vscode from 'vscode';
import { TemplateMetadata } from './types';
import { TemplateRegistry } from './templateRegistry';

/**
 * Quick pick item for template selection
 */
interface TemplateQuickPickItem extends vscode.QuickPickItem {
    template: TemplateMetadata;
}

/**
 * Template picker for selecting templates from the registry
 */
export class TemplatePicker {
    private registry: TemplateRegistry;

    constructor(registry: TemplateRegistry) {
        this.registry = registry;
    }

    /**
     * Show the template picker and return the selected template
     */
    async pickTemplate(): Promise<TemplateMetadata | undefined> {
        const templates = await this.registry.listTemplates();

        if (templates.length === 0) {
            vscode.window.showInformationMessage(
                'No templates found. Run "dx gen --init" to create example templates.'
            );
            return undefined;
        }

        const items: TemplateQuickPickItem[] = templates.map((template) => ({
            label: `$(file-code) ${template.name}`,
            description: template.version,
            detail: template.description,
            template,
        }));

        const quickPick = vscode.window.createQuickPick<TemplateQuickPickItem>();
        quickPick.items = items;
        quickPick.placeholder = 'Search templates...';
        quickPick.matchOnDescription = true;
        quickPick.matchOnDetail = true;

        return new Promise((resolve) => {
            quickPick.onDidAccept(() => {
                const selected = quickPick.selectedItems[0];
                quickPick.hide();
                resolve(selected?.template);
            });

            quickPick.onDidHide(() => {
                quickPick.dispose();
                resolve(undefined);
            });

            quickPick.show();
        });
    }

    /**
     * Show template picker filtered by category
     */
    async pickTemplateByCategory(
        category: string
    ): Promise<TemplateMetadata | undefined> {
        const templates = await this.registry.searchTemplates(category);

        if (templates.length === 0) {
            vscode.window.showInformationMessage(
                `No templates found in category '${category}'.`
            );
            return undefined;
        }

        const items: TemplateQuickPickItem[] = templates.map((template) => ({
            label: `$(file-code) ${template.name}`,
            description: template.tags.join(', '),
            detail: template.description,
            template,
        }));

        const selected = await vscode.window.showQuickPick(items, {
            placeHolder: `Select a ${category} template`,
            matchOnDescription: true,
            matchOnDetail: true,
        });

        return selected?.template;
    }

    /**
     * Show template details in a preview
     */
    async showTemplateDetails(template: TemplateMetadata): Promise<void> {
        const content = this.formatTemplateDetails(template);

        const doc = await vscode.workspace.openTextDocument({
            content,
            language: 'markdown',
        });

        await vscode.window.showTextDocument(doc, {
            viewColumn: vscode.ViewColumn.Beside,
            preview: true,
            preserveFocus: true,
        });
    }

    /**
     * Format template details as markdown
     */
    private formatTemplateDetails(template: TemplateMetadata): string {
        const lines: string[] = [
            `# ${template.name}`,
            '',
            template.description,
            '',
            '## Details',
            '',
            `- **Version:** ${template.version}`,
            `- **ID:** ${template.id}`,
        ];

        if (template.author) {
            lines.push(`- **Author:** ${template.author}`);
        }

        if (template.tags.length > 0) {
            lines.push(`- **Tags:** ${template.tags.join(', ')}`);
        }

        lines.push('', '## Parameters', '');

        if (template.parameters.length === 0) {
            lines.push('*No parameters required*');
        } else {
            lines.push('| Name | Type | Required | Description |');
            lines.push('|------|------|----------|-------------|');

            for (const param of template.parameters) {
                const required = param.required ? '✓' : '';
                const defaultVal = param.default !== undefined
                    ? ` (default: ${param.default})`
                    : '';
                lines.push(
                    `| ${param.name} | ${param.valueType} | ${required} | ${param.description}${defaultVal} |`
                );
            }
        }

        lines.push('', '## Output Pattern', '', `\`${template.outputPattern}\``);

        if (template.dependencies.length > 0) {
            lines.push(
                '',
                '## Dependencies',
                '',
                template.dependencies.map((d) => `- ${d}`).join('\n')
            );
        }

        return lines.join('\n');
    }
}
