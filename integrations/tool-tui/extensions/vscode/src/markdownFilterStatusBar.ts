/**
 * Markdown Filter Status Bar
 * 
 * Provides UI for toggling red list filters on .md files:
 * - Preset filters (Minimal, CodeOnly, DocsOnly, ApiOnly)
 * - Individual element filters (images, links, code blocks, etc.)
 * - Section filters (license, contributing, examples, etc.)
 * 
 * Filters can be applied to single file or all workspace files.
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

export interface RedListConfig {
    elements: {
        remove_images: boolean;
        remove_links: boolean;
        remove_horizontal_rules: boolean;
        remove_blockquotes: boolean;
        remove_code_blocks: boolean;
        remove_inline_code: boolean;
        remove_emphasis: boolean;
        remove_strikethrough: boolean;
        remove_task_lists: boolean;
        remove_footnotes: boolean;
        remove_emojis: boolean;
        remove_html: boolean;
        remove_math: boolean;
        remove_mermaid: boolean;
    };
    sections: {
        remove_sections: string[];
        remove_badges: boolean;
        remove_table_of_contents: boolean;
        remove_license: boolean;
        remove_contributing: boolean;
        remove_changelog: boolean;
        remove_acknowledgments: boolean;
        remove_faq: boolean;
        remove_examples: boolean;
        remove_troubleshooting: boolean;
        remove_installation: boolean;
        remove_previous_updates: boolean;
        remove_social_links: boolean;
        remove_footnotes_section: boolean;
        remove_alerts: boolean;
        remove_collapsible: boolean;
        remove_emoji_section: boolean;
        remove_math_section: boolean;
        remove_mermaid_section: boolean;
        remove_ascii_art: boolean;
        remove_html_section: boolean;
        remove_yaml_front_matter: boolean;
        remove_mentions: boolean;
        remove_geojson: boolean;
    };
    preset?: 'Minimal' | 'CodeOnly' | 'DocsOnly' | 'ApiOnly' | null;
}

export class MarkdownFilterStatusBar implements vscode.Disposable {
    private presetStatusBar: vscode.StatusBarItem;
    private filtersStatusBar: vscode.StatusBarItem;
    private disposables: vscode.Disposable[] = [];
    private currentConfig: RedListConfig;
    private applyToAllFiles: boolean = false;
    private wasmMarkdown: any = null;

    constructor(wasmMarkdown: any) {
        this.wasmMarkdown = wasmMarkdown;
        
        // Initialize with default config (no filters)
        this.currentConfig = this.getDefaultConfig();
        
        // Create preset status bar (left side, higher priority)
        this.presetStatusBar = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            200
        );
        this.presetStatusBar.command = 'dx.markdown.selectPreset';
        this.presetStatusBar.text = '$(filter) Preset: None';
        this.presetStatusBar.tooltip = 'Select markdown filter preset';
        
        // Create filters status bar (left side, lower priority)
        this.filtersStatusBar = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            199
        );
        this.filtersStatusBar.command = 'dx.markdown.toggleFilters';
        this.filtersStatusBar.text = '$(checklist) Filters';
        this.filtersStatusBar.tooltip = 'Toggle individual markdown filters';
        
        // Listen for active editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor(() => this.updateVisibility())
        );
        
        this.updateVisibility();
    }

    private getDefaultConfig(): RedListConfig {
        return {
            elements: {
                remove_images: false,
                remove_links: false,
                remove_horizontal_rules: false,
                remove_blockquotes: false,
                remove_code_blocks: false,
                remove_inline_code: false,
                remove_emphasis: false,
                remove_strikethrough: false,
                remove_task_lists: false,
                remove_footnotes: false,
                remove_emojis: false,
                remove_html: false,
                remove_math: false,
                remove_mermaid: false,
            },
            sections: {
                remove_sections: [],
                remove_badges: false,
                remove_table_of_contents: false,
                remove_license: false,
                remove_contributing: false,
                remove_changelog: false,
                remove_acknowledgments: false,
                remove_faq: false,
                remove_examples: false,
                remove_troubleshooting: false,
                remove_installation: false,
                remove_previous_updates: false,
                remove_social_links: false,
                remove_footnotes_section: false,
                remove_alerts: false,
                remove_collapsible: false,
                remove_emoji_section: false,
                remove_math_section: false,
                remove_mermaid_section: false,
                remove_ascii_art: false,
                remove_html_section: false,
                remove_yaml_front_matter: false,
                remove_mentions: false,
                remove_geojson: false,
            },
            preset: null,
        };
    }

    private updateVisibility(): void {
        const editor = vscode.window.activeTextEditor;
        if (editor && editor.document.uri.fsPath.endsWith('.md')) {
            this.presetStatusBar.show();
            this.filtersStatusBar.show();
        } else {
            this.presetStatusBar.hide();
            this.filtersStatusBar.hide();
        }
    }

    async selectPreset(): Promise<void> {
        const presets = [
            { label: '$(circle-slash) None', value: null, description: 'No filters applied' },
            { label: '$(dash) Minimal', value: 'Minimal', description: 'Remove everything possible (85% savings)' },
            { label: '$(code) Code Only', value: 'CodeOnly', description: 'Keep code, remove prose (60% savings)' },
            { label: '$(book) Docs Only', value: 'DocsOnly', description: 'Keep docs, remove code (40% savings)' },
            { label: '$(symbol-method) API Only', value: 'ApiOnly', description: 'API reference only (50% savings)' },
        ];

        const selected = await vscode.window.showQuickPick(presets, {
            placeHolder: 'Select a filter preset',
        });

        if (selected === undefined) return;

        if (selected.value === null) {
            this.currentConfig = this.getDefaultConfig();
            this.presetStatusBar.text = '$(filter) Preset: None';
        } else {
            this.currentConfig = this.getPresetConfig(selected.value);
            this.presetStatusBar.text = `$(filter) Preset: ${selected.value}`;
        }

        await this.applyFilters();
    }

    async toggleFilters(): Promise<void> {
        const items: vscode.QuickPickItem[] = [
            { label: '$(globe) Apply to All Workspace Files', description: this.applyToAllFiles ? '✓ Enabled' : 'Disabled', picked: this.applyToAllFiles },
            { label: '', kind: vscode.QuickPickItemKind.Separator },
            { label: 'Element Filters', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(file-media) Images', description: 'Remove ![alt](url)', picked: this.currentConfig.elements.remove_images },
            { label: '$(link) Links', description: 'Remove [text](url)', picked: this.currentConfig.elements.remove_links },
            { label: '$(code) Code Blocks', description: 'Remove ```code```', picked: this.currentConfig.elements.remove_code_blocks },
            { label: '$(symbol-string) Inline Code', description: 'Remove `code`', picked: this.currentConfig.elements.remove_inline_code },
            { label: '$(bold) Emphasis', description: 'Remove **bold** and *italic*', picked: this.currentConfig.elements.remove_emphasis },
            { label: '$(quote) Blockquotes', description: 'Remove > quotes', picked: this.currentConfig.elements.remove_blockquotes },
            { label: '$(dash) Horizontal Rules', description: 'Remove ---', picked: this.currentConfig.elements.remove_horizontal_rules },
            { label: '$(tasklist) Task Lists', description: 'Remove - [x] tasks', picked: this.currentConfig.elements.remove_task_lists },
            { label: '$(note) Footnotes', description: 'Remove [^1]', picked: this.currentConfig.elements.remove_footnotes },
            { label: '$(smiley) Emojis', description: 'Remove :smile: and 😀', picked: this.currentConfig.elements.remove_emojis },
            { label: '$(code) HTML', description: 'Remove <tags>', picked: this.currentConfig.elements.remove_html },
            { label: '$(symbol-operator) Math', description: 'Remove $E=mc^2$', picked: this.currentConfig.elements.remove_math },
            { label: '$(graph) Mermaid', description: 'Remove diagrams', picked: this.currentConfig.elements.remove_mermaid },
            { label: '', kind: vscode.QuickPickItemKind.Separator },
            { label: 'Section Filters', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(tag) Badges', picked: this.currentConfig.sections.remove_badges },
            { label: '$(list-tree) Table of Contents', picked: this.currentConfig.sections.remove_table_of_contents },
            { label: '$(law) License', picked: this.currentConfig.sections.remove_license },
            { label: '$(git-pull-request) Contributing', picked: this.currentConfig.sections.remove_contributing },
            { label: '$(history) Changelog', picked: this.currentConfig.sections.remove_changelog },
            { label: '$(heart) Acknowledgments', picked: this.currentConfig.sections.remove_acknowledgments },
            { label: '$(question) FAQ', picked: this.currentConfig.sections.remove_faq },
            { label: '$(beaker) Examples', picked: this.currentConfig.sections.remove_examples },
            { label: '$(tools) Troubleshooting', picked: this.currentConfig.sections.remove_troubleshooting },
            { label: '$(desktop-download) Installation', picked: this.currentConfig.sections.remove_installation },
        ];

        const selected = await vscode.window.showQuickPick(items, {
            placeHolder: 'Toggle filters (Space to select, Enter to apply)',
            canPickMany: true,
        });

        if (!selected) return;

        // Update config based on selections
        this.applyToAllFiles = selected.some(item => item.label.includes('Apply to All'));
        this.currentConfig.elements.remove_images = selected.some(item => item.label.includes('Images'));
        this.currentConfig.elements.remove_links = selected.some(item => item.label.includes('Links'));
        this.currentConfig.elements.remove_code_blocks = selected.some(item => item.label.includes('Code Blocks'));
        this.currentConfig.elements.remove_inline_code = selected.some(item => item.label.includes('Inline Code'));
        this.currentConfig.elements.remove_emphasis = selected.some(item => item.label.includes('Emphasis'));
        this.currentConfig.elements.remove_blockquotes = selected.some(item => item.label.includes('Blockquotes'));
        this.currentConfig.elements.remove_horizontal_rules = selected.some(item => item.label.includes('Horizontal Rules'));
        this.currentConfig.elements.remove_task_lists = selected.some(item => item.label.includes('Task Lists'));
        this.currentConfig.elements.remove_footnotes = selected.some(item => item.label.includes('Footnotes'));
        this.currentConfig.elements.remove_emojis = selected.some(item => item.label.includes('Emojis'));
        this.currentConfig.elements.remove_html = selected.some(item => item.label.includes('HTML'));
        this.currentConfig.elements.remove_math = selected.some(item => item.label.includes('Math'));
        this.currentConfig.elements.remove_mermaid = selected.some(item => item.label.includes('Mermaid'));
        
        this.currentConfig.sections.remove_badges = selected.some(item => item.label.includes('Badges'));
        this.currentConfig.sections.remove_table_of_contents = selected.some(item => item.label.includes('Table of Contents'));
        this.currentConfig.sections.remove_license = selected.some(item => item.label.includes('License'));
        this.currentConfig.sections.remove_contributing = selected.some(item => item.label.includes('Contributing'));
        this.currentConfig.sections.remove_changelog = selected.some(item => item.label.includes('Changelog'));
        this.currentConfig.sections.remove_acknowledgments = selected.some(item => item.label.includes('Acknowledgments'));
        this.currentConfig.sections.remove_faq = selected.some(item => item.label.includes('FAQ'));
        this.currentConfig.sections.remove_examples = selected.some(item => item.label.includes('Examples'));
        this.currentConfig.sections.remove_troubleshooting = selected.some(item => item.label.includes('Troubleshooting'));
        this.currentConfig.sections.remove_installation = selected.some(item => item.label.includes('Installation'));

        // Clear preset if custom filters applied
        this.currentConfig.preset = null;

        await this.applyFilters();
    }

    private async applyFilters(): Promise<void> {
        if (this.applyToAllFiles) {
            await this.applyToAllWorkspaceFiles();
        } else {
            await this.applyToCurrentFile();
        }
    }

    private async applyToCurrentFile(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor || !editor.document.uri.fsPath.endsWith('.md')) {
            vscode.window.showWarningMessage('No markdown file open');
            return;
        }

        await this.regenerateLlmFile(editor.document.uri.fsPath);
        vscode.window.showInformationMessage('Filters applied to current file');
    }

    private async applyToAllWorkspaceFiles(): Promise<void> {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders) {
            vscode.window.showWarningMessage('No workspace folder open');
            return;
        }

        const mdFiles = await vscode.workspace.findFiles('**/*.md', '**/node_modules/**');
        
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: 'Applying filters to all markdown files',
            cancellable: false,
        }, async (progress) => {
            for (let i = 0; i < mdFiles.length; i++) {
                progress.report({ 
                    increment: (100 / mdFiles.length),
                    message: `${i + 1}/${mdFiles.length} files`
                });
                await this.regenerateLlmFile(mdFiles[i].fsPath);
            }
        });

        vscode.window.showInformationMessage(`Filters applied to ${mdFiles.length} markdown files`);
    }

    private async regenerateLlmFile(mdPath: string): Promise<void> {
        try {
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders) return;

            const workspaceRoot = workspaceFolders[0].uri.fsPath;
            const relativePath = path.relative(workspaceRoot, mdPath);
            const relativeDir = path.dirname(relativePath);
            const baseName = path.basename(mdPath, '.md');

            // Read .human file (source of truth)
            const humanPath = path.join(workspaceRoot, '.dx', 'markdown', relativeDir, `${baseName}.human`);
            
            if (!fs.existsSync(humanPath)) {
                console.warn(`Human file not found: ${humanPath}`);
                return;
            }

            const humanContent = await fs.promises.readFile(humanPath, 'utf-8');

            // Apply red list filters to human markdown content
            let filteredContent = humanContent;
            if (this.wasmMarkdown && this.wasmMarkdown.apply_red_list_filters) {
                const configJson = JSON.stringify(this.currentConfig);
                filteredContent = this.wasmMarkdown.apply_red_list_filters(humanContent, configJson);
            }

            // Convert filtered human → LLM format (if conversion is needed)
            let llmContent = filteredContent;
            if (this.wasmMarkdown && this.wasmMarkdown.human_to_llm) {
                llmContent = this.wasmMarkdown.human_to_llm(filteredContent);
            }

            // Write to .md file (LLM format on disk)
            await fs.promises.writeFile(mdPath, llmContent, 'utf-8');

        } catch (error) {
            console.error('Failed to regenerate LLM file:', error);
        }
    }

    private getPresetConfig(preset: string): RedListConfig {
        const config = this.getDefaultConfig();
        config.preset = preset as any;

        switch (preset) {
            case 'Minimal':
                return {
                    ...config,
                    elements: {
                        remove_images: true,
                        remove_links: false,
                        remove_horizontal_rules: true,
                        remove_blockquotes: true,
                        remove_code_blocks: false,
                        remove_inline_code: false,
                        remove_emphasis: true,
                        remove_strikethrough: true,
                        remove_task_lists: true,
                        remove_footnotes: true,
                        remove_emojis: true,
                        remove_html: true,
                        remove_math: true,
                        remove_mermaid: true,
                    },
                    sections: {
                        ...config.sections,
                        remove_badges: true,
                        remove_table_of_contents: true,
                        remove_license: true,
                        remove_contributing: true,
                        remove_changelog: true,
                        remove_acknowledgments: true,
                        remove_faq: true,
                        remove_examples: true,
                        remove_troubleshooting: true,
                        remove_installation: true,
                    },
                };

            case 'CodeOnly':
                return {
                    ...config,
                    elements: {
                        remove_images: true,
                        remove_links: false,
                        remove_horizontal_rules: true,
                        remove_blockquotes: true,
                        remove_code_blocks: false,
                        remove_inline_code: false,
                        remove_emphasis: true,
                        remove_strikethrough: true,
                        remove_task_lists: true,
                        remove_footnotes: true,
                        remove_emojis: true,
                        remove_html: true,
                        remove_math: false,
                        remove_mermaid: false,
                    },
                    sections: {
                        ...config.sections,
                        remove_badges: true,
                        remove_license: true,
                        remove_contributing: true,
                        remove_examples: false,
                    },
                };

            case 'DocsOnly':
                return {
                    ...config,
                    elements: {
                        remove_images: true,
                        remove_links: false,
                        remove_horizontal_rules: true,
                        remove_blockquotes: false,
                        remove_code_blocks: true,
                        remove_inline_code: true,
                        remove_emphasis: false,
                        remove_strikethrough: true,
                        remove_task_lists: true,
                        remove_footnotes: false,
                        remove_emojis: true,
                        remove_html: true,
                        remove_math: true,
                        remove_mermaid: true,
                    },
                    sections: {
                        ...config.sections,
                        remove_badges: true,
                        remove_examples: true,
                    },
                };

            case 'ApiOnly':
                return {
                    ...config,
                    elements: {
                        remove_images: true,
                        remove_links: false,
                        remove_horizontal_rules: true,
                        remove_blockquotes: true,
                        remove_code_blocks: false,
                        remove_inline_code: false,
                        remove_emphasis: false,
                        remove_strikethrough: true,
                        remove_task_lists: true,
                        remove_footnotes: true,
                        remove_emojis: true,
                        remove_html: true,
                        remove_math: false,
                        remove_mermaid: true,
                    },
                    sections: {
                        ...config.sections,
                        remove_badges: true,
                        remove_license: true,
                        remove_examples: true,
                    },
                };

            default:
                return config;
        }
    }

    show(): void {
        this.presetStatusBar.show();
        this.filtersStatusBar.show();
    }

    hide(): void {
        this.presetStatusBar.hide();
        this.filtersStatusBar.hide();
    }

    dispose(): void {
        this.presetStatusBar.dispose();
        this.filtersStatusBar.dispose();
        this.disposables.forEach(d => d.dispose());
    }
}

export function registerMarkdownFilterCommands(
    context: vscode.ExtensionContext,
    filterStatusBar: MarkdownFilterStatusBar
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.markdown.selectPreset', async () => {
            await filterStatusBar.selectPreset();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.markdown.toggleFilters', async () => {
            await filterStatusBar.toggleFilters();
        })
    );
}
