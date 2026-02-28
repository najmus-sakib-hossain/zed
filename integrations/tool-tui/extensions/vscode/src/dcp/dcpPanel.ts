/**
 * DX DCP Panel Provider
 * 
 * Main panel provider for the DCP accordion in VS Code explorer.
 * Requirements: 11.1, 11.2
 */

import * as vscode from 'vscode';
import { DcpClient } from './dcpClient';
import {
    DcpServerStatus,
    DcpTool,
    DcpResource,
    DcpMetrics,
} from './types';

/**
 * Tree item types for the DCP panel
 */
type DcpTreeItemType =
    | 'section'
    | 'server'
    | 'tool'
    | 'resource'
    | 'metric'
    | 'action';

/**
 * Tree item for the DCP panel
 */
export class DcpTreeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly itemType: DcpTreeItemType,
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
            case 'server':
                const server = this.data as DcpServerStatus;
                if (server?.running) {
                    this.iconPath = new vscode.ThemeIcon('vm-running', new vscode.ThemeColor('charts.green'));
                } else {
                    this.iconPath = new vscode.ThemeIcon('vm-outline');
                }
                break;
            case 'tool':
                const tool = this.data as DcpTool;
                if (tool?.signed) {
                    this.iconPath = new vscode.ThemeIcon('verified', new vscode.ThemeColor('charts.blue'));
                } else {
                    this.iconPath = new vscode.ThemeIcon('tools');
                }
                break;
            case 'resource':
                const resource = this.data as DcpResource;
                switch (resource?.access) {
                    case 'admin':
                        this.iconPath = new vscode.ThemeIcon('shield');
                        break;
                    case 'execute':
                        this.iconPath = new vscode.ThemeIcon('play');
                        break;
                    case 'write':
                        this.iconPath = new vscode.ThemeIcon('edit');
                        break;
                    default:
                        this.iconPath = new vscode.ThemeIcon('file');
                }
                break;
            case 'metric':
                this.iconPath = new vscode.ThemeIcon('pulse');
                break;
            case 'action':
                this.iconPath = new vscode.ThemeIcon('play');
                break;
        }
    }

    private setTooltip(): void {
        switch (this.itemType) {
            case 'server':
                const server = this.data as DcpServerStatus;
                this.tooltip = `Port: ${server?.port}\nMode: ${server?.mode}\nStatus: ${server?.running ? 'Running' : 'Stopped'}`;
                if (server?.uptime) {
                    this.tooltip += `\nUptime: ${Math.floor(server.uptime / 60)}m`;
                }
                break;
            case 'tool':
                const tool = this.data as DcpTool;
                this.tooltip = `${tool?.description}\nSigned: ${tool?.signed ? 'Yes' : 'No'}`;
                if (tool?.version) {
                    this.tooltip += `\nVersion: ${tool.version}`;
                }
                break;
            case 'resource':
                const resource = this.data as DcpResource;
                this.tooltip = `${resource?.description || resource?.name}\nURI: ${resource?.uri}\nAccess: ${resource?.access}`;
                break;
        }
    }
}

/**
 * Tree data provider for the DCP panel
 */
export class DcpTreeDataProvider implements vscode.TreeDataProvider<DcpTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<DcpTreeItem | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    private client: DcpClient;
    private metrics: DcpMetrics | null = null;

    constructor() {
        this.client = new DcpClient();
    }

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: DcpTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: DcpTreeItem): Promise<DcpTreeItem[]> {
        if (!element) {
            // Root level - show sections
            return this.getRootItems();
        }

        // Section children
        switch (element.label) {
            case 'Servers':
                return this.getServersItems();
            case 'Tools':
                return this.getToolsItems();
            case 'Resources':
                return this.getResourcesItems();
            case 'Metrics':
                return this.getMetricsItems();
            default:
                return [];
        }
    }

    private getRootItems(): DcpTreeItem[] {
        return [
            new DcpTreeItem('Servers', 'section', vscode.TreeItemCollapsibleState.Expanded),
            new DcpTreeItem('Tools', 'section', vscode.TreeItemCollapsibleState.Expanded),
            new DcpTreeItem('Resources', 'section', vscode.TreeItemCollapsibleState.Collapsed),
            new DcpTreeItem('Metrics', 'section', vscode.TreeItemCollapsibleState.Collapsed),
        ];
    }

    private async getServersItems(): Promise<DcpTreeItem[]> {
        const servers = await this.client.getServerStatus();

        if (servers.length === 0) {
            const item = new DcpTreeItem(
                'No servers configured',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.dcp.startServer',
                title: 'Start Server',
            };
            return [item];
        }

        return servers.map(server => {
            const item = new DcpTreeItem(
                `${server.name} (:${server.port})`,
                'server',
                vscode.TreeItemCollapsibleState.None,
                server
            );
            item.description = server.running ? 'running' : 'stopped';
            return item;
        });
    }

    private async getToolsItems(): Promise<DcpTreeItem[]> {
        const tools = await this.client.getTools();

        if (tools.length === 0) {
            const item = new DcpTreeItem(
                'No tools registered',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            item.command = {
                command: 'dx.dcp.registerTool',
                title: 'Register Tool',
            };
            return [item];
        }

        return tools.map(tool => {
            const item = new DcpTreeItem(
                tool.name,
                'tool',
                vscode.TreeItemCollapsibleState.None,
                tool
            );
            item.description = tool.signed ? '✓ signed' : '';
            item.command = {
                command: 'dx.dcp.showToolSchema',
                title: 'Show Schema',
                arguments: [tool],
            };
            return item;
        });
    }

    private async getResourcesItems(): Promise<DcpTreeItem[]> {
        const resources = await this.client.getResources();

        if (resources.length === 0) {
            const item = new DcpTreeItem(
                'No resources available',
                'action',
                vscode.TreeItemCollapsibleState.None
            );
            return [item];
        }

        return resources.map(resource => {
            const item = new DcpTreeItem(
                resource.name,
                'resource',
                vscode.TreeItemCollapsibleState.None,
                resource
            );
            item.description = resource.access;
            return item;
        });
    }

    private async getMetricsItems(): Promise<DcpTreeItem[]> {
        const metrics = await this.client.getMetrics();
        this.metrics = metrics;

        return [
            this.createMetricItem('Avg Latency', `${metrics.avgLatencyUs}μs`),
            this.createMetricItem('P99 Latency', `${metrics.p99LatencyUs}μs`),
            this.createMetricItem('Throughput', `${metrics.messagesPerSecond}/s`),
            this.createMetricItem('Avg Message Size', `${metrics.avgMessageSize}B`),
            this.createMetricItem('Total Messages', metrics.totalMessages.toString()),
            this.createMetricItem('Errors', metrics.errorCount.toString()),
        ];
    }

    private createMetricItem(label: string, value: string): DcpTreeItem {
        const item = new DcpTreeItem(
            label,
            'metric',
            vscode.TreeItemCollapsibleState.None
        );
        item.description = value;
        return item;
    }

    /**
     * Get the client for external use
     */
    getClient(): DcpClient {
        return this.client;
    }
}
