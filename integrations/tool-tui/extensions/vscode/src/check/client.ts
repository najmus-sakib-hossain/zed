/**
 * DX Check Language Client
 * 
 * Integrates dx-check linting into VS Code via LSP protocol.
 * 
 * Features:
 * - Real-time diagnostics as you type
 * - Auto-fix code actions
 * - Rule documentation on hover
 * - Format on save integration
 */

import * as vscode from 'vscode';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    Executable,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

/**
 * DX Check configuration from VS Code settings
 */
export interface DxCheckConfig {
    enable: boolean;
    executablePath: string;
    lintOnSave: boolean;
    lintOnType: boolean;
    lintDelay: number;
    autoFix: boolean;
    showDiagnostics: boolean;
    rulesDir: string;
}

/**
 * Load configuration from VS Code settings
 */
export function getConfig(): DxCheckConfig {
    const config = vscode.workspace.getConfiguration('dx.check');
    return {
        enable: config.get('enable', true),
        executablePath: config.get('executablePath', ''),
        lintOnSave: config.get('lintOnSave', true),
        lintOnType: config.get('lintOnType', true),
        lintDelay: config.get('lintDelay', 300),
        autoFix: config.get('autoFix', false),
        showDiagnostics: config.get('showDiagnostics', true),
        rulesDir: config.get('rulesDir', ''),
    };
}

/**
 * Find the dx-check executable
 */
async function findDxCheckExecutable(config: DxCheckConfig): Promise<string | undefined> {
    // 1. Check configured path
    if (config.executablePath) {
        try {
            await vscode.workspace.fs.stat(vscode.Uri.file(config.executablePath));
            return config.executablePath;
        } catch {
            vscode.window.showWarningMessage(
                `dx-check: Configured executable not found at ${config.executablePath}`
            );
        }
    }

    // 2. Check workspace root for local installation
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders) {
        for (const folder of workspaceFolders) {
            // Check target/release first (release build)
            const releasePath = path.join(
                folder.uri.fsPath,
                'crates',
                'check',
                'target',
                'release',
                process.platform === 'win32' ? 'dx-check.exe' : 'dx-check'
            );
            try {
                await vscode.workspace.fs.stat(vscode.Uri.file(releasePath));
                return releasePath;
            } catch {
                // Continue checking
            }

            // Check target/debug (debug build)
            const debugPath = path.join(
                folder.uri.fsPath,
                'crates',
                'check',
                'target',
                'debug',
                process.platform === 'win32' ? 'dx-check.exe' : 'dx-check'
            );
            try {
                await vscode.workspace.fs.stat(vscode.Uri.file(debugPath));
                return debugPath;
            } catch {
                // Continue checking
            }

            // Check root target folder
            const rootReleasePath = path.join(
                folder.uri.fsPath,
                'target',
                'release',
                process.platform === 'win32' ? 'dx-check.exe' : 'dx-check'
            );
            try {
                await vscode.workspace.fs.stat(vscode.Uri.file(rootReleasePath));
                return rootReleasePath;
            } catch {
                // Continue checking
            }

            const rootDebugPath = path.join(
                folder.uri.fsPath,
                'target',
                'debug',
                process.platform === 'win32' ? 'dx-check.exe' : 'dx-check'
            );
            try {
                await vscode.workspace.fs.stat(vscode.Uri.file(rootDebugPath));
                return rootDebugPath;
            } catch {
                // Continue checking
            }
        }
    }

    // 3. Fall back to PATH
    // The LSP client will look for it in PATH if we just use the command name
    return 'dx-check';
}

/**
 * Initialize the dx-check language client
 */
export async function initializeDxCheck(context: vscode.ExtensionContext): Promise<void> {
    const config = getConfig();

    if (!config.enable) {
        console.log('dx-check: Disabled in configuration');
        return;
    }

    const executable = await findDxCheckExecutable(config);
    if (!executable) {
        console.log('dx-check: Executable not found');
        return;
    }

    console.log(`dx-check: Using executable at ${executable}`);

    // Create server options
    const serverExecutable: Executable = {
        command: executable,
        args: ['lsp'],
        transport: TransportKind.stdio,
    };

    const serverOptions: ServerOptions = {
        run: serverExecutable,
        debug: serverExecutable,
    };

    // Create client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'typescriptreact' },
            { scheme: 'file', language: 'javascriptreact' },
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{ts,tsx,js,jsx,mjs,cjs}'),
        },
        initializationOptions: {
            rulesDir: config.rulesDir || undefined,
            lintDelay: config.lintDelay,
            autoFix: config.autoFix,
        },
        outputChannelName: 'DX Check',
    };

    // Create the language client
    client = new LanguageClient(
        'dx-check',
        'DX Check Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client (also starts the server)
    await client.start();
    console.log('dx-check: Language server started');

    // Register commands
    registerDxCheckCommands(context, config);
}

/**
 * Register dx-check commands
 */
function registerDxCheckCommands(context: vscode.ExtensionContext, config: DxCheckConfig): void {
    // Lint current file
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.check.lint', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('No active editor');
                return;
            }
            // Force a save to trigger linting
            await editor.document.save();
            vscode.window.showInformationMessage('DX Check: Linting complete');
        })
    );

    // Lint workspace
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.check.lintWorkspace', async () => {
            vscode.window.showInformationMessage('DX Check: Linting workspace...');
            // TODO: Implement workspace-wide linting via LSP
        })
    );

    // Fix current file
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.check.fix', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('No active editor');
                return;
            }
            // Execute code action for all diagnostics
            const uri = editor.document.uri;
            const diagnostics = vscode.languages.getDiagnostics(uri);
            let fixCount = 0;

            for (const diag of diagnostics) {
                if (diag.source === 'dx-check') {
                    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
                        'vscode.executeCodeActionProvider',
                        uri,
                        diag.range,
                        vscode.CodeActionKind.QuickFix
                    );
                    if (actions && actions.length > 0) {
                        for (const action of actions) {
                            if (action.edit) {
                                await vscode.workspace.applyEdit(action.edit);
                                fixCount++;
                            }
                        }
                    }
                }
            }

            if (fixCount > 0) {
                vscode.window.showInformationMessage(`DX Check: Applied ${fixCount} fixes`);
            } else {
                vscode.window.showInformationMessage('DX Check: No fixes available');
            }
        })
    );

    // Fix workspace
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.check.fixWorkspace', async () => {
            vscode.window.showInformationMessage('DX Check: Fixing all files in workspace...');
            // TODO: Implement workspace-wide fixes
        })
    );

    // Show rules
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.check.showRules', async () => {
            // Open output channel with rule list
            const outputChannel = vscode.window.createOutputChannel('DX Check Rules');
            outputChannel.show();
            outputChannel.appendLine('DX Check Rules');
            outputChannel.appendLine('==============');
            outputChannel.appendLine('');
            outputChannel.appendLine('For a complete list of rules, run:');
            outputChannel.appendLine('  dx-check rule list');
            outputChannel.appendLine('');
            outputChannel.appendLine('Or visit: https://dx.dev/docs/check/rules');
        })
    );

    // Restart language server
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.check.restart', async () => {
            if (client) {
                await client.stop();
                await client.start();
                vscode.window.showInformationMessage('DX Check: Language server restarted');
            }
        })
    );
}

/**
 * Dispose the dx-check client
 */
export async function disposeDxCheck(): Promise<void> {
    if (client) {
        await client.stop();
        client = undefined;
    }
}

/**
 * Get the current client status
 */
export function isDxCheckRunning(): boolean {
    return client !== undefined && client.isRunning();
}
