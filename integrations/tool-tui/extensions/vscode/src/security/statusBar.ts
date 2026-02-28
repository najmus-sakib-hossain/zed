/**
 * Security status bar provider for VS Code
 * 
 * Displays the current security score in the VS Code status bar.
 * _Requirements: 10.4_
 */

import * as vscode from 'vscode';
import { SecurityScore, StatusBarData, getStatusBarIcon } from './types';

/**
 * Security status bar item manager
 */
export class SecurityStatusBar implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private score: SecurityScore = 100;
    private findingsCount: number = 0;
    private scanning: boolean = false;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'dx.security.showDetails';
        this.update();
        this.statusBarItem.show();
    }

    /**
     * Update the status bar with new data
     */
    public updateData(data: StatusBarData): void {
        this.score = data.score;
        this.findingsCount = data.findingsCount;
        this.scanning = data.scanning;
        this.update();
    }

    /**
     * Set the security score
     */
    public setScore(score: SecurityScore): void {
        this.score = score;
        this.update();
    }

    /**
     * Set the findings count
     */
    public setFindingsCount(count: number): void {
        this.findingsCount = count;
        this.update();
    }

    /**
     * Set scanning state
     */
    public setScanning(scanning: boolean): void {
        this.scanning = scanning;
        this.update();
    }

    /**
     * Update the status bar display
     */
    private update(): void {
        if (this.scanning) {
            this.statusBarItem.text = '$(sync~spin) Scanning...';
            this.statusBarItem.tooltip = 'Security scan in progress...';
            this.statusBarItem.color = undefined;
        } else {
            const icon = getStatusBarIcon(this.score);
            this.statusBarItem.text = `${icon} ${this.score}`;
            this.statusBarItem.tooltip = this.getTooltip();
            this.statusBarItem.color = this.getColor();
        }
    }

    /**
     * Get tooltip text
     */
    private getTooltip(): string {
        const lines = [
            `Security Score: ${this.score}/100`,
            `${this.findingsCount} issue(s) found`,
            '',
            'Click to view details',
        ];
        return lines.join('\n');
    }

    /**
     * Get status bar color based on score
     */
    private getColor(): string | undefined {
        if (this.score >= 80) {
            return '#00ff00'; // Green
        } else if (this.score >= 50) {
            return '#ffff00'; // Yellow
        } else {
            return '#ff0000'; // Red
        }
    }

    /**
     * Get current status bar data
     */
    public getData(): StatusBarData {
        return {
            score: this.score,
            findingsCount: this.findingsCount,
            scanning: this.scanning,
            text: this.statusBarItem.text,
            tooltip: this.statusBarItem.tooltip as string,
            color: this.statusBarItem.color as string | undefined,
        };
    }

    /**
     * Show the status bar item
     */
    public show(): void {
        this.statusBarItem.show();
    }

    /**
     * Hide the status bar item
     */
    public hide(): void {
        this.statusBarItem.hide();
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.statusBarItem.dispose();
    }
}
