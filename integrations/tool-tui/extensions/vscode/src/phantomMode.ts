/**
 * Phantom Mode: Hides shadow files from file explorer
 * 
 * When Phantom Mode is enabled, certain generated files are hidden
 * from the VS Code file explorer for a cleaner experience.
 * 
 * Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 11.3, 11.6
 */

import * as vscode from 'vscode';
import * as path from 'path';

/**
 * Phantom Mode Manager
 * 
 * Manages the visibility of shadow .md files in the VS Code file explorer.
 */
export class PhantomModeManager implements vscode.Disposable {
    private enabled: boolean = false;
    private excludePatterns: Map<string, boolean> = new Map();
    private fileWatcher: vscode.FileSystemWatcher | undefined;
    private disposables: vscode.Disposable[] = [];
    private context: vscode.ExtensionContext;
    private statusBarItem: vscode.StatusBarItem;

    constructor(context: vscode.ExtensionContext) {
        this.context = context;

        // Create status bar item
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            99
        );
        this.statusBarItem.command = 'dx.togglePhantomMode';
        this.disposables.push(this.statusBarItem);

        // Load enabled state from settings first, then fall back to global state
        const config = vscode.workspace.getConfiguration('dx');
        const settingsEnabled = config.get<boolean>('phantomMode.enabled');

        if (settingsEnabled !== undefined) {
            this.enabled = settingsEnabled;
        } else {
            // Fall back to global state for backward compatibility
            this.enabled = context.globalState.get<boolean>('phantomMode.enabled', false);
        }

        // Initialize if enabled
        if (this.enabled) {
            this.enable().catch(console.error);
        }

        this.updateStatusBar();

        // Listen for configuration changes (Requirements: 11.6)
        this.disposables.push(
            vscode.workspace.onDidChangeConfiguration((event) => {
                if (event.affectsConfiguration('dx.phantomMode.enabled')) {
                    this.handleSettingsChange();
                }
            })
        );
    }

    /**
     * Handle settings change - apply changes without restart
     * Requirements: 11.6
     */
    private async handleSettingsChange(): Promise<void> {
        const config = vscode.workspace.getConfiguration('dx');
        const newEnabled = config.get<boolean>('phantomMode.enabled', false);

        if (newEnabled !== this.enabled) {
            if (newEnabled) {
                await this.enable();
            } else {
                await this.disable();
            }
        }
    }

    /**
     * Enable phantom mode - hide generated files
     */
    async enable(): Promise<void> {
        if (this.enabled && this.fileWatcher) {
            return; // Already enabled
        }

        this.enabled = true;

        // Update both global state and settings
        await this.context.globalState.update('phantomMode.enabled', true);
        const config = vscode.workspace.getConfiguration('dx');
        await config.update('phantomMode.enabled', true, vscode.ConfigurationTarget.Global);

        // Set up file watcher
        this.setupFileWatcher();

        // Scan workspace and update exclusions
        await this.updateExclusions();

        this.updateStatusBar();
        vscode.window.showInformationMessage('DX: Phantom Mode enabled - shadow .md files are now hidden');
    }

    /**
     * Disable phantom mode - show all files
     */
    async disable(): Promise<void> {
        if (!this.enabled) {
            return; // Already disabled
        }

        this.enabled = false;

        // Update both global state and settings
        await this.context.globalState.update('phantomMode.enabled', false);
        const config = vscode.workspace.getConfiguration('dx');
        await config.update('phantomMode.enabled', false, vscode.ConfigurationTarget.Global);

        // Dispose file watcher
        if (this.fileWatcher) {
            this.fileWatcher.dispose();
            this.fileWatcher = undefined;
        }

        // Remove our exclusion patterns
        await this.removeExclusions();

        this.updateStatusBar();
        vscode.window.showInformationMessage('DX: Phantom Mode disabled - all files are now visible');
    }

    /**
     * Toggle phantom mode
     */
    async toggle(): Promise<void> {
        if (this.enabled) {
            await this.disable();
        } else {
            await this.enable();
        }
    }

    /**
     * Check if phantom mode is enabled
     */
    isEnabled(): boolean {
        return this.enabled;
    }

    /**
     * Set up file watcher for file changes
     */
    private setupFileWatcher(): void {
        if (this.fileWatcher) {
            this.fileWatcher.dispose();
        }

        // Watch for .machine file creation/deletion
        this.fileWatcher = vscode.workspace.createFileSystemWatcher('**/*.machine');

        this.fileWatcher.onDidCreate(async (uri) => {
            await this.handleMachineCreated(uri);
        });

        this.fileWatcher.onDidDelete(async (uri) => {
            await this.handleMachineDeleted(uri);
        });

        this.disposables.push(this.fileWatcher);
    }

    /**
     * Handle .machine file creation
     */
    private async handleMachineCreated(machineUri: vscode.Uri): Promise<void> {
        // No-op for now - machine files don't need special handling
    }

    /**
     * Handle .machine file deletion
     */
    private async handleMachineDeleted(machineUri: vscode.Uri): Promise<void> {
        // No-op for now
    }

    /**
     * Get pattern for a file
     */
    private getPatternForFile(uri: vscode.Uri): string | null {
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
        if (!workspaceFolder) {
            return null;
        }

        const relativePath = path.relative(workspaceFolder.uri.fsPath, dirName);
        const mdFileName = `${baseName}.md`;

        if (relativePath) {
            return `${relativePath}/${mdFileName}`.replace(/\\/g, '/');
        }
        return mdFileName;
    }

    /**
     * Scan workspace and update exclusion patterns
     */
    private async updateExclusions(): Promise<void> {
        this.excludePatterns.clear();

        // Hide .machine files
        this.excludePatterns.set('**/*.machine', true);
        
        // Hide .llm files
        this.excludePatterns.set('**/*.llm', true);

        await this.applyExclusions();
    }

    /**
     * Apply exclusion patterns to workspace settings
     */
    private async applyExclusions(): Promise<void> {
        const config = vscode.workspace.getConfiguration('files');
        const currentExclude = config.get<Record<string, boolean>>('exclude') || {};

        // Preserve user's existing exclusions
        const newExclude: Record<string, boolean> = { ...currentExclude };

        // Add our phantom mode exclusions
        for (const [pattern, value] of this.excludePatterns) {
            newExclude[pattern] = value;
        }

        // Update workspace settings
        await config.update('exclude', newExclude, vscode.ConfigurationTarget.Workspace);
    }

    /**
     * Remove our exclusion patterns from workspace settings
     */
    private async removeExclusions(): Promise<void> {
        const config = vscode.workspace.getConfiguration('files');
        const currentExclude = config.get<Record<string, boolean>>('exclude') || {};

        // Remove only our phantom mode exclusions
        const newExclude: Record<string, boolean> = {};
        for (const [pattern, value] of Object.entries(currentExclude)) {
            if (!this.excludePatterns.has(pattern) && pattern !== '**/*.machine' && pattern !== '**/*.llm') {
                newExclude[pattern] = value;
            }
        }

        // Update workspace settings
        await config.update('exclude', newExclude, vscode.ConfigurationTarget.Workspace);

        this.excludePatterns.clear();
    }

    /**
     * Update status bar item
     */
    private updateStatusBar(): void {
        if (this.enabled) {
            this.statusBarItem.text = '$(eye-closed) Phantom';
            this.statusBarItem.tooltip = 'DX Phantom Mode: ON - Click to show .md files';
            this.statusBarItem.backgroundColor = undefined;
        } else {
            this.statusBarItem.text = '$(eye) Phantom';
            this.statusBarItem.tooltip = 'DX Phantom Mode: OFF - Click to hide .md files';
            this.statusBarItem.backgroundColor = undefined;
        }
        this.statusBarItem.show();
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}

/**
 * Register phantom mode commands
 */
export function registerPhantomModeCommands(
    context: vscode.ExtensionContext,
    manager: PhantomModeManager
): void {
    // Toggle Phantom Mode command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.togglePhantomMode', async () => {
            await manager.toggle();
        })
    );

    // Enable Phantom Mode command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.enablePhantomMode', async () => {
            await manager.enable();
        })
    );

    // Disable Phantom Mode command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.disablePhantomMode', async () => {
            await manager.disable();
        })
    );
}
