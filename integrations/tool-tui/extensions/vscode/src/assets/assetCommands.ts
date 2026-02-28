/**
 * Asset Picker Commands
 *
 * Placeholder commands for future asset picker UI (icon picker, font picker, media tools).
 * These commands will be implemented with dx-www framework webviews in a future release.
 *
 * @module assets/assetCommands
 */

import * as vscode from 'vscode';
import { AssetBridge, AssetBridgeError, ErrorCodes } from './assetBridge';

/**
 * Register asset picker placeholder commands.
 *
 * Commands registered:
 * - dx.openIconPicker: Open icon picker UI (coming soon)
 * - dx.openFontPicker: Open font picker UI (coming soon)
 * - dx.openMediaTools: Open media tools UI (coming soon)
 * - dx.checkAssetCLI: Check dx CLI availability for assets
 *
 * @param context VS Code extension context
 */
export function registerAssetPickerCommands(context: vscode.ExtensionContext): void {
    // Create asset bridge instance
    const assetBridge = new AssetBridge();

    // DX: Open Icon Picker (placeholder)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.openIconPicker', async () => {
            vscode.window.showInformationMessage(
                'DX Icon Picker: Coming soon! This feature will provide a visual icon browser with 225+ icon sets.',
                'Learn More'
            ).then(selection => {
                if (selection === 'Learn More') {
                    vscode.env.openExternal(vscode.Uri.parse('https://dx.dev/docs/icons'));
                }
            });
        })
    );

    // DX: Open Font Picker (placeholder)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.openFontPicker', async () => {
            vscode.window.showInformationMessage(
                'DX Font Picker: Coming soon! This feature will provide a visual font browser with 50k+ fonts.',
                'Learn More'
            ).then(selection => {
                if (selection === 'Learn More') {
                    vscode.env.openExternal(vscode.Uri.parse('https://dx.dev/docs/fonts'));
                }
            });
        })
    );

    // DX: Open Media Tools (placeholder)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.openMediaTools', async () => {
            vscode.window.showInformationMessage(
                'DX Media Tools: Coming soon! This feature will provide media processing tools for images, videos, and audio.',
                'Learn More'
            ).then(selection => {
                if (selection === 'Learn More') {
                    vscode.env.openExternal(vscode.Uri.parse('https://dx.dev/docs/media'));
                }
            });
        })
    );

    // DX: Check Asset CLI availability
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.checkAssetCLI', async () => {
            try {
                const version = await assetBridge.checkCLIAvailability();
                vscode.window.showInformationMessage(`DX CLI available: ${version}`);
            } catch (error) {
                if (error instanceof AssetBridgeError) {
                    if (error.code === ErrorCodes.CLI_NOT_FOUND) {
                        vscode.window.showErrorMessage(
                            'DX CLI not found. Please install dx CLI to use asset features.',
                            'Install Guide'
                        ).then(selection => {
                            if (selection === 'Install Guide') {
                                vscode.env.openExternal(vscode.Uri.parse('https://dx.dev/docs/install'));
                            }
                        });
                    } else {
                        vscode.window.showErrorMessage(`DX CLI error: ${error.message}`);
                    }
                } else {
                    vscode.window.showErrorMessage(`Failed to check DX CLI: ${error}`);
                }
            }
        })
    );

    // DX: Search Icons (quick pick)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.searchIcons', async () => {
            const query = await vscode.window.showInputBox({
                prompt: 'Search icons',
                placeHolder: 'Enter icon name (e.g., home, user, settings)',
            });

            if (!query) {
                return;
            }

            try {
                const icons = await assetBridge.searchIcons(query, { limit: 20 });

                if (icons.length === 0) {
                    vscode.window.showInformationMessage(`No icons found for "${query}"`);
                    return;
                }

                const items = icons.map(icon => ({
                    label: `$(symbol-misc) ${icon.prefix}:${icon.id}`,
                    description: icon.setName,
                    detail: `${icon.width}x${icon.height}`,
                    icon,
                }));

                const selected = await vscode.window.showQuickPick(items, {
                    placeHolder: 'Select an icon to copy reference',
                });

                if (selected) {
                    const reference = `icon:${selected.icon.prefix}:${selected.icon.id}`;
                    await vscode.env.clipboard.writeText(reference);
                    vscode.window.showInformationMessage(`Copied: ${reference}`);
                }
            } catch (error) {
                if (error instanceof AssetBridgeError) {
                    vscode.window.showErrorMessage(`Icon search failed: ${error.message}`);
                } else {
                    vscode.window.showErrorMessage(`Icon search failed: ${error}`);
                }
            }
        })
    );

    // DX: Search Fonts (quick pick)
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.searchFonts', async () => {
            const query = await vscode.window.showInputBox({
                prompt: 'Search fonts',
                placeHolder: 'Enter font name (e.g., Roboto, Open Sans)',
            });

            if (!query) {
                return;
            }

            try {
                const fonts = await assetBridge.searchFonts(query, { limit: 20 });

                if (fonts.length === 0) {
                    vscode.window.showInformationMessage(`No fonts found for "${query}"`);
                    return;
                }

                const items = fonts.map(font => ({
                    label: `$(symbol-text) ${font.name}`,
                    description: font.provider,
                    detail: font.category,
                    font,
                }));

                const selected = await vscode.window.showQuickPick(items, {
                    placeHolder: 'Select a font to copy reference',
                });

                if (selected) {
                    const reference = `font:${selected.font.provider}:${selected.font.id}`;
                    await vscode.env.clipboard.writeText(reference);
                    vscode.window.showInformationMessage(`Copied: ${reference}`);
                }
            } catch (error) {
                if (error instanceof AssetBridgeError) {
                    vscode.window.showErrorMessage(`Font search failed: ${error.message}`);
                } else {
                    vscode.window.showErrorMessage(`Font search failed: ${error}`);
                }
            }
        })
    );

    console.log('DX: Asset picker commands registered');
}
