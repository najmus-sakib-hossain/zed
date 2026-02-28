/**
 * DX-WWW VS Code Extension Commands
 * 
 * Provides VS Code integration for dx-www CLI commands:
 * - dx.www.newProject: Create a new dx-www project
 * - dx.www.newComponent: Generate a new component
 * - dx.www.newRoute: Generate a new page route
 * - dx.www.newApi: Generate a new API route
 * 
 * Requirements: 8.1, 8.2
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { execDxCommand, getDxExecutablePath } from './wwwUtils';

/**
 * Project templates available for dx www new
 */
const PROJECT_TEMPLATES = [
    { label: 'minimal', description: 'Minimal setup with just the essentials' },
    { label: 'default', description: 'Standard setup with common features' },
    { label: 'full', description: 'Full-featured setup with all integrations' },
    { label: 'api-only', description: 'API-only setup without frontend' },
];

/**
 * HTTP methods for API routes
 */
const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'];

/**
 * Register all dx-www commands
 */
export function registerWwwCommands(context: vscode.ExtensionContext): void {
    // dx.www.newProject - Create a new dx-www project
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newProject', async () => {
            await createNewProject();
        })
    );

    // dx.www.newComponent - Generate a new component
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newComponent', async (uri?: vscode.Uri) => {
            await createNewComponent(uri);
        })
    );

    // dx.www.newRoute - Generate a new page route
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newRoute', async (uri?: vscode.Uri) => {
            await createNewRoute(uri);
        })
    );

    // dx.www.newApi - Generate a new API route
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.www.newApi', async (uri?: vscode.Uri) => {
            await createNewApi(uri);
        })
    );

    console.log('DX WWW: Commands registered');
}

/**
 * Create a new dx-www project
 * Requirements: 8.1
 */
async function createNewProject(): Promise<void> {
    // Get project name
    const name = await vscode.window.showInputBox({
        prompt: 'Enter project name',
        placeHolder: 'my-dx-app',
        validateInput: (value) => {
            if (!value || value.trim().length === 0) {
                return 'Project name is required';
            }
            if (!/^[a-z0-9-_]+$/i.test(value)) {
                return 'Project name can only contain letters, numbers, hyphens, and underscores';
            }
            return null;
        },
    });

    if (!name) {
        return;
    }

    // Select template
    const template = await vscode.window.showQuickPick(PROJECT_TEMPLATES, {
        placeHolder: 'Select project template',
        title: 'DX WWW: New Project',
    });

    if (!template) {
        return;
    }

    // Select location
    const folderUri = await vscode.window.showOpenDialog({
        canSelectFiles: false,
        canSelectFolders: true,
        canSelectMany: false,
        openLabel: 'Select Location',
        title: 'Select location for new project',
    });

    if (!folderUri || folderUri.length === 0) {
        return;
    }

    const targetPath = path.join(folderUri[0].fsPath, name);

    // Execute command
    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: `Creating dx-www project: ${name}`,
            cancellable: false,
        },
        async () => {
            try {
                await execDxCommand(`www new ${name} --template ${template.label}`, folderUri[0].fsPath);

                // Ask to open the new project
                const openChoice = await vscode.window.showInformationMessage(
                    `Project "${name}" created successfully!`,
                    'Open in New Window',
                    'Open in Current Window',
                    'Cancel'
                );

                if (openChoice === 'Open in New Window') {
                    await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(targetPath), true);
                } else if (openChoice === 'Open in Current Window') {
                    await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(targetPath), false);
                }
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to create project: ${error}`);
            }
        }
    );
}

/**
 * Create a new component
 * Requirements: 8.2
 */
async function createNewComponent(uri?: vscode.Uri): Promise<void> {
    // Get component name
    const name = await vscode.window.showInputBox({
        prompt: 'Enter component name',
        placeHolder: 'Button',
        validateInput: (value) => {
            if (!value || value.trim().length === 0) {
                return 'Component name is required';
            }
            if (!/^[A-Z][a-zA-Z0-9]*$/.test(value)) {
                return 'Component name must start with uppercase letter (PascalCase)';
            }
            return null;
        },
    });

    if (!name) {
        return;
    }

    // Ask for test file
    const withTest = await vscode.window.showQuickPick(
        [
            { label: 'Yes', description: 'Include test file', value: true },
            { label: 'No', description: 'Component only', value: false },
        ],
        {
            placeHolder: 'Include test file?',
            title: 'DX WWW: New Component',
        }
    );

    if (!withTest) {
        return;
    }

    // Determine output path
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    let outputPath = uri?.fsPath || path.join(workspaceFolder.uri.fsPath, 'components');

    // Execute command
    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: `Creating component: ${name}`,
            cancellable: false,
        },
        async () => {
            try {
                const testFlag = withTest.value ? '--with-test' : '';
                await execDxCommand(
                    `www component ${name} --path "${outputPath}" ${testFlag}`,
                    workspaceFolder.uri.fsPath
                );

                // Open the created file
                const componentFile = path.join(outputPath, `${name}.tsx`);
                const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(componentFile));
                await vscode.window.showTextDocument(doc);

                vscode.window.showInformationMessage(`Component "${name}" created successfully!`);
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to create component: ${error}`);
            }
        }
    );
}

/**
 * Create a new page route
 * Requirements: 8.2
 */
async function createNewRoute(uri?: vscode.Uri): Promise<void> {
    // Get route path
    const routePath = await vscode.window.showInputBox({
        prompt: 'Enter route path',
        placeHolder: '/dashboard or /users/[id]',
        validateInput: (value) => {
            if (!value || value.trim().length === 0) {
                return 'Route path is required';
            }
            if (!value.startsWith('/')) {
                return 'Route path must start with /';
            }
            return null;
        },
    });

    if (!routePath) {
        return;
    }

    // Ask for layout
    const layout = await vscode.window.showInputBox({
        prompt: 'Enter layout name (optional)',
        placeHolder: 'default',
    });

    // Determine if dynamic route
    const isDynamic = routePath.includes('[') && routePath.includes(']');

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    // Execute command
    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: `Creating route: ${routePath}`,
            cancellable: false,
        },
        async () => {
            try {
                let cmd = `www route ${routePath}`;
                if (layout) {
                    cmd += ` --layout ${layout}`;
                }
                if (isDynamic) {
                    cmd += ' --dynamic';
                }

                await execDxCommand(cmd, workspaceFolder.uri.fsPath);

                // Calculate the file path from route
                const filePath = routeToFilePath(routePath, workspaceFolder.uri.fsPath);
                if (filePath) {
                    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
                    await vscode.window.showTextDocument(doc);
                }

                vscode.window.showInformationMessage(`Route "${routePath}" created successfully!`);
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to create route: ${error}`);
            }
        }
    );
}

/**
 * Create a new API route
 * Requirements: 8.2
 */
async function createNewApi(uri?: vscode.Uri): Promise<void> {
    // Get API name
    const name = await vscode.window.showInputBox({
        prompt: 'Enter API name',
        placeHolder: 'users',
        validateInput: (value) => {
            if (!value || value.trim().length === 0) {
                return 'API name is required';
            }
            if (!/^[a-z][a-zA-Z0-9-]*$/.test(value)) {
                return 'API name must start with lowercase letter';
            }
            return null;
        },
    });

    if (!name) {
        return;
    }

    // Select HTTP method
    const method = await vscode.window.showQuickPick(HTTP_METHODS, {
        placeHolder: 'Select HTTP method',
        title: 'DX WWW: New API Route',
    });

    if (!method) {
        return;
    }

    // Ask for schema
    const withSchema = await vscode.window.showQuickPick(
        [
            { label: 'Yes', description: 'Include validation schema', value: true },
            { label: 'No', description: 'Handler only', value: false },
        ],
        {
            placeHolder: 'Include validation schema?',
            title: 'DX WWW: New API Route',
        }
    );

    if (!withSchema) {
        return;
    }

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    // Execute command
    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: `Creating API route: ${name}`,
            cancellable: false,
        },
        async () => {
            try {
                const schemaFlag = withSchema.value ? '--with-schema' : '';
                await execDxCommand(
                    `www api ${name} --method ${method} ${schemaFlag}`,
                    workspaceFolder.uri.fsPath
                );

                // Open the created file
                const apiFile = path.join(workspaceFolder.uri.fsPath, 'api', `${name}.ts`);
                const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(apiFile));
                await vscode.window.showTextDocument(doc);

                vscode.window.showInformationMessage(`API route "${name}" created successfully!`);
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to create API route: ${error}`);
            }
        }
    );
}

/**
 * Convert route path to file path
 * /dashboard -> pages/dashboard.tsx
 * /users/[id] -> pages/users/[id].tsx
 */
function routeToFilePath(routePath: string, workspaceRoot: string): string | null {
    try {
        // Remove leading slash and add .tsx extension
        const relativePath = routePath.slice(1) || 'index';
        return path.join(workspaceRoot, 'pages', `${relativePath}.tsx`);
    } catch {
        return null;
    }
}
