/**
 * DX VS Code Extension - Entry Point
 * 
 * The unified DX ecosystem extension providing:
 * - Serializer: Seamless editing of .dx files with human-readable display
 * - Forge Integration: Connection to Forge daemon for tool orchestration
 * - Holographic Git: Phantom Mode, Binary Cache, and Hologram View for .dxm files
 * 
 * Features:
 * - Human Format V3: Clean vertical key-value layout
 * - Multi-format input: Auto-convert JSON, YAML, TOML, CSV to DX
 * - Cache generation: .dx/cache/{filename}.human and .machine files
 * - Forge daemon status bar and commands
 * - Phantom Mode: Hide shadow .md files from file explorer
 * - Binary Cache: Auto-generate .dxb and .llm cache files
 * - Hologram View: Live preview for .dxm files
 * - Token Counter: Display token efficiency in status bar
 * 
 * Requirements: 1.1-1.7, 3.2-3.4, 5.1-5.6, 6.1-6.6, 7.1-7.6, 11.3-11.6
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { loadDxCore, DxCore } from './dxCore';
import { DxDocumentManager, DocumentManagerConfig } from './dxDocumentManager';
import { DxLensFileSystem } from './dxLensFileSystem';
import { DxsLensFileSystem } from './dxsLensFileSystem';
import { DX_LENS_SCHEME, DXS_LENS_SCHEME, isExactlyDxFile, isDxsFile, getLensUri, getDxsLensUri } from './utils';
import { detectFormat, isSourceFormat, DetectedFormat } from './formatDetector';
import { convertJsonToDocument, convertYamlToDocument, convertTomlToDocument, convertCsvToDocument } from './converters';
import { serializeToLlm, parseHuman } from './humanParser';
import { formatDocument } from './humanFormatter';
import { getForgeClient, ForgeStatusBar, registerForgeCommands } from './forge';
import { initializeDxCheck, disposeDxCheck } from './check';
import { registerHoverProvider } from './hoverProvider';
import { registerDiagnosticsProvider, disposeDiagnosticsProvider } from './diagnosticsProvider';
import { registerStyleHoverProvider } from './style/styleHoverProvider';
import { registerCSSMiniViewerCommand } from './style/cssMiniViewer';
import { initializeOutputMapping } from './style/outputMapping';
import { registerInlineDecorationProvider } from './style/inlineDecorationProvider';
import { PhantomModeManager, registerPhantomModeCommands } from './phantomMode';
import { BinaryCacheManager, setupCacheOnSave, registerCacheCommands } from './binaryCache';
import { HologramViewProvider, registerHologramView } from './hologramView';
import { TokenCounterStatusBar, registerTokenCounterCommands } from './tokenCounter';
import { UniversalTokenCounterStatusBar, registerUniversalTokenCounterCommands } from './universalTokenCounter';
import { registerAssetPickerCommands } from './assets/assetCommands';
import { FormatViewStatusBar, registerFormatViewCommands } from './formatViewProvider';
import { ExportConverterStatusBar, registerExportConverterCommands } from './exportConverter';
import {
    TemplateRegistry,
    TemplatePicker,
    ParameterInput,
    GeneratorTriggerProvider,
    GeneratorStatusBar,
    registerGeneratorCommands,
    registerGeneratorHoverProvider,
    registerStatusBarCommands,
    GeneratorTreeDataProvider,
    registerGeneratorPanelCommands,
    registerGeneratorCodeActions,
} from './generator';
import {
    DrivenTreeDataProvider,
    DrivenStatusBar,
    registerDrivenCommands,
} from './driven';
import {
    DcpTreeDataProvider,
    registerDcpCommands,
} from './dcp';
import {
    registerWwwCommands,
    registerWwwContextMenus,
    registerWwwPanel,
    WwwTreeDataProvider,
} from './www';

/**
 * Extension context holding all components
 */
interface ExtensionContext {
    dxCore: DxCore;
    documentManager: DxDocumentManager;
    lensFileSystem: DxLensFileSystem;
    statusBarItem: vscode.StatusBarItem;
    forgeStatusBar?: ForgeStatusBar;
    // Holographic Git components
    phantomModeManager?: PhantomModeManager;
    binaryCacheManager?: BinaryCacheManager;
    hologramViewProvider?: HologramViewProvider;
    tokenCounterStatusBar?: TokenCounterStatusBar;
    // Universal token counter (for all files)
    universalTokenCounter?: UniversalTokenCounterStatusBar;
    // Format view and export components
    formatViewStatusBar?: FormatViewStatusBar;
    exportConverterStatusBar?: ExportConverterStatusBar;
    // Generator components
    generatorRegistry?: TemplateRegistry;
    generatorStatusBar?: GeneratorStatusBar;
    generatorTreeDataProvider?: GeneratorTreeDataProvider;
    // Driven components
    drivenTreeDataProvider?: DrivenTreeDataProvider;
    drivenStatusBar?: DrivenStatusBar;
    // DCP components
    dcpTreeDataProvider?: DcpTreeDataProvider;
    // WWW components
    wwwTreeDataProvider?: WwwTreeDataProvider;
}

let extensionContext: ExtensionContext | undefined;

/**
 * Activate the extension
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    console.log('DX Serializer: Activating extension...');

    // Create output channel for debugging
    const outputChannel = vscode.window.createOutputChannel('DX Serializer');
    outputChannel.appendLine('DX Serializer: Starting activation...');

    // FIRST: Initialize Universal Token Counter immediately (before anything else can fail)
    // This ensures the token counter is always visible in the status bar
    try {
        outputChannel.appendLine('Initializing Universal Token Counter (early)...');
        const universalTokenCounter = new UniversalTokenCounterStatusBar();
        context.subscriptions.push(universalTokenCounter);
        registerUniversalTokenCounterCommands(context, universalTokenCounter);
        outputChannel.appendLine('Universal Token Counter initialized successfully');
    } catch (error) {
        outputChannel.appendLine(`Universal Token Counter failed: ${error}`);
        console.error('DX: Universal Token Counter initialization failed:', error);
    }

    // Initialize Format View and Export Converter status bars
    try {
        outputChannel.appendLine('Initializing Format View and Export Converter...');
        const formatViewStatusBar = new FormatViewStatusBar();
        context.subscriptions.push(formatViewStatusBar);
        registerFormatViewCommands(context, formatViewStatusBar);
        
        const exportConverterStatusBar = new ExportConverterStatusBar();
        context.subscriptions.push(exportConverterStatusBar);
        registerExportConverterCommands(context, exportConverterStatusBar);
        outputChannel.appendLine('Format View and Export Converter initialized');
    } catch (error) {
        outputChannel.appendLine(`Format View/Export Converter failed: ${error}`);
        console.error('DX: Format View/Export Converter initialization failed:', error);
    }

    try {
        // 1. Load configuration
        outputChannel.appendLine('Step 1: Loading configuration...');
        const config = loadConfiguration();
        outputChannel.appendLine('Step 1: Configuration loaded successfully');

        // 2. Load WASM core (with fallback)
        outputChannel.appendLine('Step 2: Loading DX core...');
        let dxCore: DxCore;
        try {
            dxCore = await loadDxCore(context.extensionPath, config.indentSize, config.keyPadding);
            outputChannel.appendLine(`Step 2: Using ${dxCore.isWasm ? 'WASM' : 'TypeScript fallback'} core`);
        } catch (coreError) {
            outputChannel.appendLine(`Step 2: WASM load failed, using fallback: ${coreError}`);
            // Import fallback directly
            const { createFallbackCore } = require('./dxCore');
            dxCore = createFallbackCore(config.indentSize, config.keyPadding);
        }
        console.log(`DX Serializer: Using ${dxCore.isWasm ? 'WASM' : 'TypeScript fallback'} core`);

        // 3. Initialize DocumentManager
        outputChannel.appendLine('Step 3: Initializing DocumentManager...');
        const documentManager = new DxDocumentManager(dxCore, config);
        context.subscriptions.push(documentManager);
        outputChannel.appendLine('Step 3: DocumentManager initialized');

        // 4. Initialize LensFileSystem
        outputChannel.appendLine('Step 4: Initializing LensFileSystem...');
        const lensFileSystem = new DxLensFileSystem(dxCore, documentManager);
        context.subscriptions.push(lensFileSystem);
        outputChannel.appendLine('Step 4: LensFileSystem initialized');

        // =====================================================================
        // VIRTUAL FILE SYSTEM REGISTRATION - COMMENTED OUT (2026 Architecture)
        // =====================================================================
        // New architecture: Human format on disk, LLM in .dx folder
        // Virtual FS no longer needed - keeping code for reference
        // =====================================================================
        
        /*
        // 5. Register FileSystemProvider for 'dxlens' scheme
        outputChannel.appendLine('Step 5: Registering FileSystemProvider...');
        context.subscriptions.push(
            vscode.workspace.registerFileSystemProvider(DX_LENS_SCHEME, lensFileSystem, {
                isCaseSensitive: false,
                isReadonly: false,
            })
        );
        outputChannel.appendLine('Step 5: FileSystemProvider registered');

        // 5b. Register FileSystemProvider for 'dxslens' scheme (.sr files)
        outputChannel.appendLine('Step 5b: Registering SR FileSystemProvider...');
        const dxsLensFileSystem = new DxsLensFileSystem();
        context.subscriptions.push(dxsLensFileSystem);
        context.subscriptions.push(
            vscode.workspace.registerFileSystemProvider(DXS_LENS_SCHEME, dxsLensFileSystem, {
                isCaseSensitive: false,
                isReadonly: false,
            })
        );
        outputChannel.appendLine('Step 5b: SR FileSystemProvider registered');
        */

        // 6. Create status bar item
        outputChannel.appendLine('Step 6: Creating status bar...');
        const statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        statusBarItem.command = 'dx.showDenseView';
        context.subscriptions.push(statusBarItem);
        outputChannel.appendLine('Step 6: Status bar created');

        // Store extension context
        extensionContext = {
            dxCore,
            documentManager,
            lensFileSystem,
            statusBarItem,
        };

        // 7. Set up file watcher for external changes
        outputChannel.appendLine('Step 7: Setting up file watcher...');
        setupFileWatcher(context, documentManager);
        outputChannel.appendLine('Step 7: File watcher set up');

        // 8. Register commands
        outputChannel.appendLine('Step 8: Registering commands...');
        registerCommands(context, documentManager, statusBarItem);
        outputChannel.appendLine('Step 8: Commands registered');

        // 9. Set up auto-redirect from file:// to dxlens:// for .dx files
        outputChannel.appendLine('Step 9: Setting up auto-redirect...');
        setupAutoRedirect(context, dxCore);
        outputChannel.appendLine('Step 9: Auto-redirect set up');

        // 10. Set up configuration change listener
        outputChannel.appendLine('Step 10: Setting up configuration listener...');
        setupConfigurationListener(context, documentManager);
        outputChannel.appendLine('Step 10: Configuration listener set up');

        // 11. Set up active editor change listener for status bar
        setupActiveEditorListener(context, documentManager, statusBarItem);

        // 12. Register DocumentFormattingEditProvider for safe format-on-save
        registerFormattingProvider(context, dxCore);

        // 12b. Register Section Link Providers for clickable [section] headers
        outputChannel.appendLine('Step 12b: Registering section link providers...');
        registerSectionLinkProviders(context);
        outputChannel.appendLine('Step 12b: Section link providers registered');

        // 13. Initialize Forge integration (non-critical)
        try {
            await initializeForgeIntegration(context);
        } catch (error) {
            console.warn('DX: Forge integration failed (non-critical):', error);
        }

        // 14. Initialize dx-check linting (non-critical)
        try {
            await initializeDxCheck(context);
        } catch (error) {
            console.warn('DX: dx-check initialization failed (non-critical):', error);
        }

        // 15. Register hover provider (Requirements: 5.6, 7.2)
        try {
            registerHoverProvider(context);
        } catch (error) {
            console.warn('DX: Hover provider registration failed:', error);
        }

        // 16. Register diagnostics provider (Requirements: 5.8, 10.6)
        try {
            registerDiagnosticsProvider(context);
        } catch (error) {
            console.warn('DX: Diagnostics provider registration failed:', error);
        }

        // 17. Register style hover provider for dx-style classnames (Requirements: 2.1, 2.2, 2.3)
        try {
            registerStyleHoverProvider(context);
        } catch (error) {
            console.warn('DX: Style hover provider registration failed:', error);
        }

        // 18. Initialize output mapping and CSS mini viewer (Requirements: 3.1-3.10)
        try {
            await initializeOutputMapping(context);
            registerCSSMiniViewerCommand(context);
        } catch (error) {
            console.warn('DX: Output mapping initialization failed:', error);
        }

        // 19. Register inline decoration provider for grouped classnames (Requirements: 5.1-5.6)
        try {
            registerInlineDecorationProvider(context);
        } catch (error) {
            console.warn('DX: Inline decoration provider registration failed:', error);
        }

        // 20. Initialize Holographic Git components (Requirements: 5.1-5.6, 6.1-6.6, 7.1-7.6)
        try {
            await initializeHolographicGit(context);
        } catch (error) {
            console.warn('DX: Holographic Git initialization failed:', error);
        }

        // 21. Register asset picker placeholder commands (Requirements: 8.1-8.6)
        try {
            registerAssetPickerCommands(context);
        } catch (error) {
            console.warn('DX: Asset picker commands registration failed:', error);
        }

        // 22. Initialize Generator integration (Requirements: 2.1-2.6, 10.4)
        try {
            await initializeGenerator(context);
        } catch (error) {
            console.warn('DX: Generator initialization failed:', error);
        }

        // 23. Initialize Driven integration (Requirements: 9.1-9.10)
        try {
            await initializeDriven(context);
        } catch (error) {
            console.warn('DX: Driven initialization failed:', error);
        }

        // 24. Initialize DCP integration (Requirements: 11.1-11.10)
        try {
            await initializeDcp(context);
        } catch (error) {
            console.warn('DX: DCP initialization failed:', error);
        }

        // 25. Initialize WWW integration (Requirements: 8.1-8.5)
        try {
            await initializeWww(context);
        } catch (error) {
            console.warn('DX: WWW initialization failed:', error);
        }

        outputChannel.appendLine('DX: Extension activated successfully!');
        console.log('DX: Extension activated successfully (V3 format + Forge + Check + Holographic Git + Generator + Driven + DCP + WWW)');

    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        const errorStack = error instanceof Error ? error.stack : '';
        console.error('DX Serializer: Activation failed:', error);
        vscode.window.showErrorMessage(`DX Serializer: Failed to activate: ${errorMsg}`);

        // Log to output channel for debugging
        const outputChannel = vscode.window.createOutputChannel('DX Serializer');
        outputChannel.appendLine(`ACTIVATION FAILED: ${errorMsg}`);
        outputChannel.appendLine(`Stack: ${errorStack}`);
        outputChannel.show();
    }
}

/**
 * Deactivate the extension
 */
export async function deactivate(): Promise<void> {
    console.log('DX: Deactivating extension...');

    // Stop dx-check language server
    await disposeDxCheck();

    // Dispose diagnostics provider
    disposeDiagnosticsProvider();

    // Disconnect from Forge daemon
    const forgeClient = getForgeClient();
    forgeClient.disconnect();

    extensionContext = undefined;
}

/**
 * Load configuration from VS Code settings
 */
function loadConfiguration(): DocumentManagerConfig {
    const config = vscode.workspace.getConfiguration('dx');
    return {
        validateBeforeSave: config.get<boolean>('validateBeforeSave', true),
        autoSaveGracePeriod: config.get<number>('autoSaveGracePeriod', 2000),
        indentSize: config.get<number>('indentSize', 2) as 2 | 4,
        keyPadding: config.get<number>('keyPadding', 20),
        formatOnSave: config.get<boolean>('formatOnSave', true),
        formatDelayAfterAutoSave: config.get<number>('formatDelayAfterAutoSave', 1500),
    };
}

/**
 * Set up file watcher for external changes
 */
function setupFileWatcher(
    context: vscode.ExtensionContext,
    documentManager: DxDocumentManager
): void {
    // Watch for .sr files and files named 'dx' (no extension)
    const srWatcher = vscode.workspace.createFileSystemWatcher('**/*.sr');
    const dxNoExtWatcher = vscode.workspace.createFileSystemWatcher('**/dx');

    // Handle external file changes for .sr files
    context.subscriptions.push(
        srWatcher.onDidChange(async (uri) => {
            if (isExactlyDxFile(uri) && !documentManager.isWriting(uri)) {
                await documentManager.handleExternalChange(uri);
            }
        })
    );

    // Handle external file changes for dx files (no extension)
    context.subscriptions.push(
        dxNoExtWatcher.onDidChange(async (uri) => {
            if (isExactlyDxFile(uri) && !documentManager.isWriting(uri)) {
                await documentManager.handleExternalChange(uri);
            }
        })
    );

    // Handle file deletion for .sr files
    context.subscriptions.push(
        srWatcher.onDidDelete((uri) => {
            if (isExactlyDxFile(uri)) {
                documentManager.handleFileDeleted(uri);
            }
        })
    );

    // Handle file deletion for dx files (no extension)
    context.subscriptions.push(
        dxNoExtWatcher.onDidDelete((uri) => {
            if (isExactlyDxFile(uri)) {
                documentManager.handleFileDeleted(uri);
            }
        })
    );

    context.subscriptions.push(srWatcher);
    context.subscriptions.push(dxNoExtWatcher);
}


/**
 * Register extension commands
 */
function registerCommands(
    context: vscode.ExtensionContext,
    documentManager: DxDocumentManager,
    _statusBarItem: vscode.StatusBarItem
): void {
    // DX: Refresh from Disk
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.refreshFromDisk', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DX: No active editor');
                return;
            }

            const uri = editor.document.uri;
            if (!isExactlyDxFile(uri)) {
                vscode.window.showWarningMessage('DX: Not a .dx file');
                return;
            }

            try {
                const newContent = await documentManager.forceRefresh(uri);

                // Update the editor with new content
                const edit = new vscode.WorkspaceEdit();
                const fullRange = new vscode.Range(
                    editor.document.positionAt(0),
                    editor.document.positionAt(editor.document.getText().length)
                );
                edit.replace(uri, fullRange, newContent);
                await vscode.workspace.applyEdit(edit);

                vscode.window.showInformationMessage('DX: Refreshed from disk');
            } catch (error) {
                vscode.window.showErrorMessage(`DX: Refresh failed: ${error}`);
            }
        })
    );

    // DX: Force Save
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.forceSave', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DX: No active editor');
                return;
            }

            const uri = editor.document.uri;
            if (!isExactlyDxFile(uri)) {
                vscode.window.showWarningMessage('DX: Not a .dx file');
                return;
            }

            await documentManager.forceSave(uri);
        })
    );

    // DX: Set Color Theme
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.setColorTheme', async () => {
            // Open the theme picker with DX themes pre-filtered
            await vscode.commands.executeCommand('workbench.action.selectTheme');
        })
    );

    // DX: Show Dense View
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.showDenseView', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DX: No active editor');
                return;
            }

            const uri = editor.document.uri;
            if (!isExactlyDxFile(uri)) {
                vscode.window.showWarningMessage('DX: Not a .dx file');
                return;
            }

            const denseContent = documentManager.getDenseContent(uri);
            if (!denseContent) {
                vscode.window.showWarningMessage('DX: No content available');
                return;
            }

            // Open a new untitled document with the dense content
            const doc = await vscode.workspace.openTextDocument({
                content: denseContent,
                language: 'dx',
            });
            await vscode.window.showTextDocument(doc, {
                viewColumn: vscode.ViewColumn.Beside,
                preview: true,
                preserveFocus: true,
            });
        })
    );

    // Markdown: Convert to Human Format (Requirements: 5.4, 6.4)
    context.subscriptions.push(
        vscode.commands.registerCommand('markdown.convertToHuman', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DXM: No active editor');
                return;
            }

            const uri = editor.document.uri;
            if (!isExactlyDxFile(uri)) {
                vscode.window.showWarningMessage('DXM: Not a .dx file');
                return;
            }

            await documentManager.convertFormat(uri, 'human');
        })
    );

    // DXM: Convert to LLM Format (Requirements: 5.4, 6.5)
    context.subscriptions.push(
        vscode.commands.registerCommand('dxm.convertToLlm', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DXM: No active editor');
                return;
            }

            const uri = editor.document.uri;
            if (!isExactlyDxFile(uri)) {
                vscode.window.showWarningMessage('DXM: Not a .dx file');
                return;
            }

            await documentManager.convertFormat(uri, 'llm');
        })
    );

    // DXM: Toggle Table Style (Requirements: 5.5)
    context.subscriptions.push(
        vscode.commands.registerCommand('dxm.toggleTableStyle', async () => {
            const config = vscode.workspace.getConfiguration('dx');
            const currentStyle = config.get<string>('tableStyle', 'unicode');
            const newStyle = currentStyle === 'unicode' ? 'ascii' : 'unicode';

            await config.update('tableStyle', newStyle, vscode.ConfigurationTarget.Global);
            vscode.window.showInformationMessage(`DXM: Table style set to ${newStyle.toUpperCase()}`);
        })
    );
}


/**
 * Set up auto-redirect from file:// to dxlens:// for .dx files
 * Also handles format detection and conversion for non-LLM formats
 * 
 * When a user opens a .dx file directly (file://), we:
 * 1. Detect the format (JSON, YAML, TOML, CSV, LLM, Human V3)
 * 2. Convert non-LLM formats to LLM format
 * 3. Redirect to dxlens:// scheme for human display
 * 
 * Requirements: 1.1-1.7, 3.2-3.4
 */
function setupAutoRedirect(context: vscode.ExtensionContext, dxCore: DxCore): void {
    // Track URIs we're currently redirecting to avoid loops
    const redirectingUris = new Set<string>();

    console.log('DX Serializer: Setting up auto-redirect for .dx and .sr files');

    // Listen for document opens
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(async (document) => {
            console.log(`DX Serializer: Document opened - scheme: ${document.uri.scheme}, path: ${document.uri.fsPath}`);

            // Only redirect file:// scheme files
            if (document.uri.scheme !== 'file') {
                console.log('DX Serializer: Skipping - not file:// scheme');
                return;
            }

            const isDxFile = isExactlyDxFile(document.uri);
            const isDxsFileCheck = isDxsFile(document.uri);
            console.log(`DX Serializer: isExactlyDxFile: ${isDxFile}, isDxsFile: ${isDxsFileCheck}`);

            if (!isDxFile && !isDxsFileCheck) {
                return;
            }

            const uriString = document.uri.toString();

            // Avoid redirect loops
            if (redirectingUris.has(uriString)) {
                console.log('DX Serializer: Skipping - already redirecting');
                return;
            }

            redirectingUris.add(uriString);
            console.log(`DX Serializer: Starting redirect for ${uriString}`);

            try {
                // For .sr files, redirect to dxslens:// to show human format
                if (isDxsFileCheck) {
                    const lensUri = getDxsLensUri(document.uri);
                    console.log(`DX Serializer: Redirecting .sr to ${lensUri.toString()}`);

                    setTimeout(async () => {
                        try {
                            // Close all editors showing the file:// version
                            for (const tabGroup of vscode.window.tabGroups.all) {
                                for (const tab of tabGroup.tabs) {
                                    if (tab.input instanceof vscode.TabInputText) {
                                        if (tab.input.uri.toString() === uriString) {
                                            await vscode.window.tabGroups.close(tab);
                                        }
                                    }
                                }
                            }

                            // Open with dxslens:// scheme
                            const doc = await vscode.workspace.openTextDocument(lensUri);
                            await vscode.window.showTextDocument(doc);
                            console.log('DX Serializer: .sr redirect completed successfully');
                        } catch (error) {
                            console.error('DX Serializer: .sr auto-redirect failed:', error);
                        } finally {
                            redirectingUris.delete(uriString);
                        }
                    }, 50);
                    return;
                }

                // For .dx files, detect format and convert if needed
                const content = document.getText();
                const detection = detectFormat(content);
                console.log(`DX Serializer: Detected format: ${detection.format}`);

                if (isSourceFormat(detection.format)) {
                    // Convert source format to LLM format
                    await convertAndSaveFile(document.uri, content, detection.format, dxCore);
                }

                // Close the file:// document and open with dxlens://
                const lensUri = getLensUri(document.uri);
                console.log(`DX Serializer: Redirecting to ${lensUri.toString()}`);

                // Use a small delay to avoid race conditions
                setTimeout(async () => {
                    try {
                        // Close all editors showing the file:// version
                        for (const tabGroup of vscode.window.tabGroups.all) {
                            for (const tab of tabGroup.tabs) {
                                if (tab.input instanceof vscode.TabInputText) {
                                    if (tab.input.uri.toString() === uriString) {
                                        await vscode.window.tabGroups.close(tab);
                                    }
                                }
                            }
                        }

                        // Open with dxlens:// scheme
                        const doc = await vscode.workspace.openTextDocument(lensUri);
                        await vscode.window.showTextDocument(doc);
                        console.log('DX Serializer: Redirect completed successfully');
                    } catch (error) {
                        console.error('DX Serializer: Auto-redirect failed:', error);
                    } finally {
                        redirectingUris.delete(uriString);
                    }
                }, 50);
            } catch (error) {
                redirectingUris.delete(uriString);
                console.error('DX Serializer: Auto-redirect error:', error);
            }
        })
    );
}

/**
 * Convert a source format file to LLM format and save
 * Requirements: 1.1-1.4
 */
async function convertAndSaveFile(
    uri: vscode.Uri,
    content: string,
    format: DetectedFormat,
    _dxCore: DxCore
): Promise<void> {
    try {
        let conversionResult;

        switch (format) {
            case 'json':
                conversionResult = convertJsonToDocument(content);
                break;
            case 'yaml':
                conversionResult = convertYamlToDocument(content);
                break;
            case 'toml':
                conversionResult = convertTomlToDocument(content);
                break;
            case 'csv':
                conversionResult = convertCsvToDocument(content);
                break;
            default:
                return;
        }

        if (!conversionResult.success || !conversionResult.document) {
            console.warn(`DX Serializer: Failed to convert ${format}: ${conversionResult.error}`);
            return;
        }

        // Serialize to LLM format
        const llmContent = serializeToLlm(conversionResult.document);

        // Write back to file
        const fs = await import('fs');
        await fs.promises.writeFile(uri.fsPath, llmContent, 'utf-8');

        vscode.window.showInformationMessage(
            `DX: Converted ${format.toUpperCase()} to LLM format`
        );
    } catch (error) {
        console.error(`DX Serializer: Conversion failed:`, error);
    }
}

/**
 * Set up configuration change listener
 */
function setupConfigurationListener(
    context: vscode.ExtensionContext,
    documentManager: DxDocumentManager
): void {
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((event) => {
            if (event.affectsConfiguration('dx')) {
                const config = loadConfiguration();
                documentManager.updateConfig(config);
                console.log('DX Serializer: Configuration updated');
            }
        })
    );
}

/**
 * Set up active editor change listener for status bar updates
 */
function setupActiveEditorListener(
    context: vscode.ExtensionContext,
    documentManager: DxDocumentManager,
    statusBarItem: vscode.StatusBarItem
): void {
    // Update status bar when active editor changes
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor((editor) => {
            updateStatusBar(editor, documentManager, statusBarItem);
        })
    );

    // Update status bar when document changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument((event) => {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document === event.document) {
                // Notify document manager of content change
                if (isExactlyDxFile(event.document.uri)) {
                    documentManager.handleContentChange(
                        event.document.uri,
                        event.document.getText()
                    );
                }
                updateStatusBar(editor, documentManager, statusBarItem);
            }
        })
    );

    // Initial status bar update
    updateStatusBar(vscode.window.activeTextEditor, documentManager, statusBarItem);
}

/**
 * Update the status bar based on current editor state
 */
function updateStatusBar(
    editor: vscode.TextEditor | undefined,
    documentManager: DxDocumentManager,
    statusBarItem: vscode.StatusBarItem
): void {
    if (!editor || !isExactlyDxFile(editor.document.uri)) {
        statusBarItem.hide();
        return;
    }

    const state = documentManager.getState(editor.document.uri);

    if (!state) {
        statusBarItem.text = '$(file-code) DX';
        statusBarItem.tooltip = 'DX Serializer active';
        statusBarItem.backgroundColor = undefined;
        statusBarItem.show();
        return;
    }

    if (state.isValid) {
        statusBarItem.text = '$(check) DX';
        statusBarItem.tooltip = 'DX: Valid - Click to show dense format';
        statusBarItem.backgroundColor = undefined;
    } else {
        statusBarItem.text = '$(warning) DX';
        statusBarItem.tooltip = `DX: ${state.lastError || 'Invalid'} - Click to show dense format`;
        statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    }

    statusBarItem.show();
}

/**
 * Format a DX document and return TextEdit array
 * Shared logic for both formatting provider and willSave handler
 * 
 * Handles both LLM format and Human V3 format:
 * - LLM format: Parse with parseLlm, then format to Human V3
 * - Human V3 format: Parse with parseHumanV3, then re-format for alignment
 */
function formatDxDocument(document: vscode.TextDocument): vscode.TextEdit[] | null {
    try {
        const content = document.getText();

        // Skip empty documents
        if (!content.trim()) {
            return null;
        }

        // Detect the format
        const detection = detectFormat(content);
        let dxDocument: import('./llmParser').DxDocument | null = null;

        if (detection.format === 'llm') {
            // Parse LLM format
            const { parseLlm } = require('./llmParser');
            const parseResult = parseLlm(content);
            if (!parseResult.success || !parseResult.document) {
                console.log('DX Format: LLM parse failed, skipping format');
                return null;
            }
            dxDocument = parseResult.document;
        } else if (detection.format === 'human-v3') {
            // Parse Human format
            const parseResult = parseHuman(content);
            if (!parseResult.success || !parseResult.document) {
                console.log('DX Format: Human parse failed, skipping format');
                return null;
            }
            dxDocument = parseResult.document;
        } else {
            // Unknown or other format - try Human as fallback
            const parseResult = parseHuman(content);
            if (!parseResult.success || !parseResult.document) {
                console.log(`DX Format: Format '${detection.format}' not supported for formatting`);
                return null;
            }
            dxDocument = parseResult.document;
        }

        // Safety check (should never happen due to early returns above)
        if (!dxDocument) {
            return null;
        }

        // Get config from settings
        const config = vscode.workspace.getConfiguration('dx');
        const keyPadding = config.get<number>('keyPadding', 20);

        // Format the document with proper alignment
        const formattedContent = formatDocument(dxDocument);

        // Skip if content is unchanged
        if (formattedContent === content) {
            return null;
        }

        // Return a single edit that replaces the entire document
        const fullRange = new vscode.Range(
            document.positionAt(0),
            document.positionAt(content.length)
        );

        return [vscode.TextEdit.replace(fullRange, formattedContent)];
    } catch (error) {
        console.error('DX Format: Error during formatting:', error);
        return null;
    }
}

/**
 * Register DocumentFormattingEditProvider for safe format-on-save
 * 
 * This is the proper VS Code way to handle formatting:
 * 1. Parse the human content
 * 2. Convert to LLM format (normalizes the data)
 * 3. Convert back to human format (applies proper alignment)
 * 4. Return TextEdit array for VS Code to apply safely
 * 
 * Also registers willSaveTextDocument handler for auto-save support
 */
function registerFormattingProvider(
    context: vscode.ExtensionContext,
    dxCore: DxCore
): void {
    // Create the formatting provider
    const formattingProvider: vscode.DocumentFormattingEditProvider = {
        provideDocumentFormattingEdits(
            document: vscode.TextDocument,
            _options: vscode.FormattingOptions,
            _token: vscode.CancellationToken
        ): vscode.TextEdit[] | null {
            return formatDxDocument(document);
        }
    };

    // Register for both dxlens scheme and file scheme
    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(
            { scheme: DX_LENS_SCHEME, language: 'dx' },
            formattingProvider
        )
    );

    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(
            { scheme: 'file', language: 'dx' },
            formattingProvider
        )
    );

    // willSaveTextDocument handler for auto-format on save
    // Re-enabled after fixing format detection to handle both LLM and Human V3 formats
    context.subscriptions.push(
        vscode.workspace.onWillSaveTextDocument((event) => {
            // Only format .dx files
            if (!isExactlyDxFile(event.document.uri)) {
                return;
            }

            // Check if formatOnSave is enabled
            const config = vscode.workspace.getConfiguration('dx');
            const formatOnSave = config.get<boolean>('formatOnSave', true);
            if (!formatOnSave) {
                return;
            }

            // Wait for formatting edits and apply them before save
            event.waitUntil(
                Promise.resolve().then(() => {
                    const edits = formatDxDocument(event.document);
                    return edits || [];
                })
            );
        })
    );

    // Register the "DX: Format Document" command
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.formatDocument', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('DX: No active editor');
                return;
            }

            if (!isExactlyDxFile(editor.document.uri)) {
                vscode.window.showWarningMessage('DX: Not a .dx file');
                return;
            }

            // Use VS Code's built-in format command which will use our provider
            await vscode.commands.executeCommand('editor.action.formatDocument');
        })
    );

    console.log('DX Serializer: Formatting provider registered (with auto-save support)');
}


/**
 * Initialize Forge daemon integration
 */
async function initializeForgeIntegration(context: vscode.ExtensionContext): Promise<void> {
    try {
        // Register Forge commands
        registerForgeCommands(context);

        // Get Forge client
        const forgeClient = getForgeClient();

        // Create status bar
        const forgeStatusBar = new ForgeStatusBar(forgeClient);
        context.subscriptions.push({ dispose: () => forgeStatusBar.dispose() });

        // Store in extension context
        if (extensionContext) {
            extensionContext.forgeStatusBar = forgeStatusBar;
        }

        // Auto-connect if enabled - DISABLED to prevent loops
        // Users should manually start the daemon with "DX: Start Forge Daemon"
        const config = vscode.workspace.getConfiguration('dx.forge');
        if (config.get('autoConnect', true)) {
            // Just show disconnected state, don't try to connect automatically
            forgeStatusBar.showDisconnected();
            console.log('DX: Forge daemon not running. Use "DX: Start Forge Daemon" to start.');
        }

        // Set up file change notifications to Forge
        setupForgeFileNotifications(context, forgeClient);

        console.log('DX: Forge integration initialized');
    } catch (error) {
        console.error('DX: Failed to initialize Forge integration:', error);
    }
}

/**
 * Set up file change notifications to Forge daemon
 */
function setupForgeFileNotifications(
    context: vscode.ExtensionContext,
    forgeClient: import('./forge').ForgeClient
): void {
    // Watch for file saves
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(async (document) => {
            if (forgeClient.isConnected()) {
                const relativePath = vscode.workspace.asRelativePath(document.uri);
                await forgeClient.notifyFileChange(relativePath, 'modified');
            }
        })
    );

    // Watch for file creates
    const watcher = vscode.workspace.createFileSystemWatcher('**/*');

    context.subscriptions.push(
        watcher.onDidCreate(async (uri) => {
            if (forgeClient.isConnected()) {
                const relativePath = vscode.workspace.asRelativePath(uri);
                await forgeClient.notifyFileChange(relativePath, 'created');
            }
        })
    );

    context.subscriptions.push(
        watcher.onDidDelete(async (uri) => {
            if (forgeClient.isConnected()) {
                const relativePath = vscode.workspace.asRelativePath(uri);
                await forgeClient.notifyFileChange(relativePath, 'deleted');
            }
        })
    );

    context.subscriptions.push(watcher);
}

/**
 * Initialize Holographic Git components
 * 
 * Sets up:
 * - PhantomModeManager: Hides shadow .md files
 * - BinaryCacheManager: Generates .dxb and .llm cache files
 * - HologramViewProvider: Live preview for .dxm files
 * - TokenCounterStatusBar: Token efficiency display
 * 
 * Requirements: 5.1-5.6, 6.1-6.6, 7.1-7.6, 11.3-11.6
 */
async function initializeHolographicGit(context: vscode.ExtensionContext): Promise<void> {
    try {
        // Get workspace root
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            console.log('DX: No workspace folder, skipping Holographic Git initialization');
            return;
        }
        const workspaceRoot = workspaceFolders[0].uri.fsPath;

        // 1. Initialize PhantomModeManager (Requirements: 5.1-5.6)
        const phantomModeManager = new PhantomModeManager(context);
        context.subscriptions.push(phantomModeManager);
        registerPhantomModeCommands(context, phantomModeManager);

        // 2. Initialize BinaryCacheManager (Requirements: 7.1-7.6)
        const binaryCacheManager = new BinaryCacheManager(workspaceRoot);
        await binaryCacheManager.initialize();
        context.subscriptions.push(binaryCacheManager);
        setupCacheOnSave(context, binaryCacheManager);
        registerCacheCommands(context, binaryCacheManager);

        // 3. Initialize HologramViewProvider (Requirements: 6.1-6.6)
        const hologramViewProvider = registerHologramView(context, binaryCacheManager);

        // 4. Initialize TokenCounterStatusBar (Requirements: 6.3, 6.4)
        const tokenCounterStatusBar = new TokenCounterStatusBar();
        context.subscriptions.push(tokenCounterStatusBar);
        registerTokenCounterCommands(context, tokenCounterStatusBar);

        // Store in extension context
        if (extensionContext) {
            extensionContext.phantomModeManager = phantomModeManager;
            extensionContext.binaryCacheManager = binaryCacheManager;
            extensionContext.hologramViewProvider = hologramViewProvider;
            extensionContext.tokenCounterStatusBar = tokenCounterStatusBar;
        }

        console.log('DX: Holographic Git components initialized');
    } catch (error) {
        console.error('DX: Failed to initialize Holographic Git:', error);
    }
}

/**
 * Initialize Generator integration
 * 
 * Sets up:
 * - TemplateRegistry: Template discovery and management
 * - TemplatePicker: Template selection UI
 * - ParameterInput: Parameter input dialogs
 * - GeneratorTriggerProvider: Trigger detection and execution
 * - GeneratorStatusBar: Token savings display
 * - GeneratorTreeDataProvider: Tree view for Generator panel
 * 
 * Requirements: 2.1-2.6, 10.1-10.10
 */
async function initializeGenerator(context: vscode.ExtensionContext): Promise<void> {
    try {
        // 1. Initialize TemplateRegistry
        const registry = new TemplateRegistry();
        await registry.initialize();

        // 2. Initialize ParameterInput
        const parameterInput = new ParameterInput();

        // 3. Initialize TemplatePicker
        const picker = new TemplatePicker(registry);

        // 4. Initialize GeneratorTriggerProvider
        const triggerProvider = new GeneratorTriggerProvider(registry, parameterInput);
        context.subscriptions.push(triggerProvider);

        // 5. Initialize GeneratorStatusBar
        const statusBar = new GeneratorStatusBar();
        context.subscriptions.push(statusBar);

        // 6. Register commands
        registerGeneratorCommands(context, registry, picker, parameterInput, triggerProvider);
        registerStatusBarCommands(context, statusBar);

        // 7. Register hover provider
        registerGeneratorHoverProvider(context, registry, triggerProvider);

        // 8. Register code action provider
        registerGeneratorCodeActions(context, registry, triggerProvider);

        // 9. Initialize TreeDataProvider for panel
        const treeDataProvider = new GeneratorTreeDataProvider(registry, statusBar);
        const treeView = vscode.window.createTreeView('dx.generatorView', {
            treeDataProvider,
            showCollapseAll: true,
        });
        context.subscriptions.push(treeView);

        // 10. Register panel commands
        registerGeneratorPanelCommands(context, treeDataProvider);

        // Store in extension context
        if (extensionContext) {
            extensionContext.generatorRegistry = registry;
            extensionContext.generatorStatusBar = statusBar;
            extensionContext.generatorTreeDataProvider = treeDataProvider;
        }

        console.log('DX: Generator integration initialized');
    } catch (error) {
        console.error('DX: Failed to initialize Generator:', error);
    }
}


/**
 * Initialize Driven integration
 * 
 * Sets up:
 * - DrivenTreeDataProvider: Tree view for Driven panel
 * - DrivenStatusBar: Sync status display
 * - Commands for driven operations
 * 
 * Requirements: 9.1-9.10
 */
async function initializeDriven(context: vscode.ExtensionContext): Promise<void> {
    try {
        // 1. Initialize TreeDataProvider
        const treeDataProvider = new DrivenTreeDataProvider();

        // 2. Register tree view
        const treeView = vscode.window.createTreeView('dx.drivenView', {
            treeDataProvider,
            showCollapseAll: true,
        });
        context.subscriptions.push(treeView);

        // 3. Initialize status bar
        const statusBar = new DrivenStatusBar();
        context.subscriptions.push(statusBar);

        // 4. Register commands
        registerDrivenCommands(context, treeDataProvider);

        // Store in extension context
        if (extensionContext) {
            extensionContext.drivenTreeDataProvider = treeDataProvider;
            extensionContext.drivenStatusBar = statusBar;
        }

        console.log('DX: Driven integration initialized');
    } catch (error) {
        console.error('DX: Failed to initialize Driven:', error);
    }
}

/**
 * Initialize DCP integration
 * 
 * Sets up:
 * - DcpTreeDataProvider: Tree view for DCP panel
 * - Commands for DCP operations
 * 
 * Requirements: 11.1-11.10
 */
async function initializeDcp(context: vscode.ExtensionContext): Promise<void> {
    try {
        // 1. Initialize TreeDataProvider
        const treeDataProvider = new DcpTreeDataProvider();

        // 2. Register tree view
        const treeView = vscode.window.createTreeView('dx.dcpView', {
            treeDataProvider,
            showCollapseAll: true,
        });
        context.subscriptions.push(treeView);

        // 3. Register commands
        registerDcpCommands(context, treeDataProvider);

        // Store in extension context
        if (extensionContext) {
            extensionContext.dcpTreeDataProvider = treeDataProvider;
        }

        console.log('DX: DCP integration initialized');
    } catch (error) {
        console.error('DX: Failed to initialize DCP:', error);
    }
}

/**
 * Initialize WWW integration
 * 
 * Sets up:
 * - WwwTreeDataProvider: Tree view for WWW panel
 * - Commands for www operations
 * - Context menu contributions
 * 
 * Requirements: 8.1-8.5
 */
async function initializeWww(context: vscode.ExtensionContext): Promise<void> {
    try {
        // 1. Register WWW commands
        registerWwwCommands(context);

        // 2. Register context menu contributions
        registerWwwContextMenus(context);

        // 3. Initialize TreeDataProvider and panel
        const treeDataProvider = registerWwwPanel(context);

        // Store in extension context
        if (extensionContext) {
            extensionContext.wwwTreeDataProvider = treeDataProvider;
        }

        console.log('DX: WWW integration initialized');
    } catch (error) {
        console.error('DX: Failed to initialize WWW:', error);
    }
}

// ============================================================================
// Section Link Providers - Makes [section] headers clickable
// ============================================================================

/**
 * Maps section names to their corresponding .dx/ folder paths
 */
const SECTION_FOLDER_MAP: Record<string, string> = {
    'forge': 'forge',
    'style': 'style',
    'ui': 'ui',
    'media': 'media',
    'icon': 'icon',
    'font': 'font',
    'driven': 'driven',
    'generator': 'generator',
    'scripts': 'cli',
    'dependencies': 'workspace',
    'js.dependencies': 'runtime',
    'python.dependencies': 'runtime',
    'rust.dependencies': 'runtime',
    'i18n': 'i18n',
    'i18n.locales': 'i18n',
    'i18n.ttses': 'i18n',
    'stack': 'workspace',
    'config': 'workspace',
    'workspace': 'workspace',
    'editors': 'extension',
    'auth': 'auth',
    'cache': 'cache',
    'test': 'test',
    'test-runner': 'test-runner',
    'templates': 'templates',
    'www': 'www',
    'serializer': 'serializer',
    'compatibility': 'compatibility',
    'benchmarks': 'benchmarks',
    'package-manager': 'package-manager',
};

/**
 * Provides clickable links for section headers in DX files
 */
class DxSectionLinkProvider implements vscode.DocumentLinkProvider {
    provideDocumentLinks(
        document: vscode.TextDocument,
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DocumentLink[]> {
        const links: vscode.DocumentLink[] = [];
        const text = document.getText();
        const lines = text.split('\n');

        // Pattern for human format section headers: [sectionName]
        const sectionPattern = /^\[([a-zA-Z0-9_.]+)\]/;
        
        // Pattern for DSR format objects: name[key=value,...]
        const dsrObjectPattern = /^([a-zA-Z0-9_.]+)\[/;

        for (let lineNum = 0; lineNum < lines.length; lineNum++) {
            const line = lines[lineNum];
            
            const sectionMatch = line.match(sectionPattern);
            if (sectionMatch) {
                const sectionName = sectionMatch[1];
                const folderName = this.getFolderForSection(sectionName);
                
                if (folderName) {
                    const startPos = new vscode.Position(lineNum, 1);
                    const endPos = new vscode.Position(lineNum, 1 + sectionName.length);
                    const range = new vscode.Range(startPos, endPos);
                    
                    const link = new vscode.DocumentLink(range);
                    link.tooltip = `Open .dx/${folderName}/ folder`;
                    links.push(link);
                }
                continue;
            }

            const dsrMatch = line.match(dsrObjectPattern);
            if (dsrMatch) {
                const objectName = dsrMatch[1];
                const folderName = this.getFolderForSection(objectName);
                
                if (folderName) {
                    const startPos = new vscode.Position(lineNum, 0);
                    const endPos = new vscode.Position(lineNum, objectName.length);
                    const range = new vscode.Range(startPos, endPos);
                    
                    const link = new vscode.DocumentLink(range);
                    link.tooltip = `Open .dx/${folderName}/ folder`;
                    links.push(link);
                }
            }
        }

        return links;
    }

    resolveDocumentLink(
        link: vscode.DocumentLink,
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DocumentLink> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return link;

        const document = editor.document;
        const sectionName = document.getText(link.range);
        const folderName = this.getFolderForSection(sectionName);

        if (folderName) {
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
            if (workspaceFolder) {
                const dxFolderPath = path.join(workspaceFolder.uri.fsPath, '.dx', folderName);
                link.target = vscode.Uri.file(dxFolderPath);
            }
        }

        return link;
    }

    private getFolderForSection(sectionName: string): string | undefined {
        if (SECTION_FOLDER_MAP[sectionName]) {
            return SECTION_FOLDER_MAP[sectionName];
        }

        const lower = sectionName.toLowerCase();
        if (SECTION_FOLDER_MAP[lower]) {
            return SECTION_FOLDER_MAP[lower];
        }

        if (sectionName.includes('.')) {
            const parent = sectionName.split('.')[0];
            if (SECTION_FOLDER_MAP[parent]) {
                return SECTION_FOLDER_MAP[parent];
            }
        }

        return sectionName;
    }
}

/**
 * CodeLens provider for section headers - shows folder link action
 */
class DxSectionCodeLensProvider implements vscode.CodeLensProvider {
    provideCodeLenses(
        document: vscode.TextDocument,
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.CodeLens[]> {
        const codeLenses: vscode.CodeLens[] = [];
        const text = document.getText();
        const lines = text.split('\n');

        const sectionPattern = /^\[([a-zA-Z0-9_.]+)\]/;
        const dsrObjectPattern = /^([a-zA-Z0-9_.]+)\[/;

        for (let lineNum = 0; lineNum < lines.length; lineNum++) {
            const line = lines[lineNum];
            
            let sectionName: string | null = null;
            
            const sectionMatch = line.match(sectionPattern);
            if (sectionMatch) {
                sectionName = sectionMatch[1];
            } else {
                const dsrMatch = line.match(dsrObjectPattern);
                if (dsrMatch) {
                    sectionName = dsrMatch[1];
                }
            }

            if (sectionName) {
                const folderName = this.getFolderForSection(sectionName);
                if (folderName) {
                    const range = new vscode.Range(lineNum, 0, lineNum, line.length);
                    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
                    
                    if (workspaceFolder) {
                        const dxFolderPath = path.join(workspaceFolder.uri.fsPath, '.dx', folderName);
                        
                        const codeLens = new vscode.CodeLens(range, {
                            title: `📁 .dx/${folderName}/`,
                            command: 'dx.openSectionFolder',
                            arguments: [dxFolderPath],
                        });
                        codeLenses.push(codeLens);
                    }
                }
            }
        }

        return codeLenses;
    }

    private getFolderForSection(sectionName: string): string | undefined {
        if (SECTION_FOLDER_MAP[sectionName]) {
            return SECTION_FOLDER_MAP[sectionName];
        }

        const lower = sectionName.toLowerCase();
        if (SECTION_FOLDER_MAP[lower]) {
            return SECTION_FOLDER_MAP[lower];
        }

        if (sectionName.includes('.')) {
            const parent = sectionName.split('.')[0];
            if (SECTION_FOLDER_MAP[parent]) {
                return SECTION_FOLDER_MAP[parent];
            }
        }

        return sectionName;
    }
}

/**
 * Register section link providers for DX files
 */
function registerSectionLinkProviders(context: vscode.ExtensionContext): void {
    // Register command to open section folders
    context.subscriptions.push(
        vscode.commands.registerCommand('dx.openSectionFolder', async (folderPath: string) => {
            try {
                const uri = vscode.Uri.file(folderPath);
                await vscode.commands.executeCommand('revealInExplorer', uri);
            } catch {
                vscode.window.showInformationMessage(`Folder not found: ${folderPath}`);
            }
        })
    );

    // Document selector for DX files
    const dxSelector: vscode.DocumentSelector = [
        { pattern: '**/dx' },
        { pattern: '**/*.sr' },
        { scheme: 'dxlens' },
        { scheme: 'dxslens' },
    ];

    // Register DocumentLinkProvider
    context.subscriptions.push(
        vscode.languages.registerDocumentLinkProvider(dxSelector, new DxSectionLinkProvider())
    );

    // Register CodeLensProvider
    context.subscriptions.push(
        vscode.languages.registerCodeLensProvider(dxSelector, new DxSectionCodeLensProvider())
    );
}
