/**
 * DX-WWW VS Code Extension Utilities
 * 
 * Utility functions for executing dx CLI commands and managing paths.
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { spawn } from 'child_process';

/**
 * Get the path to the dx executable
 */
export function getDxExecutablePath(): string {
    const config = vscode.workspace.getConfiguration('dx');
    const configuredPath = config.get<string>('executablePath', '');

    if (configuredPath) {
        return configuredPath;
    }

    // Try to find in workspace
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (workspaceFolder) {
        const workspaceDx = path.join(workspaceFolder.uri.fsPath, 'dx');
        // On Windows, also check for dx.exe
        const workspaceDxExe = path.join(workspaceFolder.uri.fsPath, 'dx.exe');

        // Return workspace path if it might exist (actual check happens at runtime)
        return process.platform === 'win32' ? workspaceDxExe : workspaceDx;
    }

    // Fall back to PATH
    return 'dx';
}

/**
 * Execute a dx CLI command
 */
export async function execDxCommand(command: string, cwd?: string): Promise<string> {
    return new Promise((resolve, reject) => {
        const dxPath = getDxExecutablePath();
        const args = command.split(' ').filter(arg => arg.length > 0);

        const workingDir = cwd || vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || process.cwd();

        console.log(`DX WWW: Executing: ${dxPath} ${args.join(' ')} in ${workingDir}`);

        const child = spawn(dxPath, args, {
            cwd: workingDir,
            shell: true,
            env: { ...process.env },
        });

        let stdout = '';
        let stderr = '';

        child.stdout?.on('data', (data) => {
            stdout += data.toString();
        });

        child.stderr?.on('data', (data) => {
            stderr += data.toString();
        });

        child.on('close', (code) => {
            if (code === 0) {
                resolve(stdout);
            } else {
                reject(new Error(stderr || `Command failed with exit code ${code}`));
            }
        });

        child.on('error', (error) => {
            reject(error);
        });
    });
}

/**
 * Check if we're in a dx-www project
 */
export async function isInDxWwwProject(): Promise<boolean> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return false;
    }

    // Check for dx.config file
    const configFiles = ['dx.config', 'dx', 'dx.config.json', 'dx.config.toml'];

    for (const configFile of configFiles) {
        const configPath = path.join(workspaceFolder.uri.fsPath, configFile);
        try {
            await vscode.workspace.fs.stat(vscode.Uri.file(configPath));
            return true;
        } catch {
            // File doesn't exist, continue checking
        }
    }

    return false;
}

/**
 * Get the project root directory
 */
export function getProjectRoot(): string | undefined {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

/**
 * Validate that a path is within the workspace
 */
export function isPathInWorkspace(targetPath: string): boolean {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return false;
    }

    const normalizedTarget = path.normalize(targetPath);
    const normalizedWorkspace = path.normalize(workspaceFolder.uri.fsPath);

    return normalizedTarget.startsWith(normalizedWorkspace);
}
