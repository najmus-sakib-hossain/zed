/**
 * DX-WWW VS Code Extension Context Menu
 * 
 * Provides context menu contributions for dx-www:
 * - "New Component" on folder context menu
 * - "New Route" on pages folder context menu
 * - "New API" on api folder context menu
 * 
 * Requirements: 8.3
 */

import * as vscode from 'vscode';
import * as path from 'path';

/**
 * Register context menu contributions for dx-www
 */
export function registerWwwContextMenus(context: vscode.ExtensionContext): void {
    // Register context menu command handlers
    // These commands are triggered from the explorer context menu

    // New Component from context menu
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newComponentFromContext', async (uri: vscode.Uri) => {
            if (uri) {
                await vscode.commands.executeCommand('dx.www.newComponent', uri);
            }
        })
    );

    // New Route from context menu
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newRouteFromContext', async (uri: vscode.Uri) => {
            if (uri) {
                await vscode.commands.executeCommand('dx.www.newRoute', uri);
            }
        })
    );

    // New API from context menu
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newApiFromContext', async (uri: vscode.Uri) => {
            if (uri) {
                await vscode.commands.executeCommand('dx.www.newApi', uri);
            }
        })
    );

    console.log('DX WWW: Context menu contributions registered');
}

/**
 * Check if a URI is a components folder
 */
export function isComponentsFolder(uri: vscode.Uri): boolean {
    const folderName = path.basename(uri.fsPath).toLowerCase();
    return folderName === 'components' || folderName === 'component';
}

/**
 * Check if a URI is a pages folder
 */
export function isPagesFolder(uri: vscode.Uri): boolean {
    const folderName = path.basename(uri.fsPath).toLowerCase();
    return folderName === 'pages' || folderName === 'routes' || folderName === 'app';
}

/**
 * Check if a URI is an API folder
 */
export function isApiFolder(uri: vscode.Uri): boolean {
    const folderName = path.basename(uri.fsPath).toLowerCase();
    return folderName === 'api' || folderName === 'apis';
}

/**
 * Check if a URI is within a dx-www project
 */
export async function isInDxWwwProjectFolder(uri: vscode.Uri): Promise<boolean> {
    // Walk up the directory tree looking for dx.config
    let currentPath = uri.fsPath;
    const root = path.parse(currentPath).root;

    while (currentPath !== root) {
        const configFiles = ['dx.config', 'dx', 'dx.config.json', 'dx.config.toml'];

        for (const configFile of configFiles) {
            const configPath = path.join(currentPath, configFile);
            try {
                await vscode.workspace.fs.stat(vscode.Uri.file(configPath));
                return true;
            } catch {
                // File doesn't exist, continue
            }
        }

        currentPath = path.dirname(currentPath);
    }

    return false;
}
