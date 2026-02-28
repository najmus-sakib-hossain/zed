/**
 * DX-WWW VS Code Extension Panel
 * 
 * Provides a tree view panel showing:
 * - Project structure
 * - Routes and components
 * - API endpoints
 * 
 * Requirements: 8.4
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Tree item types for the DX-WWW panel
 */
type WwwTreeItemType = 'root' | 'section' | 'route' | 'component' | 'api' | 'content' | 'layout';

/**
 * Tree item for the DX-WWW panel
 */
export class WwwTreeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly itemType: WwwTreeItemType,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly filePath?: string,
        public readonly routePath?: string,
    ) {
        super(label, collapsibleState);

        this.tooltip = this.getTooltip();
        this.iconPath = this.getIcon();
        this.contextValue = itemType;

        if (filePath) {
            this.command = {
                command: 'vscode.open',
                title: 'Open File',
                arguments: [vscode.Uri.file(filePath)],
            };
        }
    }

    private getTooltip(): string {
        switch (this.itemType) {
            case 'route':
                return `Route: ${this.routePath || this.label}`;
            case 'component':
                return `Component: ${this.label}`;
            case 'api':
                return `API: /api/${this.label}`;
            case 'content':
                return `Content: ${this.label}`;
            case 'layout':
                return `Layout: ${this.label}`;
            default:
                return this.label;
        }
    }

    private getIcon(): vscode.ThemeIcon {
        switch (this.itemType) {
            case 'root':
                return new vscode.ThemeIcon('globe');
            case 'section':
                return new vscode.ThemeIcon('folder');
            case 'route':
                return new vscode.ThemeIcon('file-code');
            case 'component':
                return new vscode.ThemeIcon('symbol-class');
            case 'api':
                return new vscode.ThemeIcon('plug');
            case 'content':
                return new vscode.ThemeIcon('file-text');
            case 'layout':
                return new vscode.ThemeIcon('layout');
            default:
                return new vscode.ThemeIcon('file');
        }
    }
}

/**
 * Tree data provider for the DX-WWW panel
 */
export class WwwTreeDataProvider implements vscode.TreeDataProvider<WwwTreeItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<WwwTreeItem | undefined | null | void> = new vscode.EventEmitter<WwwTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<WwwTreeItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private workspaceRoot: string | undefined;

    constructor() {
        this.workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: WwwTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: WwwTreeItem): Promise<WwwTreeItem[]> {
        if (!this.workspaceRoot) {
            return [];
        }

        if (!element) {
            // Root level - show sections
            return this.getRootItems();
        }

        // Get children based on section type
        switch (element.itemType) {
            case 'section':
                return this.getSectionChildren(element.label);
            default:
                return [];
        }
    }

    private async getRootItems(): Promise<WwwTreeItem[]> {
        const items: WwwTreeItem[] = [];

        // Check if this is a dx-www project
        if (!await this.isDxWwwProject()) {
            return [
                new WwwTreeItem(
                    'Not a DX-WWW project',
                    'root',
                    vscode.TreeItemCollapsibleState.None
                ),
            ];
        }

        // Add sections
        items.push(new WwwTreeItem(
            'Routes',
            'section',
            vscode.TreeItemCollapsibleState.Expanded
        ));

        items.push(new WwwTreeItem(
            'Components',
            'section',
            vscode.TreeItemCollapsibleState.Expanded
        ));

        items.push(new WwwTreeItem(
            'API',
            'section',
            vscode.TreeItemCollapsibleState.Expanded
        ));

        items.push(new WwwTreeItem(
            'Content',
            'section',
            vscode.TreeItemCollapsibleState.Collapsed
        ));

        items.push(new WwwTreeItem(
            'Layouts',
            'section',
            vscode.TreeItemCollapsibleState.Collapsed
        ));

        return items;
    }

    private async getSectionChildren(section: string): Promise<WwwTreeItem[]> {
        if (!this.workspaceRoot) {
            return [];
        }

        switch (section) {
            case 'Routes':
                return this.getRoutes();
            case 'Components':
                return this.getComponents();
            case 'API':
                return this.getApiRoutes();
            case 'Content':
                return this.getContent();
            case 'Layouts':
                return this.getLayouts();
            default:
                return [];
        }
    }

    private async getRoutes(): Promise<WwwTreeItem[]> {
        const pagesDir = path.join(this.workspaceRoot!, 'pages');
        return this.scanDirectory(pagesDir, 'route', this.fileToRoute.bind(this));
    }

    private async getComponents(): Promise<WwwTreeItem[]> {
        const componentsDir = path.join(this.workspaceRoot!, 'components');
        return this.scanDirectory(componentsDir, 'component');
    }

    private async getApiRoutes(): Promise<WwwTreeItem[]> {
        const apiDir = path.join(this.workspaceRoot!, 'api');
        return this.scanDirectory(apiDir, 'api', this.fileToApiRoute.bind(this));
    }

    private async getContent(): Promise<WwwTreeItem[]> {
        const contentDir = path.join(this.workspaceRoot!, 'content');
        return this.scanDirectory(contentDir, 'content', undefined, ['.md']);
    }

    private async getLayouts(): Promise<WwwTreeItem[]> {
        const layoutsDir = path.join(this.workspaceRoot!, 'layouts');
        return this.scanDirectory(layoutsDir, 'layout');
    }

    private async scanDirectory(
        dir: string,
        itemType: WwwTreeItemType,
        labelTransform?: (filePath: string) => string,
        extensions: string[] = ['.tsx', '.ts', '.jsx', '.js']
    ): Promise<WwwTreeItem[]> {
        const items: WwwTreeItem[] = [];

        try {
            if (!fs.existsSync(dir)) {
                return [];
            }

            const files = await this.walkDirectory(dir, extensions);

            for (const file of files) {
                const relativePath = path.relative(dir, file);
                const label = labelTransform ? labelTransform(relativePath) : this.getFileName(relativePath);

                items.push(new WwwTreeItem(
                    label,
                    itemType,
                    vscode.TreeItemCollapsibleState.None,
                    file,
                    itemType === 'route' ? this.fileToRoute(relativePath) : undefined
                ));
            }
        } catch (error) {
            console.error(`DX WWW: Error scanning directory ${dir}:`, error);
        }

        return items;
    }

    private async walkDirectory(dir: string, extensions: string[]): Promise<string[]> {
        const files: string[] = [];

        try {
            const entries = fs.readdirSync(dir, { withFileTypes: true });

            for (const entry of entries) {
                const fullPath = path.join(dir, entry.name);

                if (entry.isDirectory()) {
                    // Skip node_modules and hidden directories
                    if (!entry.name.startsWith('.') && entry.name !== 'node_modules') {
                        files.push(...await this.walkDirectory(fullPath, extensions));
                    }
                } else if (entry.isFile()) {
                    const ext = path.extname(entry.name);
                    if (extensions.includes(ext)) {
                        files.push(fullPath);
                    }
                }
            }
        } catch (error) {
            // Directory doesn't exist or can't be read
        }

        return files;
    }

    private getFileName(filePath: string): string {
        const basename = path.basename(filePath);
        const ext = path.extname(basename);
        return basename.slice(0, -ext.length);
    }

    private fileToRoute(filePath: string): string {
        // Convert file path to route path
        // pages/index.tsx -> /
        // pages/about.tsx -> /about
        // pages/users/[id].tsx -> /users/[id]

        let route = '/' + filePath
            .replace(/\\/g, '/')
            .replace(/\.(tsx|ts|jsx|js)$/, '')
            .replace(/\/index$/, '');

        if (route === '/') {
            return '/';
        }

        return route || '/';
    }

    private fileToApiRoute(filePath: string): string {
        // Convert file path to API route
        // api/users.ts -> /api/users

        return '/api/' + filePath
            .replace(/\\/g, '/')
            .replace(/\.(tsx|ts|jsx|js)$/, '');
    }

    private async isDxWwwProject(): Promise<boolean> {
        if (!this.workspaceRoot) {
            return false;
        }

        const configFiles = ['dx.config', 'dx', 'dx.config.json', 'dx.config.toml'];

        for (const configFile of configFiles) {
            const configPath = path.join(this.workspaceRoot, configFile);
            if (fs.existsSync(configPath)) {
                return true;
            }
        }

        return false;
    }
}

/**
 * Register the DX-WWW panel
 */
export function registerWwwPanel(context: vscode.ExtensionContext): WwwTreeDataProvider {
    const treeDataProvider = new WwwTreeDataProvider();

    // Register tree view
    const treeView = vscode.window.createTreeView('dx.wwwView', {
        treeDataProvider,
        showCollapseAll: true,
    });

    context.subscriptions.push(treeView);

    // Register refresh command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.refresh', () => {
            treeDataProvider.refresh();
        })
    );

    // Watch for file changes to auto-refresh
    const watcher = vscode.workspace.createFileSystemWatcher('**/*.{tsx,ts,jsx,js,md}');

    watcher.onDidCreate(() => treeDataProvider.refresh());
    watcher.onDidDelete(() => treeDataProvider.refresh());
    watcher.onDidChange(() => treeDataProvider.refresh());

    context.subscriptions.push(watcher);

    console.log('DX WWW: Panel registered');

    return treeDataProvider;
}
