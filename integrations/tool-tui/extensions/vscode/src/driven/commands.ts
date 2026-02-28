/**
 * DX Driven Commands
 * 
 * Command handlers for the Driven panel.
 * Requirements: 9.8, 9.9
 */

import * as vscode from 'vscode';
import { DrivenTreeDataProvider, DrivenTreeItem } from './drivenPanel';
import { SpecMetadata, HookDefinition, DrivenTemplate } from './types';

/**
 * Register all Driven commands
 */
export function registerDrivenCommands(
    context: vscode.ExtensionContext,
    treeDataProvider: DrivenTreeDataProvider
): void {
    const client = treeDataProvider.getClient();

    // Init command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.init', async () => {
            const interactive = await vscode.window.showQuickPick(
                ['Quick Setup', 'Interactive Setup'],
                { placeHolder: 'Choose setup mode' }
            );
            if (interactive) {
                await client.runInit(interactive === 'Interactive Setup');
                treeDataProvider.refresh();
            }
        })
    );

    // Sync command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.sync', async () => {
            vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Syncing rules...',
                    cancellable: false,
                },
                async () => {
                    await client.runSync();
                    treeDataProvider.refresh();
                }
            );
        })
    );

    // Validate command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.validate', async () => {
            await client.runValidate();
        })
    );

    // Refresh command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.refresh', () => {
            treeDataProvider.refresh();
        })
    );

    // Spec init command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.specInit', async () => {
            const name = await vscode.window.showInputBox({
                prompt: 'Enter spec name',
                placeHolder: 'feature-name',
            });
            if (name) {
                const terminal = vscode.window.createTerminal('DX Driven');
                terminal.show();
                terminal.sendText(`dx driven spec init "${name}"`);
                setTimeout(() => treeDataProvider.refresh(), 2000);
            }
        })
    );

    // Open spec command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.openSpec', async (spec: SpecMetadata) => {
            const specFile = vscode.Uri.file(`${spec.path}/spec.md`);
            try {
                await vscode.window.showTextDocument(specFile);
            } catch {
                vscode.window.showWarningMessage(`Spec file not found: ${specFile.fsPath}`);
            }
        })
    );

    // Spec context menu commands
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.specSpecify', async (item: DrivenTreeItem) => {
            const spec = item.data as SpecMetadata;
            const description = await vscode.window.showInputBox({
                prompt: 'Enter feature description',
                placeHolder: 'Describe the feature...',
            });
            if (description) {
                const terminal = vscode.window.createTerminal('DX Driven');
                terminal.show();
                terminal.sendText(`dx driven spec specify "${description}"`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.specPlan', async (item: DrivenTreeItem) => {
            const techStack = await vscode.window.showInputBox({
                prompt: 'Enter tech stack',
                placeHolder: 'rust, typescript, etc.',
            });
            if (techStack) {
                const terminal = vscode.window.createTerminal('DX Driven');
                terminal.show();
                terminal.sendText(`dx driven spec plan "${techStack}"`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.specTasks', async () => {
            const terminal = vscode.window.createTerminal('DX Driven');
            terminal.show();
            terminal.sendText('dx driven spec tasks');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.specImplement', async () => {
            const terminal = vscode.window.createTerminal('DX Driven');
            terminal.show();
            terminal.sendText('dx driven spec implement');
        })
    );

    // Hooks commands
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.hooksCreate', async () => {
            const name = await vscode.window.showInputBox({
                prompt: 'Enter hook name',
                placeHolder: 'my-hook',
            });
            if (name) {
                const terminal = vscode.window.createTerminal('DX Driven');
                terminal.show();
                terminal.sendText(`dx driven hooks create "${name}"`);
                setTimeout(() => treeDataProvider.refresh(), 2000);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.hookToggle', async (item: DrivenTreeItem) => {
            const hook = item.data as HookDefinition;
            const newState = !hook.enabled;
            const success = await client.toggleHook(hook.name, newState);
            if (success) {
                vscode.window.showInformationMessage(
                    `Hook "${hook.name}" ${newState ? 'enabled' : 'disabled'}`
                );
                treeDataProvider.refresh();
            } else {
                vscode.window.showErrorMessage(`Failed to toggle hook "${hook.name}"`);
            }
        })
    );

    // Steering commands
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.steeringCreate', async () => {
            const name = await vscode.window.showInputBox({
                prompt: 'Enter steering file name',
                placeHolder: 'my-guidelines',
            });
            if (name) {
                const terminal = vscode.window.createTerminal('DX Driven');
                terminal.show();
                terminal.sendText(`dx driven steering create "${name}"`);
                setTimeout(() => treeDataProvider.refresh(), 2000);
            }
        })
    );

    // Template apply command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.driven.templateApply', async (template: DrivenTemplate) => {
            const confirm = await vscode.window.showQuickPick(
                ['Yes', 'No'],
                { placeHolder: `Apply template "${template.name}"?` }
            );
            if (confirm === 'Yes') {
                const terminal = vscode.window.createTerminal('DX Driven');
                terminal.show();
                terminal.sendText(`dx driven template apply "${template.id}"`);
            }
        })
    );
}
