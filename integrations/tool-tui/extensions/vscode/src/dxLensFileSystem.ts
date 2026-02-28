/**
 * DxLensFileSystem - Virtual file system provider for .dx files
 * 
 * ⚠️ DEPRECATED: Virtual file system is now commented out.
 * New architecture (2026):
 * - Front-facing .dx/.sr files: Human format (on disk)
 * - .dx/serializer/*.llm: LLM format (token-optimized)
 * - .dx/serializer/*.machine: Machine format (binary)
 * 
 * This code is preserved for reference but not currently active.
 * 
 * OLD BEHAVIOR:
 * Intercepts .dx file operations through VS Code's FileSystemProvider API,
 * presenting human-readable content to users while maintaining token-efficient
 * storage on disk.
 * 
 * Requirements: 1.1, 1.2, 1.3, 1.4, 1.5
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import { DxCore } from './dxCore';
import { DxDocumentManager } from './dxDocumentManager';
import { getDiskUri } from './utils';

/**
 * File type enumeration for VS Code
 */
const FileType = vscode.FileType;

/**
 * DxLensFileSystem - Virtual file system provider
 */
export class DxLensFileSystem implements vscode.FileSystemProvider {
    private dxCore: DxCore;
    private documentManager: DxDocumentManager;

    // Event emitters for file system changes
    private _onDidChangeFile = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
    readonly onDidChangeFile = this._onDidChangeFile.event;

    // Track watched files
    private watchedFiles: Map<string, vscode.Disposable> = new Map();

    constructor(dxCore: DxCore, documentManager: DxDocumentManager) {
        this.dxCore = dxCore;
        this.documentManager = documentManager;
    }

    /**
     * Watch for file changes
     */
    watch(
        uri: vscode.Uri,
        options: { recursive: boolean; excludes: string[] }
    ): vscode.Disposable {
        const diskUri = getDiskUri(uri);
        const key = diskUri.fsPath.toLowerCase();

        // Don't create duplicate watchers
        if (this.watchedFiles.has(key)) {
            return this.watchedFiles.get(key)!;
        }

        // Create file system watcher for the disk file
        const watcher = vscode.workspace.createFileSystemWatcher(
            new vscode.RelativePattern(
                vscode.Uri.file(diskUri.fsPath.substring(0, diskUri.fsPath.lastIndexOf('/'))),
                diskUri.fsPath.substring(diskUri.fsPath.lastIndexOf('/') + 1)
            )
        );

        const disposables: vscode.Disposable[] = [];

        // Handle file changes
        disposables.push(watcher.onDidChange(async (changedUri) => {
            if (!this.documentManager.isWriting(changedUri)) {
                await this.documentManager.handleExternalChange(changedUri);
                this._onDidChangeFile.fire([{
                    type: vscode.FileChangeType.Changed,
                    uri: uri
                }]);
            }
        }));

        // Handle file deletion
        disposables.push(watcher.onDidDelete((deletedUri) => {
            this.documentManager.handleFileDeleted(deletedUri);
            this._onDidChangeFile.fire([{
                type: vscode.FileChangeType.Deleted,
                uri: uri
            }]);
        }));

        disposables.push(watcher);

        const disposable = vscode.Disposable.from(...disposables);
        this.watchedFiles.set(key, disposable);

        return disposable;
    }


    /**
     * Get file stats
     * 
     * Returns stats with the human content size for proper display
     */
    async stat(uri: vscode.Uri): Promise<vscode.FileStat> {
        const diskUri = getDiskUri(uri);

        try {
            const stats = await fs.promises.stat(diskUri.fsPath);

            // Get human content size if available
            const state = this.documentManager.getState(uri);
            const size = state ?
                Buffer.byteLength(state.currentHuman, 'utf-8') :
                stats.size;

            return {
                type: stats.isDirectory() ? FileType.Directory : FileType.File,
                ctime: stats.ctimeMs,
                mtime: stats.mtimeMs,
                size: size,
            };
        } catch (error) {
            throw vscode.FileSystemError.FileNotFound(uri);
        }
    }

    /**
     * Read directory contents
     */
    async readDirectory(uri: vscode.Uri): Promise<[string, vscode.FileType][]> {
        const diskUri = getDiskUri(uri);

        try {
            const entries = await fs.promises.readdir(diskUri.fsPath, { withFileTypes: true });
            return entries.map(entry => [
                entry.name,
                entry.isDirectory() ? FileType.Directory : FileType.File
            ]);
        } catch (error) {
            throw vscode.FileSystemError.FileNotFound(uri);
        }
    }

    /**
     * Create directory
     */
    async createDirectory(uri: vscode.Uri): Promise<void> {
        const diskUri = getDiskUri(uri);

        try {
            await fs.promises.mkdir(diskUri.fsPath, { recursive: true });
        } catch (error) {
            throw vscode.FileSystemError.NoPermissions(uri);
        }
    }

    /**
     * Read file and transform to human format
     * 
     * This is called when opening a .dx file in the editor.
     */
    async readFile(uri: vscode.Uri): Promise<Uint8Array> {
        // Initialize document and get human content
        const humanContent = await this.documentManager.initializeDocument(uri);
        return new TextEncoder().encode(humanContent);
    }

    /**
     * Transform to dense and write to disk
     * 
     * This is called when saving a .dx file in the editor.
     * Format-on-save is handled by the document manager.
     */
    async writeFile(
        uri: vscode.Uri,
        content: Uint8Array,
        options: { create: boolean; overwrite: boolean }
    ): Promise<void> {
        const diskUri = getDiskUri(uri);

        // Check if file exists when create is false
        if (!options.create) {
            try {
                await fs.promises.access(diskUri.fsPath);
            } catch {
                throw vscode.FileSystemError.FileNotFound(uri);
            }
        }

        // Check if file exists when overwrite is false
        if (!options.overwrite) {
            try {
                await fs.promises.access(diskUri.fsPath);
                throw vscode.FileSystemError.FileExists(uri);
            } catch (error) {
                if (error instanceof vscode.FileSystemError) {
                    throw error;
                }
                // File doesn't exist, which is fine
            }
        }

        // Save through document manager (handles validation, formatting, and transformation)
        const saved = await this.documentManager.saveDocument(uri, content);

        if (!saved) {
            // Save was skipped (validation failed or grace period)
            // Don't throw - VS Code will handle this gracefully
        }
    }


    /**
     * Delete file
     */
    async delete(uri: vscode.Uri, options: { recursive: boolean }): Promise<void> {
        const diskUri = getDiskUri(uri);

        try {
            const stats = await fs.promises.stat(diskUri.fsPath);

            if (stats.isDirectory()) {
                if (options.recursive) {
                    await fs.promises.rm(diskUri.fsPath, { recursive: true });
                } else {
                    await fs.promises.rmdir(diskUri.fsPath);
                }
            } else {
                await fs.promises.unlink(diskUri.fsPath);
            }

            // Clean up document state
            this.documentManager.handleFileDeleted(uri);

        } catch (error) {
            throw vscode.FileSystemError.FileNotFound(uri);
        }
    }

    /**
     * Rename file
     */
    async rename(
        oldUri: vscode.Uri,
        newUri: vscode.Uri,
        options: { overwrite: boolean }
    ): Promise<void> {
        const oldDiskUri = getDiskUri(oldUri);
        const newDiskUri = getDiskUri(newUri);

        // Check if target exists when overwrite is false
        if (!options.overwrite) {
            try {
                await fs.promises.access(newDiskUri.fsPath);
                throw vscode.FileSystemError.FileExists(newUri);
            } catch (error) {
                if (error instanceof vscode.FileSystemError) {
                    throw error;
                }
                // File doesn't exist, which is fine
            }
        }

        try {
            await fs.promises.rename(oldDiskUri.fsPath, newDiskUri.fsPath);

            // Clean up old document state
            this.documentManager.handleFileDeleted(oldUri);

        } catch (error) {
            throw vscode.FileSystemError.NoPermissions(oldUri);
        }
    }

    /**
     * Copy file
     */
    async copy(
        source: vscode.Uri,
        destination: vscode.Uri,
        options: { overwrite: boolean }
    ): Promise<void> {
        const sourceDiskUri = getDiskUri(source);
        const destDiskUri = getDiskUri(destination);

        // Check if target exists when overwrite is false
        if (!options.overwrite) {
            try {
                await fs.promises.access(destDiskUri.fsPath);
                throw vscode.FileSystemError.FileExists(destination);
            } catch (error) {
                if (error instanceof vscode.FileSystemError) {
                    throw error;
                }
                // File doesn't exist, which is fine
            }
        }

        try {
            await fs.promises.copyFile(sourceDiskUri.fsPath, destDiskUri.fsPath);
        } catch (error) {
            throw vscode.FileSystemError.NoPermissions(source);
        }
    }

    /**
     * Dispose of resources
     */
    dispose(): void {
        // Dispose all watchers
        for (const disposable of this.watchedFiles.values()) {
            disposable.dispose();
        }
        this.watchedFiles.clear();

        // Dispose event emitter
        this._onDidChangeFile.dispose();
    }
}
