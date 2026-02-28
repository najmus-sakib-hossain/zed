/**
 * DX Generator Hover Provider
 * 
 * Provides hover previews for trigger patterns in the editor.
 * Requirements: 2.6
 */

import * as vscode from 'vscode';
import { TemplateRegistry } from './templateRegistry';
import { GeneratorTriggerProvider } from './triggerProvider';

/**
 * Hover provider for generator triggers
 */
export class GeneratorHoverProvider implements vscode.HoverProvider {
    private registry: TemplateRegistry;
    private triggerProvider: GeneratorTriggerProvider;

    constructor(
        registry: TemplateRegistry,
        triggerProvider: GeneratorTriggerProvider
    ) {
        this.registry = registry;
        this.triggerProvider = triggerProvider;
    }

    async provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): Promise<vscode.Hover | null> {
        const line = document.lineAt(position.line);
        const triggerMatch = this.triggerProvider.detectTrigger(line.text);

        if (!triggerMatch) {
            return null;
        }

        // Check if cursor is within the trigger range
        if (
            position.character < triggerMatch.startIndex ||
            position.character > triggerMatch.endIndex
        ) {
            return null;
        }

        const templateId = triggerMatch.trigger.templateId;
        const template = await this.registry.getTemplate(templateId);

        if (!template) {
            return new vscode.Hover(
                new vscode.MarkdownString(
                    `⚠️ Template \`${templateId}\` not found\n\nRun \`dx gen --list\` to see available templates.`
                ),
                new vscode.Range(
                    position.line,
                    triggerMatch.startIndex,
                    position.line,
                    triggerMatch.endIndex
                )
            );
        }

        // Generate preview content
        const preview = await this.generatePreview(
            templateId,
            triggerMatch.params
        );

        // Build hover content
        const markdown = new vscode.MarkdownString();
        markdown.isTrusted = true;
        markdown.supportHtml = true;

        // Template info
        markdown.appendMarkdown(`## 📄 ${template.name}\n\n`);
        markdown.appendMarkdown(`${template.description}\n\n`);

        // Parameters section
        if (template.parameters.length > 0) {
            markdown.appendMarkdown(`### Parameters\n\n`);

            for (const param of template.parameters) {
                const provided = param.name in triggerMatch.params;
                const icon = provided ? '✅' : param.required ? '❌' : '⚪';
                const value = triggerMatch.params[param.name];

                markdown.appendMarkdown(
                    `${icon} **${param.name}** (${param.valueType})${param.required ? ' *required*' : ''}`
                );

                if (value) {
                    markdown.appendMarkdown(` = \`${value}\``);
                } else if (param.default !== undefined) {
                    markdown.appendMarkdown(` = \`${param.default}\` (default)`);
                }

                markdown.appendMarkdown(`\n`);
            }
            markdown.appendMarkdown(`\n`);
        }

        // Preview section
        if (preview) {
            markdown.appendMarkdown(`### Preview\n\n`);
            markdown.appendCodeblock(
                this.truncatePreview(preview, 500),
                this.detectLanguage(template.outputPattern)
            );
        }

        // Action hint
        markdown.appendMarkdown(`\n---\n`);
        markdown.appendMarkdown(`*Press Enter to generate*`);

        return new vscode.Hover(
            markdown,
            new vscode.Range(
                position.line,
                triggerMatch.startIndex,
                position.line,
                triggerMatch.endIndex
            )
        );
    }

    /**
     * Generate a preview of the template output
     */
    private async generatePreview(
        templateId: string,
        params: Record<string, string>
    ): Promise<string | null> {
        try {
            // Use placeholder values for missing required params
            const template = await this.registry.getTemplate(templateId);
            if (!template) {
                return null;
            }

            const previewParams = { ...params };
            for (const param of template.parameters) {
                if (!(param.name in previewParams)) {
                    if (param.default !== undefined) {
                        previewParams[param.name] = String(param.default);
                    } else {
                        // Use placeholder based on type
                        previewParams[param.name] = this.getPlaceholderValue(param);
                    }
                }
            }

            const result = await this.registry.generate(templateId, previewParams);
            return result.success ? result.content || null : null;
        } catch {
            return null;
        }
    }

    /**
     * Get a placeholder value for preview
     */
    private getPlaceholderValue(param: { name: string; valueType: string }): string {
        switch (param.valueType) {
            case 'PascalCase':
                return 'MyComponent';
            case 'camelCase':
                return 'myVariable';
            case 'snake_case':
                return 'my_variable';
            case 'kebab-case':
                return 'my-component';
            case 'UPPER_CASE':
                return 'MY_CONSTANT';
            case 'integer':
                return '42';
            case 'float':
                return '3.14';
            case 'boolean':
                return 'true';
            case 'date':
                return new Date().toISOString().split('T')[0];
            default:
                return `<${param.name}>`;
        }
    }

    /**
     * Truncate preview to max length
     */
    private truncatePreview(content: string, maxLength: number): string {
        if (content.length <= maxLength) {
            return content;
        }
        return content.substring(0, maxLength) + '\n... (truncated)';
    }

    /**
     * Detect language from output pattern
     */
    private detectLanguage(outputPattern: string): string {
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
}

/**
 * Register the hover provider
 */
export function registerGeneratorHoverProvider(
    context: vscode.ExtensionContext,
    registry: TemplateRegistry,
    triggerProvider: GeneratorTriggerProvider
): void {
    const hoverProvider = new GeneratorHoverProvider(registry, triggerProvider);

    // Register for all common file types
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
            vscode.languages.registerHoverProvider(
                { language },
                hoverProvider
            )
        );
    }
}
