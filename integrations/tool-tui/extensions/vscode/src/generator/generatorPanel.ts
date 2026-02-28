/**
 * DX Generator Panel Provider
 * 
 * Main panel provider for the Generator accordion in VS Code explorer.
 * Requirements: 10.1, 10.2
 */

import * as vscode from 'vscode';
import { TemplateRegistry } from './templateRegistry';
import { TemplateMetadata, TriggerDefinition, TokenSavings } from './types';
import { GeneratorStatusBar } from './statusBar';

/**
 * Tree item types for the Generator panel
 */
type GeneratorTreeItemType =
    | 'section'
    | 'template'
    | 'trigger'
    | 'stat'
    | 'action';

/**
 * Trigger configuration from settings
 */
interface TriggerConfig {
    pattern: string;
    templateId: string;
}

/**
 * Tree item for the Generator panel
 */
export class GeneratorTreeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly itemType: GeneratorTreeItemType,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly data?: any,
    ) {
        super(label, collapsibleState);
        this.contextValue = itemType;
        this.setIcon();
        this.setTooltip();
    }

    private setIcon(): void {
        switch (this.itemType) {
            case 'section':
                this.iconPath = new vscode.ThemeIcon('folder');
                break;
            case 'template':
                const template = this.data as TemplateMetadata;
                if (template?.tags?.includes('component')) {
                    this.iconPath = new vscode.ThemeIcon('symbol-class');
                } else if (template?.tags?.includes('model')) {
                    this.iconPath = new vscode.ThemeIcon('symbol-interface');
                } else if (template?.tags?.includes('test')) {
                    this.iconPath = new vscode.ThemeIcon('beaker');
                } else {
                    this.iconPath = new vscode.ThemeIcon('file-code');
                }
                break;
            case 'trigger':
                this.iconPath = new vscode.ThemeIcon('zap');
                break;
            case 'stat':
                this.iconPath = new vscode.ThemeIcon('graph');
                break;
            case 'action':
                this.iconPath = new vscode.ThemeIcon('play');
                break;
        }
    }

    private setTooltip(): void {
        switch (this.itemType) {
            case 'template':
                const template = this.data as TemplateMetadata;
                this.tooltip = `${template?.description || template?.name}\nVersion: ${template?.version}\nTags: ${template?.tags?.join(', ') || 'none'}`;
                break;
            case 'trigger':
                const trigger = this.data as TriggerConfig;
                this.tooltip = `Pattern: ${trigger?.pattern}\nTemplate: ${trigger?.templateId}`;
                break;
            case 'stat':
                this.tooltip = this.description?.toString() || '';
                break;
        }
    }
}

/**
 * Tree data provider for the Generator panel
 */
export class GeneratorTreeDataProvider implements vscode.TreeDataProvider<GeneratorTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<GeneratorTreeItem | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    private registry: TemplateRegistry;
    private statusBar: GeneratorStatusBar;

    constructor(registry: TemplateRegistry, statusBar: GeneratorStatusBar) {
        this.registry = registry;
        this.statusBar = statusBar;
    }

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: GeneratorTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: GeneratorTreeItem): Promise<GeneratorTreeItem[]> {
        if (!element) {
            // Root level - show sections
            return this.getRootItems();
        }

        // Section children
        switch (element.label) {
            case 'Templates':
                return this.getTemplatesItems();
            case 'Triggers':
                return this.getTriggersItems();
            case 'Stats':
                return this.getStatsItems();
            default:
                return [];
        }
    }

    private getRootItems(): GeneratorTreeItem[] {
        return [
            new GeneratorTreeItem('Templates', 'section', vscode.TreeItemCollapsibleState.Expanded),
            new GeneratorTreeItem('Triggers', 'section', vscode.TreeItemCollapsibleState.Collapsed),
            new GeneratorTreeItem('Stats', 'section', vscode.TreeItemCollapsibleState.Collapsed),
        ];
    }

    private async getTemplatesItems(): Promise<GeneratorTreeItem[]> {
        const templates = await this.registry.listTemplates();

        if (templates.length === 0) {
            const item = new GeneratorTreeItem(
                'No templates found',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.generator.refreshTemplates',
                title: 'Refresh Templates',
            };
            return [item];
        }

        return templates.map(template => {
            const item = new GeneratorTreeItem(
                template.name,
                'template',
                vscode.TreeItemCollapsibleState.None,
                template
            );
            item.description = template.version;
            item.command = {
                command: 'dx.generator.generateById',
                title: 'Generate',
                arguments: [template.id],
            };
            return item;
        });
    }

    private getTriggersItems(): GeneratorTreeItem[] {
        const config = vscode.workspace.getConfiguration('dx.generator');
        const triggers = config.get<TriggerConfig[]>('triggerPatterns', []);
        const enabled = config.get<boolean>('enableTriggers', true);

        if (!enabled) {
            const item = new GeneratorTreeItem(
                'Triggers disabled',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.description = 'Click to enable';
            item.command = {
                command: 'dx.generator.enableTriggers',
                title: 'Enable Triggers',
            };
            return [item];
        }

        if (triggers.length === 0) {
            const item = new GeneratorTreeItem(
                'No triggers configured',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.generator.configureTriggers',
                title: 'Configure Triggers',
            };
            return [item];
        }

        return triggers.map(trigger => {
            const item = new GeneratorTreeItem(
                trigger.pattern,
                'trigger',
                vscode.TreeItemCollapsibleState.None,
                trigger
            );
            item.description = `→ ${trigger.templateId}`;
            return item;
        });
    }

    private getStatsItems(): GeneratorTreeItem[] {
        const stats = this.statusBar.getStats();

        return [
            this.createStatItem('Session Tokens', this.formatNumber(stats.sessionTokens)),
            this.createStatItem('Total Tokens', this.formatNumber(stats.totalTokens)),
            this.createStatItem('Generations', stats.generationCount.toString()),
        ];
    }

    private createStatItem(label: string, value: string): GeneratorTreeItem {
        const item = new GeneratorTreeItem(
            label,
            'stat',
            vscode.TreeItemCollapsibleState.None
        );
        item.description = value;
        return item;
    }

    private formatNumber(num: number): string {
        if (num >= 1000000) {
            return (num / 1000000).toFixed(1) + 'M';
        }
        if (num >= 1000) {
            return (num / 1000).toFixed(1) + 'K';
        }
        return num.toString();
    }

    /**
     * Get the registry for external use
     */
    getRegistry(): TemplateRegistry {
        return this.registry;
    }
}

/**
 * Register additional Generator panel commands
 */
export function registerGeneratorPanelCommands(
    context: vscode.ExtensionContext,
    treeDataProvider: GeneratorTreeDataProvider
): void {
    // Refresh panel
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.refreshPanel', () => {
            treeDataProvider.refresh();
        })
    );

    // Enable triggers
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.enableTriggers', async () => {
            const config = vscode.workspace.getConfiguration('dx.generator');
            await config.update('enableTriggers', true, vscode.ConfigurationTarget.Workspace);
            treeDataProvider.refresh();
            vscode.window.showInformationMessage('Generator triggers enabled');
        })
    );

    // Configure triggers
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.configureTriggers', async () => {
            await vscode.commands.executeCommand(
                'workbench.action.openSettings',
                'dx.generator.triggerPatterns'
            );
        })
    );

    // Template drag-and-drop support (placeholder for future implementation)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.generator.applyToFolder', async (template: TemplateMetadata, folder: vscode.Uri) => {
            vscode.window.showInformationMessage(
                `Would apply template "${template.name}" to ${folder.fsPath}`
            );
        })
    );
}
