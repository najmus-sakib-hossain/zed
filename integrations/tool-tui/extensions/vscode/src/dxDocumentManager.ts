/**
 * DxDocumentManager - Document state management for DX files
 * 
 * Manages document state, validation, and save coordination for .dx files.
 * Handles:
 * - Document state tracking (disk content, editor content, validation status)
 * - Save coordination with validation gating and grace period
 * - External file change detection
 * - Diagnostic updates for validation errors
 * - Cache file generation (.dx/cache/{filename}.human and .machine)
 * 
 * Requirements: 3.1-3.5, 4.1-4.5, 6.1-6.5
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import { DxCore, ValidationResult } from './dxCore';
import { getDiskUri, getDocumentKey, DX_LENS_SCHEME } from './utils';
import { writeCache, deleteCache } from './cacheManager';
import { parseLlm } from './llmParser';
import { parseHuman } from './humanParser';
import { formatDocument } from './humanFormatter';

/**
 * State for a single document
 */
export interface DocumentState {
    /** Dense content currently on disk */
    diskDense: string;

    /** Last successfully saved dense content */
    lastValidDense: string;

    /** Current human content in editor */
    currentHuman: string;

    /** Whether content is syntactically valid */
    isValid: boolean;

    /** Last validation error */
    lastError: string | null;

    /** Timestamp of last keystroke */
    lastKeystroke: number;

    /** Pending save timeout */
    saveTimeout: NodeJS.Timeout | null;

    /** Whether a save is in progress */
    isSaving: boolean;
}

/**
 * Configuration for the document manager
 */
export interface DocumentManagerConfig {
    /** Validate syntax before saving (default: true) */
    validateBeforeSave: boolean;

    /** Grace period in ms after last keystroke (default: 2000) */
    autoSaveGracePeriod: number;

    /** Indent size: 2 or 4 spaces (default: 2) */
    indentSize: number;

    /** Minimum key padding width (default: 20) */
    keyPadding: number;

    /** Format on save (default: true) */
    formatOnSave: boolean;

    /** Additional delay for format-on-save when auto-save is enabled (default: 1500ms) */
    formatDelayAfterAutoSave: number;
}


/**
 * DxDocumentManager - Manages document state and save coordination
 */
export class DxDocumentManager implements vscode.Disposable {
    private documents: Map<string, DocumentState> = new Map();
    private writingFiles: Set<string> = new Set();
    private diagnosticCollection: vscode.DiagnosticCollection;
    private config: DocumentManagerConfig;
    private dxCore: DxCore;
    private disposables: vscode.Disposable[] = [];
    
    /** Pending format timeouts for debounced format-on-save */
    private pendingFormatTimeouts: Map<string, NodeJS.Timeout> = new Map();
    
    /** Track last auto-save time per document */
    private lastAutoSaveTime: Map<string, number> = new Map();

    constructor(dxCore: DxCore, config?: Partial<DocumentManagerConfig>) {
        this.dxCore = dxCore;
        this.config = {
            validateBeforeSave: config?.validateBeforeSave ?? true,
            autoSaveGracePeriod: config?.autoSaveGracePeriod ?? 2000,
            indentSize: config?.indentSize ?? 2,
            keyPadding: config?.keyPadding ?? 20,
            formatOnSave: config?.formatOnSave ?? true,
            formatDelayAfterAutoSave: config?.formatDelayAfterAutoSave ?? 1500,
        };

        this.diagnosticCollection = vscode.languages.createDiagnosticCollection('dx');
        this.disposables.push(this.diagnosticCollection);
    }

    /**
     * Update configuration
     */
    updateConfig(config: Partial<DocumentManagerConfig>): void {
        if (config.validateBeforeSave !== undefined) {
            this.config.validateBeforeSave = config.validateBeforeSave;
        }
        if (config.autoSaveGracePeriod !== undefined) {
            this.config.autoSaveGracePeriod = config.autoSaveGracePeriod;
        }
        if (config.indentSize !== undefined) {
            this.config.indentSize = config.indentSize;
        }
        if (config.keyPadding !== undefined) {
            this.config.keyPadding = config.keyPadding;
        }
        if (config.formatOnSave !== undefined) {
            this.config.formatOnSave = config.formatOnSave;
        }
        if (config.formatDelayAfterAutoSave !== undefined) {
            this.config.formatDelayAfterAutoSave = config.formatDelayAfterAutoSave;
        }
    }

    /**
     * Check if VS Code auto-save is enabled with delay
     * @returns Object with autoSave mode and delay in ms
     */
    private getAutoSaveSettings(): { isAutoSaveAfterDelay: boolean; delay: number } {
        const autoSave = vscode.workspace.getConfiguration('files').get<string>('autoSave');
        const autoSaveDelay = vscode.workspace.getConfiguration('files').get<number>('autoSaveDelay') ?? 1000;
        
        return {
            isAutoSaveAfterDelay: autoSave === 'afterDelay',
            delay: autoSaveDelay
        };
    }

    /**
     * Check if this save appears to be triggered by auto-save
     * @param key - Document key
     * @returns Whether this is likely an auto-save trigger
     */
    private isAutoSaveTrigger(key: string): boolean {
        const state = this.documents.get(key);
        if (!state) {
            return false;
        }

        const { isAutoSaveAfterDelay, delay } = this.getAutoSaveSettings();
        if (!isAutoSaveAfterDelay) {
            return false;
        }

        // If the last keystroke was within the auto-save delay window, this is likely auto-save
        const timeSinceLastKeystroke = Date.now() - state.lastKeystroke;
        // Add a small buffer (200ms) to account for timing variations
        return timeSinceLastKeystroke <= delay + 200;
    }

    /**
     * Schedule a debounced format for a document
     * @param uri - Document URI
     * @param content - Content to format
     */
    private scheduleDebouncedFormat(uri: vscode.Uri, content: string): void {
        const key = getDocumentKey(uri);
        
        // Clear any existing pending format
        const existingTimeout = this.pendingFormatTimeouts.get(key);
        if (existingTimeout) {
            clearTimeout(existingTimeout);
        }

        const { delay: autoSaveDelay } = this.getAutoSaveSettings();
        // Wait for auto-save delay + additional buffer to ensure user has stopped typing
        const formatDelay = autoSaveDelay + this.config.formatDelayAfterAutoSave;

        const timeout = setTimeout(async () => {
            this.pendingFormatTimeouts.delete(key);
            
            const state = this.documents.get(key);
            if (!state) {
                return;
            }

            // Check if user is still typing (keystroke within last autoSaveDelay)
            const timeSinceLastKeystroke = Date.now() - state.lastKeystroke;
            if (timeSinceLastKeystroke < autoSaveDelay) {
                // User is still typing, reschedule
                this.scheduleDebouncedFormat(uri, state.currentHuman);
                return;
            }

            // Perform the format
            try {
                const parseResult = parseHuman(state.currentHuman);
                if (parseResult.success && parseResult.document) {
                    const doc = parseResult.document;
                    if (doc.context.size > 0 || doc.refs.size > 0 || doc.sections.size > 0) {
                        const formattedContent = formatDocument(parseResult.document);
                        if (formattedContent !== state.currentHuman) {
                            state.currentHuman = formattedContent;
                            await this.updateEditorContent(uri, formattedContent);
                            
                            // Also save the formatted content to disk
                            const denseResult = this.dxCore.toDense(formattedContent);
                            if (denseResult.success) {
                                const diskUri = getDiskUri(uri);
                                this.writingFiles.add(key);
                                await fs.promises.writeFile(diskUri.fsPath, denseResult.content, 'utf-8');
                                state.diskDense = denseResult.content;
                                state.lastValidDense = denseResult.content;
                                setTimeout(() => this.writingFiles.delete(key), 100);
                            }
                        }
                    }
                }
            } catch (error) {
                console.warn(`DX: Debounced format failed: ${error}`);
            }
        }, formatDelay);

        this.pendingFormatTimeouts.set(key, timeout);
    }

    /**
     * Initialize document when first opened
     * 
     * @param uri - The URI of the document
     * @returns The human-readable content for display
     */
    async initializeDocument(uri: vscode.Uri): Promise<string> {
        const diskUri = getDiskUri(uri);
        const key = getDocumentKey(uri);

        // Read dense content from disk
        let diskDense = '';
        try {
            const content = await fs.promises.readFile(diskUri.fsPath, 'utf-8');
            diskDense = content;
        } catch (error) {
            // File might not exist yet
            diskDense = '';
        }

        // Transform to human format
        const humanResult = this.dxCore.toHuman(diskDense);
        const currentHuman = humanResult.success ? humanResult.content : diskDense;

        // Validate content
        const validation = this.dxCore.validate(currentHuman);

        // Create document state
        const state: DocumentState = {
            diskDense,
            lastValidDense: diskDense,
            currentHuman,
            isValid: validation.success,
            lastError: validation.error ?? null,
            lastKeystroke: Date.now(),
            saveTimeout: null,
            isSaving: false,
        };

        this.documents.set(key, state);

        // Update diagnostics
        this.updateDiagnostics(uri, validation);

        return currentHuman;
    }


    /**
     * Handle content change in editor
     * 
     * @param uri - The URI of the document
     * @param content - The new human-readable content
     */
    handleContentChange(uri: vscode.Uri, content: string): void {
        const key = getDocumentKey(uri);
        const state = this.documents.get(key);

        if (!state) {
            return;
        }

        // Update state
        state.currentHuman = content;
        state.lastKeystroke = Date.now();

        // Validate content
        const validation = this.dxCore.validate(content);
        state.isValid = validation.success;
        state.lastError = validation.error ?? null;

        // Update diagnostics
        this.updateDiagnostics(uri, validation);
    }

    /**
     * Save document with validation gating and format-on-save
     * 
     * @param uri - The URI of the document
     * @param content - The human-readable content to save
     * @returns Whether the save was successful
     */
    async saveDocument(uri: vscode.Uri, content: Uint8Array): Promise<boolean> {
        const key = getDocumentKey(uri);
        const state = this.documents.get(key);

        if (!state) {
            // Initialize if not tracked
            await this.initializeDocument(uri);
            return this.saveDocument(uri, content);
        }

        let humanContent = new TextDecoder().decode(content);
        state.currentHuman = humanContent;

        // Note: Grace period check removed to ensure saves happen immediately
        // The grace period was causing saves to be skipped

        // Validate if configured
        if (this.config.validateBeforeSave) {
            const validation = this.dxCore.validate(humanContent);
            state.isValid = validation.success;
            state.lastError = validation.error ?? null;

            // Update diagnostics
            this.updateDiagnostics(uri, validation);

            if (!validation.success) {
                // Skip save for invalid content
                this.showValidationWarning(validation);
                return false;
            }
        }

        // Smart format-on-save: handle auto-save delay scenario
        // Only apply to human format content (not LLM format)
        const isAutoSave = this.isAutoSaveTrigger(key);
        const shouldFormatNow = this.config.formatOnSave && !humanContent.trim().startsWith('#') && !isAutoSave;
        const shouldScheduleFormat = this.config.formatOnSave && !humanContent.trim().startsWith('#') && isAutoSave;

        if (shouldScheduleFormat) {
            // Auto-save is active - schedule debounced format for later
            this.scheduleDebouncedFormat(uri, humanContent);
        } else if (shouldFormatNow) {
            // Manual save or auto-save not active - format immediately
            try {
                const parseResult = parseHuman(humanContent);
                if (parseResult.success && parseResult.document) {
                    // Check if the parsed document has any content
                    const doc = parseResult.document;
                    if (doc.context.size > 0 || doc.refs.size > 0 || doc.sections.size > 0) {
                        const formattedContent = formatDocument(parseResult.document);
                        humanContent = formattedContent;
                        state.currentHuman = humanContent;
                    }
                }
                // If parsing fails, continue with original content (graceful degradation)
            } catch (formatError) {
                // Format-on-save failed - log warning but continue with original content
                console.warn(`DX: Format-on-save failed, using original content: ${formatError}`);
            }
        }

        // Transform to dense format
        const denseResult = this.dxCore.toDense(humanContent);
        if (!denseResult.success) {
            vscode.window.showErrorMessage(`DX: Transform failed: ${denseResult.error}`);
            return false;
        }

        // Write to disk
        const diskUri = getDiskUri(uri);
        try {
            state.isSaving = true;
            this.writingFiles.add(key);

            await fs.promises.writeFile(diskUri.fsPath, denseResult.content, 'utf-8');

            // Update state
            state.diskDense = denseResult.content;
            state.lastValidDense = denseResult.content;

            // Generate cache files (Requirements: 4.1-4.3)
            await this.generateCacheFiles(diskUri.fsPath, denseResult.content);

            // Update editor with formatted content if format-on-save is enabled
            if (this.config.formatOnSave) {
                await this.updateEditorContent(uri, humanContent);
            }

            return true;
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            vscode.window.showErrorMessage(`DX: Failed to save file: ${message}`);
            return false;
        } finally {
            state.isSaving = false;
            // Remove from writing set after a short delay to handle file watcher events
            setTimeout(() => this.writingFiles.delete(key), 100);
        }
    }

    /**
     * Update editor content with reformatted text
     * 
     * @param uri - The URI of the document
     * @param newContent - The new content to display
     */
    private async updateEditorContent(uri: vscode.Uri, newContent: string): Promise<void> {
        // Find the editor showing this document
        for (const editor of vscode.window.visibleTextEditors) {
            // Check both the direct URI and the lens URI
            const editorUriStr = editor.document.uri.toString();
            const targetUriStr = uri.toString();
            const lensUriStr = uri.with({ scheme: DX_LENS_SCHEME }).toString();

            if (editorUriStr === targetUriStr || editorUriStr === lensUriStr) {
                const fullRange = new vscode.Range(
                    editor.document.positionAt(0),
                    editor.document.positionAt(editor.document.getText().length)
                );

                const edit = new vscode.WorkspaceEdit();
                edit.replace(editor.document.uri, fullRange, newContent);
                await vscode.workspace.applyEdit(edit);

                // Save the document again to persist the reformatted content
                // (This won't cause infinite loop because content will match)
                break;
            }
        }
    }

    /**
     * Generate cache files for a saved document
     * Requirements: 4.1, 4.2, 4.3
     */
    private async generateCacheFiles(filePath: string, llmContent: string): Promise<void> {
        try {
            const parseResult = parseLlm(llmContent);
            if (parseResult.success && parseResult.document) {
                const cacheResult = await writeCache(filePath, parseResult.document);
                if (!cacheResult.success) {
                    console.warn(`DX: Cache generation failed: ${cacheResult.error}`);
                }
            }
        } catch (error) {
            console.warn(`DX: Cache generation error: ${error}`);
        }
    }


    /**
     * Force save without validation
     * 
     * @param uri - The URI of the document
     */
    async forceSave(uri: vscode.Uri): Promise<boolean> {
        const key = getDocumentKey(uri);
        const state = this.documents.get(key);

        if (!state) {
            return false;
        }

        // Transform to dense format
        const denseResult = this.dxCore.toDense(state.currentHuman);
        if (!denseResult.success) {
            vscode.window.showErrorMessage(`DX: Transform failed: ${denseResult.error}`);
            return false;
        }

        // Write to disk
        const diskUri = getDiskUri(uri);
        try {
            state.isSaving = true;
            this.writingFiles.add(key);

            await fs.promises.writeFile(diskUri.fsPath, denseResult.content, 'utf-8');

            // Update state
            state.diskDense = denseResult.content;
            state.lastValidDense = denseResult.content;

            vscode.window.showInformationMessage('DX: File saved (validation bypassed)');
            return true;
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            vscode.window.showErrorMessage(`DX: Failed to save file: ${message}`);
            return false;
        } finally {
            state.isSaving = false;
            setTimeout(() => this.writingFiles.delete(key), 100);
        }
    }

    /**
     * Handle external file changes
     * 
     * @param uri - The URI of the changed file
     */
    async handleExternalChange(uri: vscode.Uri): Promise<void> {
        const key = getDocumentKey(uri);

        // Ignore if we're currently writing this file
        if (this.writingFiles.has(key)) {
            return;
        }

        const state = this.documents.get(key);
        if (!state) {
            return;
        }

        // Read new content from disk
        const diskUri = getDiskUri(uri);
        try {
            const newDense = await fs.promises.readFile(diskUri.fsPath, 'utf-8');

            // Skip if content hasn't changed
            if (newDense === state.diskDense) {
                return;
            }

            // Transform to human format
            const humanResult = this.dxCore.toHuman(newDense);
            const newHuman = humanResult.success ? humanResult.content : newDense;

            // Update state
            state.diskDense = newDense;
            state.lastValidDense = newDense;
            state.currentHuman = newHuman;

            // Validate
            const validation = this.dxCore.validate(newHuman);
            state.isValid = validation.success;
            state.lastError = validation.error ?? null;

            // Update diagnostics
            this.updateDiagnostics(uri, validation);

        } catch (error) {
            // File might have been deleted
            console.warn(`DX: Failed to read external change: ${error}`);
        }
    }

    /**
     * Force refresh from disk
     * 
     * @param uri - The URI of the document
     * @returns The refreshed human-readable content
     */
    async forceRefresh(uri: vscode.Uri): Promise<string> {
        const key = getDocumentKey(uri);

        // Remove existing state
        this.documents.delete(key);

        // Re-initialize
        return this.initializeDocument(uri);
    }


    /**
     * Handle file deletion
     * 
     * @param uri - The URI of the deleted file
     */
    handleFileDeleted(uri: vscode.Uri): void {
        const key = getDocumentKey(uri);

        // Clear state
        const state = this.documents.get(key);
        if (state?.saveTimeout) {
            clearTimeout(state.saveTimeout);
        }
        this.documents.delete(key);

        // Clear diagnostics
        this.diagnosticCollection.delete(uri);

        // Delete cache files (Requirement: 4.5)
        const diskUri = getDiskUri(uri);
        deleteCache(diskUri.fsPath).catch(error => {
            console.warn(`DX: Failed to delete cache: ${error}`);
        });
    }

    /**
     * Get current document state
     * 
     * @param uri - The URI of the document
     * @returns The document state or undefined
     */
    getState(uri: vscode.Uri): DocumentState | undefined {
        const key = getDocumentKey(uri);
        return this.documents.get(key);
    }

    /**
     * Check if extension is currently writing to a file
     * 
     * @param uri - The URI to check
     * @returns Whether the extension is writing to this file
     */
    isWriting(uri: vscode.Uri): boolean {
        const key = getDocumentKey(uri);
        return this.writingFiles.has(key);
    }

    /**
     * Get the dense content for a document
     * 
     * @param uri - The URI of the document
     * @returns The dense content or undefined
     */
    getDenseContent(uri: vscode.Uri): string | undefined {
        const state = this.getState(uri);
        if (!state) {
            return undefined;
        }

        // Transform current human content to dense
        const result = this.dxCore.toDense(state.currentHuman);
        return result.success ? result.content : state.diskDense;
    }

    /**
     * Update diagnostics for a document
     */
    private updateDiagnostics(uri: vscode.Uri, validation: ValidationResult): void {
        if (validation.success) {
            this.diagnosticCollection.delete(uri);
            return;
        }

        const line = (validation.line ?? 1) - 1; // Convert to 0-indexed
        const column = (validation.column ?? 1) - 1;

        const range = new vscode.Range(
            new vscode.Position(line, column),
            new vscode.Position(line, column + 1)
        );

        const diagnostic = new vscode.Diagnostic(
            range,
            validation.error ?? 'Syntax error',
            vscode.DiagnosticSeverity.Error
        );

        diagnostic.source = 'DX Serializer';

        if (validation.hint) {
            diagnostic.message += `\n\nHint: ${validation.hint}`;
        }

        this.diagnosticCollection.set(uri, [diagnostic]);
    }

    /**
     * Show validation warning in status bar
     */
    private showValidationWarning(validation: ValidationResult): void {
        const message = validation.error ?? 'Syntax error';
        vscode.window.setStatusBarMessage(`$(warning) DX: ${message}`, 5000);
    }

    // ========================================================================
    // Format Conversion Methods (Requirements: 5.4, 6.4, 6.5)
    // ========================================================================

    /**
     * Convert document content to Human format
     * Requirements: 6.4
     * 
     * @param content - The content to convert (LLM or Human format)
     * @returns Converted Human format content or error
     */
    convertToHuman(content: string): { success: boolean; content?: string; error?: string } {
        try {
            // Try parsing as LLM format first
            const llmResult = parseLlm(content);
            if (llmResult.success && llmResult.document) {
                const humanContent = formatDocument(llmResult.document);
                return { success: true, content: humanContent };
            }

            // Try parsing as Human format (already human, just reformat)
            const humanResult = parseHuman(content);
            if (humanResult.success && humanResult.document) {
                const humanContent = formatDocument(humanResult.document);
                return { success: true, content: humanContent };
            }

            return { success: false, error: 'Failed to parse content as LLM or Human format' };
        } catch (error) {
            return { success: false, error: `Conversion error: ${error}` };
        }
    }

    /**
     * Convert document content to LLM format
     * Requirements: 6.5
     * 
     * @param content - The content to convert (LLM or Human format)
     * @returns Converted LLM format content or error
     */
    convertToLlm(content: string): { success: boolean; content?: string; error?: string } {
        try {
            // Try parsing as Human format first
            const humanResult = parseHuman(content);
            if (humanResult.success && humanResult.document) {
                const { serializeToLlm } = require('./humanParser');
                const llmContent = serializeToLlm(humanResult.document);
                return { success: true, content: llmContent };
            }

            // Try parsing as LLM format (already LLM, just return)
            const llmResult = parseLlm(content);
            if (llmResult.success && llmResult.document) {
                const { serializeToLlm } = require('./humanParser');
                const llmContent = serializeToLlm(llmResult.document);
                return { success: true, content: llmContent };
            }

            return { success: false, error: 'Failed to parse content as LLM or Human format' };
        } catch (error) {
            return { success: false, error: `Conversion error: ${error}` };
        }
    }

    /**
     * Format document content (auto-detect format and reformat)
     * Requirements: 5.3
     * 
     * @param uri - The URI of the document
     * @returns Whether formatting was successful
     */
    async formatDocument(uri: vscode.Uri): Promise<boolean> {
        const state = this.getState(uri);
        if (!state) {
            return false;
        }

        const result = this.convertToHuman(state.currentHuman);
        if (!result.success || !result.content) {
            return false;
        }

        // Update editor content
        await this.updateEditorContent(uri, result.content);
        state.currentHuman = result.content;

        return true;
    }

    /**
     * Convert document to specified format
     * Requirements: 5.4, 6.4, 6.5
     * 
     * @param uri - The URI of the document
     * @param targetFormat - 'human' or 'llm'
     * @returns Whether conversion was successful
     */
    async convertFormat(uri: vscode.Uri, targetFormat: 'human' | 'llm'): Promise<boolean> {
        const state = this.getState(uri);
        if (!state) {
            return false;
        }

        const result = targetFormat === 'human'
            ? this.convertToHuman(state.currentHuman)
            : this.convertToLlm(state.currentHuman);

        if (!result.success || !result.content) {
            vscode.window.showErrorMessage(`DX: Conversion failed: ${result.error}`);
            return false;
        }

        // Update editor content
        await this.updateEditorContent(uri, result.content);
        state.currentHuman = result.content;

        vscode.window.showInformationMessage(`DX: Converted to ${targetFormat.toUpperCase()} format`);
        return true;
    }

    /**
     * Dispose of resources
     */
    dispose(): void {
        // Clear all timeouts
        for (const state of this.documents.values()) {
            if (state.saveTimeout) {
                clearTimeout(state.saveTimeout);
            }
        }
        
        // Clear pending format timeouts
        for (const timeout of this.pendingFormatTimeouts.values()) {
            clearTimeout(timeout);
        }
        this.pendingFormatTimeouts.clear();

        // Clear collections
        this.documents.clear();
        this.writingFiles.clear();
        this.lastAutoSaveTime.clear();

        // Dispose of VS Code resources
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }
}
