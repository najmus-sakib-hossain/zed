/**
 * DX Driven Panel Provider
 * 
 * Main panel provider for the Driven accordion in VS Code explorer.
 * Requirements: 9.1, 9.2
 */

import * as vscode from 'vscode';
import { DrivenClient } from './drivenClient';
import {
    EditorSyncStatus,
    SpecMetadata,
    HookDefinition,
    SteeringFile,
    DrivenTemplate,
} from './types';

/**
 * Tree item types for the Driven panel
 */
type DrivenTreeItemType =
    | 'section'
    | 'rule'
    | 'spec'
    | 'hook'
    | 'steering'
    | 'template'
    | 'action';

/**
 * Tree item for the Driven panel
 */
export class DrivenTreeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly itemType: DrivenTreeItemType,
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
            case 'rule':
                const status = this.data as EditorSyncStatus;
                if (status?.status === 'synced') {
                    this.iconPath = new vscode.ThemeIcon('check', new vscode.ThemeColor('charts.green'));
                } else if (status?.status === 'error') {
                    this.iconPath = new vscode.ThemeIcon('error', new vscode.ThemeColor('charts.red'));
                } else if (status?.status === 'pending') {
                    this.iconPath = new vscode.ThemeIcon('sync~spin');
                } else {
                    this.iconPath = new vscode.ThemeIcon('circle-slash');
                }
                break;
            case 'spec':
                const spec = this.data as SpecMetadata;
                switch (spec?.status) {
                    case 'completed':
                        this.iconPath = new vscode.ThemeIcon('check-all', new vscode.ThemeColor('charts.green'));
                        break;
                    case 'in-progress':
                        this.iconPath = new vscode.ThemeIcon('play-circle');
                        break;
                    case 'tasks-ready':
                        this.iconPath = new vscode.ThemeIcon('tasklist');
                        break;
                    case 'planned':
                        this.iconPath = new vscode.ThemeIcon('note');
                        break;
                    case 'specified':
                        this.iconPath = new vscode.ThemeIcon('file-text');
                        break;
                    default:
                        this.iconPath = new vscode.ThemeIcon('file');
                }
                break;
            case 'hook':
                const hook = this.data as HookDefinition;
                if (hook?.enabled) {
                    this.iconPath = new vscode.ThemeIcon('zap', new vscode.ThemeColor('charts.yellow'));
                } else {
                    this.iconPath = new vscode.ThemeIcon('zap');
                }
                break;
            case 'steering':
                const steering = this.data as SteeringFile;
                switch (steering?.inclusionMode) {
                    case 'always':
                        this.iconPath = new vscode.ThemeIcon('pin');
                        break;
                    case 'fileMatch':
                        this.iconPath = new vscode.ThemeIcon('file-symlink-file');
                        break;
                    case 'manual':
                        this.iconPath = new vscode.ThemeIcon('tag');
                        break;
                    default:
                        this.iconPath = new vscode.ThemeIcon('file');
                }
                break;
            case 'template':
                const template = this.data as DrivenTemplate;
                switch (template?.category) {
                    case 'persona':
                        this.iconPath = new vscode.ThemeIcon('account');
                        break;
                    case 'project':
                        this.iconPath = new vscode.ThemeIcon('folder-library');
                        break;
                    case 'standard':
                        this.iconPath = new vscode.ThemeIcon('law');
                        break;
                    case 'workflow':
                        this.iconPath = new vscode.ThemeIcon('workflow');
                        break;
                    default:
                        this.iconPath = new vscode.ThemeIcon('file-code');
                }
                break;
            case 'action':
                this.iconPath = new vscode.ThemeIcon('play');
                break;
        }
    }

    private setTooltip(): void {
        switch (this.itemType) {
            case 'rule':
                const status = this.data as EditorSyncStatus;
                this.tooltip = `${status?.editor}: ${status?.status}`;
                if (status?.lastSync) {
                    this.tooltip += `\nLast sync: ${status.lastSync.toLocaleString()}`;
                }
                break;
            case 'spec':
                const spec = this.data as SpecMetadata;
                this.tooltip = `${spec?.name}\nStatus: ${spec?.status}\nPath: ${spec?.path}`;
                break;
            case 'hook':
                const hook = this.data as HookDefinition;
                this.tooltip = `${hook?.description || hook?.name}\nTrigger: ${hook?.triggerType}\nEnabled: ${hook?.enabled}`;
                break;
            case 'steering':
                const steering = this.data as SteeringFile;
                this.tooltip = `${steering?.description || steering?.name}\nMode: ${steering?.inclusionMode}`;
                if (steering?.fileMatchPattern) {
                    this.tooltip += `\nPattern: ${steering.fileMatchPattern}`;
                }
                break;
            case 'template':
                const template = this.data as DrivenTemplate;
                this.tooltip = `${template?.description}\nCategory: ${template?.category}`;
                break;
        }
    }
}

/**
 * Tree data provider for the Driven panel
 */
export class DrivenTreeDataProvider implements vscode.TreeDataProvider<DrivenTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<DrivenTreeItem | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    private client: DrivenClient;

    constructor() {
        this.client = new DrivenClient();
    }

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: DrivenTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: DrivenTreeItem): Promise<DrivenTreeItem[]> {
        if (!element) {
            // Root level - show sections
            return this.getRootItems();
        }

        // Section children
        switch (element.label) {
            case 'Rules':
                return this.getRulesItems();
            case 'Specs':
                return this.getSpecsItems();
            case 'Hooks':
                return this.getHooksItems();
            case 'Steering':
                return this.getSteeringItems();
            case 'Templates':
                return this.getTemplatesItems();
            default:
                return [];
        }
    }

    private getRootItems(): DrivenTreeItem[] {
        return [
            new DrivenTreeItem('Rules', 'section', vscode.TreeItemCollapsibleState.Expanded),
            new DrivenTreeItem('Specs', 'section', vscode.TreeItemCollapsibleState.Expanded),
            new DrivenTreeItem('Hooks', 'section', vscode.TreeItemCollapsibleState.Collapsed),
            new DrivenTreeItem('Steering', 'section', vscode.TreeItemCollapsibleState.Collapsed),
            new DrivenTreeItem('Templates', 'section', vscode.TreeItemCollapsibleState.Collapsed),
        ];
    }

    private async getRulesItems(): Promise<DrivenTreeItem[]> {
        const statuses = await this.client.getSyncStatus();
        return statuses.map(status => {
            const item = new DrivenTreeItem(
                status.editor,
                'rule',
                vscode.TreeItemCollapsibleState.None,
                status
            );
            item.description = status.status;
            return item;
        });
    }

    private async getSpecsItems(): Promise<DrivenTreeItem[]> {
        const specs = await this.client.getSpecs();
        if (specs.length === 0) {
            const item = new DrivenTreeItem(
                'No specs found',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.driven.specInit',
                title: 'Create Spec',
            };
            return [item];
        }
        return specs.map(spec => {
            const item = new DrivenTreeItem(
                `${spec.id} - ${spec.name}`,
                'spec',
                vscode.TreeItemCollapsibleState.None,
                spec
            );
            item.description = spec.status;
            item.command = {
                command: 'dx.driven.openSpec',
                title: 'Open Spec',
                arguments: [spec],
            };
            return item;
        });
    }

    private async getHooksItems(): Promise<DrivenTreeItem[]> {
        const hooks = await this.client.getHooks();
        if (hooks.length === 0) {
            const item = new DrivenTreeItem(
                'No hooks configured',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.driven.hooksCreate',
                title: 'Create Hook',
            };
            return [item];
        }
        return hooks.map(hook => {
            const item = new DrivenTreeItem(
                hook.name,
                'hook',
                vscode.TreeItemCollapsibleState.None,
                hook
            );
            item.description = hook.enabled ? 'enabled' : 'disabled';
            return item;
        });
    }

    private async getSteeringItems(): Promise<DrivenTreeItem[]> {
        const files = await this.client.getSteeringFiles();
        if (files.length === 0) {
            const item = new DrivenTreeItem(
                'No steering files',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.driven.steeringCreate',
                title: 'Create Steering File',
            };
            return [item];
        }
        return files.map(file => {
            const item = new DrivenTreeItem(
                file.name,
                'steering',
                vscode.TreeItemCollapsibleState.None,
                file
            );
            item.description = file.inclusionMode;
            item.command = {
                command: 'vscode.open',
                title: 'Open File',
                arguments: [vscode.Uri.file(file.path)],
            };
            return item;
        });
    }

    private async getTemplatesItems(): Promise<DrivenTreeItem[]> {
        const templates = await this.client.getTemplates();
        return templates.map(template => {
            const item = new DrivenTreeItem(
                template.name,
                'template',
                vscode.TreeItemCollapsibleState.None,
                template
            );
            item.description = template.category;
            item.command = {
                command: 'dx.driven.templateApply',
                title: 'Apply Template',
                arguments: [template],
            };
            return item;
        });
    }

    /**
     * Get the client for external use
     */
    getClient(): DrivenClient {
        return this.client;
    }
}
