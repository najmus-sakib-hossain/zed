/**
 * DX DCP Commands
 * 
 * Command handlers for the DCP panel.
 * Requirements: 11.7, 11.8, 11.9, 11.10
 */

import * as vscode from 'vscode';
import { DcpTreeDataProvider, DcpTreeItem } from './dcpPanel';
import { DcpTool, DcpServerStatus } from './types';

/**
 * Register all DCP commands
 */
export function registerDcpCommands(
    context: vscode.ExtensionContext,
    treeDataProvider: DcpTreeDataProvider
): void {
    const client = treeDataProvider.getClient();

    // Start server command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.startServer', async () => {
            const portStr = await vscode.window.showInputBox({
                prompt: 'Enter server port',
                value: '9877',
                validateInput: (value) => {
                    const port = parseInt(value);
                    if (isNaN(port) || port < 1 || port > 65535) {
                        return 'Please enter a valid port number (1-65535)';
                    }
                    return undefined;
                },
            });

            if (portStr) {
                const port = parseInt(portStr);
                await client.startServer(port);
                setTimeout(() => treeDataProvider.refresh(), 2000);
            }
        })
    );

    // Stop server command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.stopServer', async (item?: DcpTreeItem) => {
            const server = item?.data as DcpServerStatus;
            if (server) {
                await client.stopServer();
                treeDataProvider.refresh();
            } else {
                vscode.window.showWarningMessage('No server selected');
            }
        })
    );

    // Refresh command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.refresh', () => {
            treeDataProvider.refresh();
        })
    );

    // Register tool command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.registerTool', async () => {
            const schemaUri = await vscode.window.showOpenDialog({
                canSelectFiles: true,
                canSelectFolders: false,
                canSelectMany: false,
                filters: {
                    'JSON Schema': ['json'],
                },
                title: 'Select Tool Schema',
            });

            if (schemaUri && schemaUri.length > 0) {
                await client.registerTool(schemaUri[0].fsPath);
                setTimeout(() => treeDataProvider.refresh(), 2000);
            }
        })
    );

    // Show tool schema command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.showToolSchema', async (tool: DcpTool) => {
            const schemaContent = JSON.stringify({
                name: tool.name,
                description: tool.description,
                inputSchema: tool.inputSchema,
                outputSchema: tool.outputSchema,
                capabilities: tool.capabilities,
                signed: tool.signed,
                version: tool.version,
            }, null, 2);

            const doc = await vscode.workspace.openTextDocument({
                content: schemaContent,
                language: 'json',
            });
            await vscode.window.showTextDocument(doc, {
                viewColumn: vscode.ViewColumn.Beside,
                preview: true,
            });
        })
    );

    // Invoke tool command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.invokeTool', async (item?: DcpTreeItem) => {
            const tool = item?.data as DcpTool;
            if (!tool) {
                // Show tool picker
                const tools = await client.getTools();
                if (tools.length === 0) {
                    vscode.window.showWarningMessage('No tools registered');
                    return;
                }

                const selected = await vscode.window.showQuickPick(
                    tools.map(t => ({
                        label: t.name,
                        description: t.description,
                        tool: t,
                    })),
                    { placeHolder: 'Select a tool to invoke' }
                );

                if (!selected) return;
                await invokeToolWithParams(client, selected.tool);
            } else {
                await invokeToolWithParams(client, tool);
            }
        })
    );

    // Sign tool command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.signTool', async (item?: DcpTreeItem) => {
            const tool = item?.data as DcpTool;
            if (tool) {
                await client.signTool(tool.id);
                setTimeout(() => treeDataProvider.refresh(), 2000);
            } else {
                vscode.window.showWarningMessage('No tool selected');
            }
        })
    );

    // Run benchmarks command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.runBenchmarks', async () => {
            await client.runBenchmarks();
        })
    );

    // Convert from MCP command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.convertFromMcp', async () => {
            const inputUri = await vscode.window.showOpenDialog({
                canSelectFiles: true,
                canSelectFolders: false,
                canSelectMany: false,
                filters: {
                    'JSON': ['json'],
                },
                title: 'Select MCP Schema',
            });

            if (!inputUri || inputUri.length === 0) return;

            const outputUri = await vscode.window.showSaveDialog({
                filters: {
                    'JSON': ['json'],
                },
                title: 'Save DCP Schema',
            });

            if (!outputUri) return;

            await client.convertFromMcp(inputUri[0].fsPath, outputUri.fsPath);
        })
    );

    // Show MCP compatibility status
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.showMcpStatus', async () => {
            const status = await client.getMcpCompatibility();

            const items = [
                {
                    label: '$(check) MCP Compatibility',
                    description: status.available ? 'Enabled' : 'Disabled',
                },
                {
                    label: '$(versions) MCP Version',
                    description: status.version || 'N/A',
                },
            ];

            if (status.suggestions.length > 0) {
                items.push({ label: '', kind: vscode.QuickPickItemKind.Separator } as any);
                items.push({
                    label: '$(lightbulb) Suggestions',
                    description: '',
                });
                for (const suggestion of status.suggestions) {
                    items.push({
                        label: `  • ${suggestion}`,
                        description: '',
                    });
                }
            }

            await vscode.window.showQuickPick(items, {
                placeHolder: 'MCP Compatibility Status',
            });
        })
    );

    // Show info command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.dcp.showInfo', async () => {
            const terminal = vscode.window.createTerminal('DX DCP');
            terminal.show();
            terminal.sendText('dx dcp info');
        })
    );
}

/**
 * Invoke a tool with parameter input
 */
async function invokeToolWithParams(client: any, tool: DcpTool): Promise<void> {
    const args: Record<string, any> = {};

    // Collect parameters from input schema
    if (tool.inputSchema.properties) {
        for (const [name, prop] of Object.entries(tool.inputSchema.properties)) {
            const isRequired = tool.inputSchema.required?.includes(name);

            const value = await vscode.window.showInputBox({
                prompt: `${name}${isRequired ? ' (required)' : ''}`,
                placeHolder: prop.description || name,
                value: prop.default?.toString(),
            });

            if (value === undefined && isRequired) {
                vscode.window.showWarningMessage('Invocation cancelled');
                return;
            }

            if (value !== undefined && value !== '') {
                // Convert to appropriate type
                if (prop.type === 'number' || prop.type === 'integer') {
                    args[name] = parseFloat(value);
                } else if (prop.type === 'boolean') {
                    args[name] = value.toLowerCase() === 'true';
                } else {
                    args[name] = value;
                }
            }
        }
    }

    const result = await client.invokeTool(tool.id, args);

    if (result.success) {
        if (result.result) {
            // Show result in new document
            const content = typeof result.result === 'string'
                ? result.result
                : JSON.stringify(result.result, null, 2);

            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'json',
            });
            await vscode.window.showTextDocument(doc, {
                viewColumn: vscode.ViewColumn.Beside,
                preview: true,
            });
        }

        if (result.timeUs) {
            vscode.window.setStatusBarMessage(
                `$(check) Tool invoked in ${result.timeUs}μs`,
                5000
            );
        }
    } else {
        vscode.window.showErrorMessage(`Invocation failed: ${result.error}`);
    }
}
