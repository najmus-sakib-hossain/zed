/**
 * Forge Status Bar - Shows daemon connection status
 */

import * as vscode from 'vscode';
import { ForgeClient, ForgeStatus, TrackedFileChange, GitStatusResponse } from './client';

export class ForgeStatusBar {
    private statusBarItem: vscode.StatusBarItem;
    private client: ForgeClient;
    private updateInterval: NodeJS.Timeout | null = null;
    private recentChanges: TrackedFileChange[] = [];
    private gitStatus: GitStatusResponse | null = null;

    constructor(client: ForgeClient) {
        this.client = client;
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            100
        );
        this.statusBarItem.command = 'dx.forge.gitStatus';
        this.show();
    }

    /**
     * Show the status bar item
     */
    show(): void {
        const config = vscode.workspace.getConfiguration('dx.forge');
        if (config.get('showStatusBar', true)) {
            this.statusBarItem.show();
            this.startUpdating();
        }
    }

    /**
     * Hide the status bar item
     */
    hide(): void {
        this.statusBarItem.hide();
        this.stopUpdating();
    }

    /**
     * Start periodic status updates
     */
    private startUpdating(): void {
        // Only update once initially, then use interval
        // But only if connected - don't poll when disconnected
        if (this.client.isConnected()) {
            this.updateStatus();
        }
        this.updateInterval = setInterval(() => {
            // Only update if connected to avoid unnecessary polling
            if (this.client.isConnected()) {
                this.updateStatus();
            }
        }, 5000); // Update every 5 seconds (reduced frequency)
    }

    /**
     * Stop periodic updates
     */
    private stopUpdating(): void {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
            this.updateInterval = null;
        }
    }

    /**
     * Update status display
     */
    private async updateStatus(): Promise<void> {
        if (this.client.isConnected()) {
            try {
                const status = await this.client.getStatus();
                const gitStatus = await this.client.getGitStatus();
                this.gitStatus = gitStatus;
                if (status) {
                    this.showConnected(status, gitStatus);
                } else {
                    this.showConnected();
                }
            } catch (e) {
                console.error('[Forge StatusBar] Error updating status:', e);
                this.showDisconnected();
            }
        } else {
            this.showDisconnected();
        }
    }

    /**
     * Show connected status
     */
    showConnected(status?: ForgeStatus, gitStatus?: GitStatusResponse | null): void {
        // Show Git-based change count (actual uncommitted changes)
        const gitChangeCount = gitStatus
            ? gitStatus.staged.length + gitStatus.unstaged.length + gitStatus.untracked.length
            : 0;

        if (gitStatus?.is_clean) {
            this.statusBarItem.text = `$(check) DX Forge`;
        } else if (gitChangeCount > 0) {
            this.statusBarItem.text = `$(git-commit) DX Forge (${gitChangeCount})`;
        } else {
            this.statusBarItem.text = '$(check) DX Forge';
        }
        this.statusBarItem.backgroundColor = undefined;

        if (status) {
            const uptime = this.formatUptime(status.uptime_seconds);
            let tooltipContent =
                `**DX Forge** - Connected\n\n` +
                `- Uptime: ${uptime}\n` +
                `- Tools executed: ${status.tools_executed}\n` +
                `- Cache hits: ${status.cache_hits}\n` +
                `- Errors: ${status.errors}`;

            // Add Git status info
            if (gitStatus) {
                tooltipContent += `\n\n---\n\n**Git Status** (${gitStatus.branch}):\n`;
                if (gitStatus.is_clean) {
                    tooltipContent += `\n$(check) Working tree clean`;
                } else {
                    if (gitStatus.staged.length > 0) {
                        tooltipContent += `\n$(diff-added) ${gitStatus.staged.length} staged`;
                    }
                    if (gitStatus.unstaged.length > 0) {
                        tooltipContent += `\n$(diff-modified) ${gitStatus.unstaged.length} unstaged`;
                    }
                    if (gitStatus.untracked.length > 0) {
                        tooltipContent += `\n$(question) ${gitStatus.untracked.length} untracked`;
                    }

                    // Show first few changed files
                    const allChanges = [...gitStatus.staged, ...gitStatus.unstaged];
                    if (allChanges.length > 0) {
                        tooltipContent += `\n\n**Changed Files:**`;
                        for (const file of allChanges.slice(0, 5)) {
                            const fileName = file.path.split(/[/\\]/).pop() || file.path;
                            const diffInfo = file.diff ? ` (+${file.diff.additions} -${file.diff.deletions})` : '';
                            tooltipContent += `\n- \`${fileName}\`${diffInfo}`;
                        }
                        if (allChanges.length > 5) {
                            tooltipContent += `\n\n_...and ${allChanges.length - 5} more_`;
                        }
                    }
                }
            }

            tooltipContent += `\n\n_Click to view Git status_`;
            this.statusBarItem.tooltip = new vscode.MarkdownString(tooltipContent);
        } else {
            this.statusBarItem.tooltip = 'DX Forge - Connected';
        }
    }

    /**
     * Get icon for change type
     */
    private getChangeIcon(changeType: string): string {
        switch (changeType.toLowerCase()) {
            case 'created': return '$(add)';
            case 'modified': return '$(edit)';
            case 'deleted': return '$(trash)';
            default: return '$(file)';
        }
    }

    /**
     * Show disconnected status
     */
    showDisconnected(): void {
        this.statusBarItem.text = '$(circle-slash) DX Forge';
        this.statusBarItem.backgroundColor = new vscode.ThemeColor(
            'statusBarItem.warningBackground'
        );
        this.statusBarItem.tooltip = 'DX Forge - Disconnected (click to start)';
    }

    /**
     * Show connecting status
     */
    showConnecting(): void {
        this.statusBarItem.text = '$(sync~spin) DX Forge';
        this.statusBarItem.backgroundColor = undefined;
        this.statusBarItem.tooltip = 'DX Forge - Connecting...';
    }

    /**
     * Format uptime in human-readable form
     */
    private formatUptime(seconds: number): string {
        if (seconds < 60) {
            return `${seconds}s`;
        } else if (seconds < 3600) {
            return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
        } else {
            const hours = Math.floor(seconds / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            return `${hours}h ${mins}m`;
        }
    }

    /**
     * Get recent changes
     */
    getRecentChanges(): TrackedFileChange[] {
        return this.recentChanges;
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        this.stopUpdating();
        this.statusBarItem.dispose();
    }
}
