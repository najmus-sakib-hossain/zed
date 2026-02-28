/**
 * Forge Commands - VS Code command handlers for Forge integration
 */

import * as vscode from 'vscode';
import { ForgeClient, getForgeClient, TrackedFileChange, GitStatusResponse, GitFileStatus } from './client';
import { exec } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Register all Forge-related commands
 */
export function registerForgeCommands(context: vscode.ExtensionContext): void {
    const client = getForgeClient();

    // Start Forge Daemon
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.start', async () => {
            await startForgeDaemon();
        })
    );

    // Stop Forge Daemon
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.stop', async () => {
            await stopForgeDaemon();
        })
    );

    // Restart Forge Daemon
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.restart', async () => {
            await restartForgeDaemon();
        })
    );

    // Show Forge Status
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.status', async () => {
            await showForgeStatus(client);
        })
    );

    // Show Tool List
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.tools.list', async () => {
            await showToolList(client);
        })
    );

    // Show File Changes
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.changes', async () => {
            await showFileChanges(client);
        })
    );

    // Show Git Status (actual uncommitted changes)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.gitStatus', async () => {
            await showGitStatus(client);
        })
    );

    // Sync with Git
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forge.syncGit', async () => {
            await syncWithGit(client);
        })
    );
}

/**
 * Get the configured forge-cli executable path
 * Auto-detects from workspace if not explicitly configured
 */
function getForgeCliPath(): string {
    const config = vscode.workspace.getConfiguration('dx.forge');
    const configuredPath = config.get<string>('executablePath', '');

    // If explicitly configured, use that
    if (configuredPath) {
        console.log(`DX Forge: Using configured executable path: ${configuredPath}`);
        return configuredPath;
    }

    // Try to auto-detect from workspace
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders && workspaceFolders.length > 0) {
        const workspaceRoot = workspaceFolders[0].uri.fsPath;
        console.log(`DX Forge: Searching for forge-cli executable in workspace: ${workspaceRoot}`);

        // Check common locations in order of preference
        // Windows executables first, then Unix
        const isWindows = process.platform === 'win32';
        const candidates = isWindows ? [
            path.join(workspaceRoot, 'target', 'release', 'forge-cli.exe'),
            path.join(workspaceRoot, 'target', 'debug', 'forge-cli.exe'),
            path.join(workspaceRoot, 'target', 'release', 'forge-cli'),
            path.join(workspaceRoot, 'target', 'debug', 'forge-cli'),
        ] : [
            path.join(workspaceRoot, 'target', 'release', 'forge-cli'),
            path.join(workspaceRoot, 'target', 'debug', 'forge-cli'),
            path.join(workspaceRoot, 'target', 'release', 'forge-cli.exe'),
            path.join(workspaceRoot, 'target', 'debug', 'forge-cli.exe'),
        ];

        for (const candidate of candidates) {
            console.log(`DX Forge: Checking candidate: ${candidate}`);
            if (fs.existsSync(candidate)) {
                console.log(`DX Forge: Found executable at: ${candidate}`);
                return candidate;
            }
        }

        console.log('DX Forge: No executable found in workspace, falling back to PATH');
    } else {
        console.log('DX Forge: No workspace folders found');
    }

    // Fallback to PATH lookup
    return 'forge-cli';
}

/**
 * Start the Forge daemon
 */
async function startForgeDaemon(): Promise<void> {
    const client = getForgeClient();

    if (client.isConnected()) {
        vscode.window.showInformationMessage('Forge daemon is already running');
        return;
    }

    const forgePath = getForgeCliPath();
    const config = vscode.workspace.getConfiguration('dx.forge');
    const port = config.get('port', 9876);

    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: 'Starting Forge daemon...',
            cancellable: false
        },
        async () => {
            return new Promise<void>((resolve) => {
                // Use daemon start command with port
                // Set a timeout to prevent hanging if the process doesn't exit cleanly
                const child = exec(`"${forgePath}" daemon start --port ${port}`, { timeout: 10000 }, (error, stdout, stderr) => {
                    if (error) {
                        const errorMsg = stderr || error.message;
                        // Provide helpful message if forge-cli executable not found
                        if (errorMsg.includes('not recognized') || errorMsg.includes('not found') || errorMsg.includes('ENOENT')) {
                            vscode.window.showErrorMessage(
                                `Failed to start Forge daemon: forge-cli executable not found at "${forgePath}". ` +
                                `Please build the project with 'cargo build --release' or configure 'dx.forge.executablePath' in settings.`,
                                'Open Settings'
                            ).then(action => {
                                if (action === 'Open Settings') {
                                    vscode.commands.executeCommand('workbench.action.openSettings', 'dx.forge.executablePath');
                                }
                            });
                        } else if (!errorMsg.includes('SIGTERM') && !errorMsg.includes('killed')) {
                            // Ignore timeout kills - daemon likely started successfully
                            vscode.window.showErrorMessage(
                                `Failed to start Forge daemon: ${errorMsg}`
                            );
                        }
                    }
                    resolve();
                });

                // Also resolve after a short delay if the process seems to have started
                // This handles the case where the daemon daemonizes but exec doesn't return
                setTimeout(() => {
                    resolve();
                }, 3000);
            });
        }
    );

    // Try to connect after the progress closes
    setTimeout(async () => {
        const connected = await client.connect();
        if (connected) {
            vscode.window.showInformationMessage('Forge daemon started');
        } else {
            vscode.window.showWarningMessage(
                'Forge daemon may have started but connection failed. Check if it\'s running.'
            );
        }
    }, 1000);
}

/**
 * Stop the Forge daemon
 */
async function stopForgeDaemon(): Promise<void> {
    const client = getForgeClient();
    const forgePath = getForgeCliPath();

    vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: 'Stopping Forge daemon...',
            cancellable: false
        },
        async () => {
            return new Promise<void>((resolve) => {
                exec(`"${forgePath}" daemon stop`, (error, stdout, stderr) => {
                    if (error) {
                        vscode.window.showErrorMessage(
                            `Failed to stop Forge daemon: ${stderr || error.message}`
                        );
                    } else {
                        client.disconnect();
                        vscode.window.showInformationMessage('Forge daemon stopped');
                    }
                    resolve();
                });
            });
        }
    );
}

/**
 * Restart the Forge daemon
 */
async function restartForgeDaemon(): Promise<void> {
    const forgePath = getForgeCliPath();

    vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: 'Restarting Forge daemon...',
            cancellable: false
        },
        async () => {
            return new Promise<void>((resolve) => {
                exec(`"${forgePath}" daemon restart`, (error, stdout, stderr) => {
                    if (error) {
                        vscode.window.showErrorMessage(
                            `Failed to restart Forge daemon: ${stderr || error.message}`
                        );
                    } else {
                        const client = getForgeClient();
                        setTimeout(async () => {
                            await client.connect();
                            vscode.window.showInformationMessage('Forge daemon restarted');
                        }, 1500);
                    }
                    resolve();
                });
            });
        }
    );
}

/**
 * Show Forge status in a quick pick
 */
async function showForgeStatus(client: ForgeClient): Promise<void> {
    if (!client.isConnected()) {
        const action = await vscode.window.showWarningMessage(
            'Forge daemon is not running',
            'Start Daemon'
        );
        if (action === 'Start Daemon') {
            await startForgeDaemon();
        }
        return;
    }

    const status = await client.getStatus();
    if (!status) {
        vscode.window.showErrorMessage('Failed to get Forge status');
        return;
    }

    const uptime = formatUptime(status.uptime_seconds);

    const items: vscode.QuickPickItem[] = [
        { label: '$(check) Status', description: 'Running' },
        { label: '$(clock) Uptime', description: uptime },
        { label: '$(file) Files Changed', description: status.files_changed.toString() },
        { label: '$(tools) Tools Executed', description: status.tools_executed.toString() },
        { label: '$(database) Cache Hits', description: status.cache_hits.toString() },
        { label: '$(error) Errors', description: status.errors.toString() },
        { label: '', kind: vscode.QuickPickItemKind.Separator },
        { label: '$(debug-stop) Stop Daemon', description: 'Stop the Forge daemon' },
        { label: '$(debug-restart) Restart Daemon', description: 'Restart the Forge daemon' }
    ];

    const selected = await vscode.window.showQuickPick(items, {
        title: 'DX Forge Status',
        placeHolder: 'Select an action'
    });

    if (selected?.label.includes('Stop')) {
        await stopForgeDaemon();
    } else if (selected?.label.includes('Restart')) {
        await restartForgeDaemon();
    }
}

/**
 * Show tool list in a quick pick
 */
async function showToolList(client: ForgeClient): Promise<void> {
    if (!client.isConnected()) {
        vscode.window.showWarningMessage('Forge daemon is not running');
        return;
    }

    const tools = await client.listTools();
    if (tools.length === 0) {
        vscode.window.showInformationMessage('No tools registered');
        return;
    }

    const items: vscode.QuickPickItem[] = tools.map(tool => ({
        label: `${getStatusIcon(tool.status)} ${tool.name}`,
        description: `v${tool.version}${tool.is_dummy ? ' [dummy]' : ''}`,
        detail: `Runs: ${tool.run_count} | Errors: ${tool.error_count}`
    }));

    const selected = await vscode.window.showQuickPick(items, {
        title: 'DX Tools',
        placeHolder: 'Select a tool to run'
    });

    if (selected) {
        const toolName = selected.label.replace(/^\$\([^)]+\)\s*/, '');
        const run = await vscode.window.showInformationMessage(
            `Run ${toolName}?`,
            'Run'
        );
        if (run === 'Run') {
            await client.runTool(toolName);
            vscode.window.showInformationMessage(`Running ${toolName}...`);
        }
    }
}

function getStatusIcon(status: string): string {
    switch (status) {
        case 'Ready': return '$(check)';
        case 'Running': return '$(sync~spin)';
        case 'Disabled': return '$(circle-slash)';
        case 'Error': return '$(error)';
        default: return '$(question)';
    }
}

function formatUptime(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return `${h}h ${m}m`;
}


/**
 * Show file changes with diffs in a quick pick
 */
async function showFileChanges(client: ForgeClient): Promise<void> {
    if (!client.isConnected()) {
        vscode.window.showWarningMessage('Forge daemon is not running');
        return;
    }

    const changes = await client.getFileChanges(50);
    if (changes.length === 0) {
        vscode.window.showInformationMessage('No file changes tracked');
        return;
    }

    const items: vscode.QuickPickItem[] = changes.map(change => {
        const fileName = change.path.split(/[/\\]/).pop() || change.path;
        const icon = getChangeTypeIcon(change.change_type);
        const diffInfo = change.diff
            ? `+${change.diff.additions} -${change.diff.deletions}`
            : 'no diff';
        const time = formatTimestamp(change.timestamp);

        return {
            label: `${icon} ${fileName}`,
            description: `${change.change_type} | ${diffInfo}`,
            detail: `${change.path} • ${time}`,
            change: change
        } as vscode.QuickPickItem & { change: TrackedFileChange };
    });

    // Add action items
    items.unshift(
        { label: '', kind: vscode.QuickPickItemKind.Separator },
        { label: '$(trash) Clear All Changes', description: 'Clear tracked changes' }
    );

    const selected = await vscode.window.showQuickPick(items, {
        title: 'DX Forge - File Changes',
        placeHolder: 'Select a file to view diff or clear changes'
    });

    if (selected) {
        if (selected.label.includes('Clear All')) {
            await client.clearFileChanges();
            vscode.window.showInformationMessage('File changes cleared');
        } else if ((selected as any).change) {
            const change = (selected as any).change as TrackedFileChange;
            await showFileDiff(change);
        }
    }
}

/**
 * Show file diff in a new document
 */
async function showFileDiff(change: TrackedFileChange): Promise<void> {
    if (!change.diff || change.diff.hunks.length === 0) {
        vscode.window.showInformationMessage(`No diff available for ${change.path}`);
        return;
    }

    // Build diff content
    let diffContent = `# Diff: ${change.path}\n`;
    diffContent += `# Type: ${change.change_type}\n`;
    diffContent += `# Time: ${change.timestamp}\n`;
    diffContent += `# Additions: +${change.diff.additions} | Deletions: -${change.diff.deletions}\n`;
    diffContent += `\n`;

    for (const hunk of change.diff.hunks) {
        diffContent += `@@ -${hunk.old_start},${hunk.old_lines} +${hunk.new_start},${hunk.new_lines} @@\n`;
        diffContent += hunk.content;
        diffContent += '\n';
    }

    // Show in a new document
    const doc = await vscode.workspace.openTextDocument({
        content: diffContent,
        language: 'diff'
    });
    await vscode.window.showTextDocument(doc, { preview: true });
}

function getChangeTypeIcon(changeType: string): string {
    switch (changeType.toLowerCase()) {
        case 'created': return '$(add)';
        case 'modified': return '$(edit)';
        case 'deleted': return '$(trash)';
        default: return '$(file)';
    }
}

function formatTimestamp(timestamp: string): string {
    try {
        const date = new Date(timestamp);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffSecs = Math.floor(diffMs / 1000);

        if (diffSecs < 60) return `${diffSecs}s ago`;
        if (diffSecs < 3600) return `${Math.floor(diffSecs / 60)}m ago`;
        if (diffSecs < 86400) return `${Math.floor(diffSecs / 3600)}h ago`;
        return date.toLocaleDateString();
    } catch {
        return timestamp;
    }
}


/**
 * Show Git status (actual uncommitted changes)
 */
async function showGitStatus(client: ForgeClient): Promise<void> {
    if (!client.isConnected()) {
        // Offer to start the daemon instead of just showing a warning
        const action = await vscode.window.showWarningMessage(
            'Forge daemon is not running',
            'Start Daemon',
            'Cancel'
        );
        if (action === 'Start Daemon') {
            await vscode.commands.executeCommand('dx.forge.start');
        }
        return;
    }

    const status = await client.getGitStatus();
    if (!status) {
        vscode.window.showErrorMessage('Failed to get Git status');
        return;
    }

    if (status.is_clean) {
        vscode.window.showInformationMessage(`Git: Working tree clean (branch: ${status.branch})`);
        return;
    }

    const items: (vscode.QuickPickItem & { file?: GitFileStatus; filePath?: string })[] = [];

    // Staged changes
    if (status.staged.length > 0) {
        items.push({ label: 'Staged Changes', kind: vscode.QuickPickItemKind.Separator });
        for (const file of status.staged) {
            const fileName = file.path.split(/[/\\]/).pop() || file.path;
            const diffInfo = file.diff ? `+${file.diff.additions} -${file.diff.deletions}` : '';
            items.push({
                label: `$(diff-added) ${fileName}`,
                description: `${file.status} ${diffInfo}`,
                detail: file.path,
                file,
            });
        }
    }

    // Unstaged changes
    if (status.unstaged.length > 0) {
        items.push({ label: 'Unstaged Changes', kind: vscode.QuickPickItemKind.Separator });
        for (const file of status.unstaged) {
            const fileName = file.path.split(/[/\\]/).pop() || file.path;
            const diffInfo = file.diff ? `+${file.diff.additions} -${file.diff.deletions}` : '';
            items.push({
                label: `$(diff-modified) ${fileName}`,
                description: `${file.status} ${diffInfo}`,
                detail: file.path,
                file,
            });
        }
    }

    // Untracked files
    if (status.untracked.length > 0) {
        items.push({ label: 'Untracked Files', kind: vscode.QuickPickItemKind.Separator });
        for (const filePath of status.untracked) {
            const fileName = filePath.split(/[/\\]/).pop() || filePath;
            items.push({
                label: `$(question) ${fileName}`,
                description: 'untracked',
                detail: filePath,
                filePath,
            });
        }
    }

    // Actions
    items.push(
        { label: '', kind: vscode.QuickPickItemKind.Separator },
        { label: '$(sync) Sync Forge with Git', description: 'Reset Forge counters to match Git status' }
    );

    const selected = await vscode.window.showQuickPick(items, {
        title: `Git Status (${status.branch}) - ${status.staged.length} staged, ${status.unstaged.length} unstaged, ${status.untracked.length} untracked`,
        placeHolder: 'Select a file to view diff'
    });

    if (selected) {
        if (selected.label.includes('Sync Forge')) {
            await syncWithGit(client);
        } else if (selected.file) {
            await showGitFileDiff(selected.file);
        } else if (selected.filePath) {
            // Open untracked file
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (workspaceFolders) {
                const fullPath = vscode.Uri.joinPath(workspaceFolders[0].uri, selected.filePath);
                await vscode.window.showTextDocument(fullPath);
            }
        }
    }
}

/**
 * Show diff for a Git file using VS Code's built-in diff editor
 */
async function showGitFileDiff(file: GitFileStatus): Promise<void> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const workspaceRoot = workspaceFolders[0].uri;
    const fileUri = vscode.Uri.joinPath(workspaceRoot, file.path);

    // Use VS Code's built-in Git diff
    // Create a git: URI for the HEAD version
    const gitUri = fileUri.with({ scheme: 'git', query: JSON.stringify({ path: file.path, ref: 'HEAD' }) });

    try {
        // Try to use VS Code's diff command
        await vscode.commands.executeCommand('vscode.diff',
            gitUri,
            fileUri,
            `${file.path} (HEAD ↔ Working Tree)`
        );
    } catch {
        // Fallback: show the diff content in a document
        if (file.diff && file.diff.hunks.length > 0) {
            let diffContent = `# Diff: ${file.path}\n`;
            diffContent += `# Status: ${file.status}\n`;
            diffContent += `# Additions: +${file.diff.additions} | Deletions: -${file.diff.deletions}\n\n`;

            for (const hunk of file.diff.hunks) {
                diffContent += `@@ -${hunk.old_start},${hunk.old_lines} +${hunk.new_start},${hunk.new_lines} @@\n`;
                diffContent += hunk.content;
                diffContent += '\n';
            }

            const doc = await vscode.workspace.openTextDocument({
                content: diffContent,
                language: 'diff'
            });
            await vscode.window.showTextDocument(doc, { preview: true });
        } else {
            // Just open the file
            await vscode.window.showTextDocument(fileUri);
        }
    }
}

/**
 * Sync Forge with Git status
 */
async function syncWithGit(client: ForgeClient): Promise<void> {
    if (!client.isConnected()) {
        vscode.window.showWarningMessage('Forge daemon is not running');
        return;
    }

    const status = await client.syncWithGit();
    if (status) {
        const totalChanges = status.staged.length + status.unstaged.length + status.untracked.length;
        if (status.is_clean) {
            vscode.window.showInformationMessage('Forge synced with Git: Working tree clean');
        } else {
            vscode.window.showInformationMessage(
                `Forge synced with Git: ${totalChanges} changes (${status.staged.length} staged, ${status.unstaged.length} unstaged, ${status.untracked.length} untracked)`
            );
        }
    } else {
        vscode.window.showErrorMessage('Failed to sync with Git');
    }
}
