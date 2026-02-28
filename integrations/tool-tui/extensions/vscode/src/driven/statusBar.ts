/**
 * DX Driven Status Bar
 * 
 * Status bar integration for real-time sync status.
 * Requirements: 9.10
 */

import * as vscode from 'vscode';
import { DrivenClient } from './drivenClient';

/**
 * Status bar for Driven sync status
 */
export class DrivenStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private client: DrivenClient;
    private updateInterval: NodeJS.Timeout | undefined;

    constructor() {
        this.client = new DrivenClient();
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            98
        );
        this.statusBarItem.command = 'dx.driven.sync';
        this.update();
        this.startAutoUpdate();
    }

    /**
     * Update the status bar
     */
    async update(): Promise<void> {
        if (!this.client.isInitialized()) {
            this.statusBarItem.text = '$(circle-slash) Driven';
            this.statusBarItem.tooltip = 'Driven not initialized. Click to init.';
            this.statusBarItem.command = 'dx.driven.init';
            this.statusBarItem.show();
            return;
        }

        const statuses = await this.client.getSyncStatus();
        const enabledCount = statuses.filter(s => s.enabled).length;
        const syncedCount = statuses.filter(s => s.status === 'synced').length;
        const errorCount = statuses.filter(s => s.status === 'error').length;

        if (errorCount > 0) {
            this.statusBarItem.text = `$(error) Driven (${errorCount} errors)`;
            this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        } else if (syncedCount === enabledCount && enabledCount > 0) {
            this.statusBarItem.text = `$(check) Driven (${syncedCount}/${enabledCount})`;
            this.statusBarItem.backgroundColor = undefined;
        } else {
            this.statusBarItem.text = `$(sync) Driven (${syncedCount}/${enabledCount})`;
            this.statusBarItem.backgroundColor = undefined;
        }

        this.statusBarItem.tooltip = `Driven: ${syncedCount} of ${enabledCount} editors synced\nClick to sync`;
        this.statusBarItem.show();
    }

    /**
     * Start auto-update interval
     */
    private startAutoUpdate(): void {
        // Update every 30 seconds
        this.updateInterval = setInterval(() => this.update(), 30000);
    }

    /**
     * Show syncing state
     */
    showSyncing(): void {
        this.statusBarItem.text = '$(sync~spin) Driven';
        this.statusBarItem.tooltip = 'Syncing rules...';
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
        }
        this.statusBarItem.dispose();
    }
}
