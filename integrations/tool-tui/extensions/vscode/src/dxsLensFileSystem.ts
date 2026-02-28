/**
 * SR Lens File System Provider
 * 
 * Virtual file system that displays .sr files (LLM format) as human-readable format.
 * When a user opens a .sr file, they see the clean human format instead of the
 * token-efficient LLM format stored on disk.
 * 
 * Flow:
 * 1. User opens .sr file
 * 2. Extension redirects to dxslens:// scheme
 * 3. This provider reads the LLM format from disk
 * 4. Converts to Human Format V3 for display
 * 5. On save, converts back to LLM format
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import { getDxsDiskUri } from './utils';
import { parseLlm, DxDocument } from './llmParser';
import { formatDocument } from './humanFormatter';
import { parseHuman, serializeToLlm } from './humanParser';

/**
 * File system provider for SR Lens virtual documents
 */
export class DxsLensFileSystem implements vscode.FileSystemProvider {
    private _emitter = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
    readonly onDidChangeFile: vscode.Event<vscode.FileChangeEvent[]> = this._emitter.event;

    // Cache for converted content
    private _cache = new Map<string, { content: string; mtime: number }>();

    watch(_uri: vscode.Uri): vscode.Disposable {
        return new vscode.Disposable(() => { });
    }

    stat(uri: vscode.Uri): vscode.FileStat {
        const diskUri = getDxsDiskUri(uri);
        const stats = fs.statSync(diskUri.fsPath);

        return {
            type: vscode.FileType.File,
            ctime: stats.ctimeMs,
            mtime: stats.mtimeMs,
            size: stats.size,
        };
    }

    readDirectory(_uri: vscode.Uri): [string, vscode.FileType][] {
        return [];
    }

    createDirectory(_uri: vscode.Uri): void {
        throw vscode.FileSystemError.NoPermissions('Cannot create directories in SR Lens');
    }

    /**
     * Read a .sr file and return human-readable format
     */
    readFile(uri: vscode.Uri): Uint8Array {
        const diskUri = getDxsDiskUri(uri);
        const diskPath = diskUri.fsPath;

        // Read the LLM format from disk
        const llmContent = fs.readFileSync(diskPath, 'utf-8');

        // Parse LLM format
        const parseResult = parseLlm(llmContent);
        if (!parseResult.success || !parseResult.document) {
            // If parsing fails, return the raw content
            console.warn('SR Lens: Failed to parse LLM format, showing raw content');
            return Buffer.from(llmContent, 'utf-8');
        }

        // Convert to Human Format
        const humanContent = formatDocument(parseResult.document);

        // Cache the result
        const stats = fs.statSync(diskPath);
        this._cache.set(diskPath, {
            content: humanContent,
            mtime: stats.mtimeMs,
        });

        return Buffer.from(humanContent, 'utf-8');
    }

    /**
     * Write human format back to disk as LLM format
     */
    writeFile(uri: vscode.Uri, content: Uint8Array, _options: { create: boolean; overwrite: boolean }): void {
        const diskUri = getDxsDiskUri(uri);
        const diskPath = diskUri.fsPath;

        const humanContent = Buffer.from(content).toString('utf-8');

        // Parse the human format
        const parseResult = parseHuman(humanContent);
        if (!parseResult.success || !parseResult.document) {
            // If parsing fails, try to save as-is (might be LLM format already)
            console.warn('SR Lens: Failed to parse human format, saving as-is');
            fs.writeFileSync(diskPath, humanContent, 'utf-8');
            return;
        }

        // Convert to LLM format for storage
        const llmContent = serializeToLlm(parseResult.document);

        // Write to disk
        fs.writeFileSync(diskPath, llmContent, 'utf-8');

        // Update cache
        const stats = fs.statSync(diskPath);
        this._cache.set(diskPath, {
            content: humanContent,
            mtime: stats.mtimeMs,
        });

        // Notify of change
        this._emitter.fire([{ type: vscode.FileChangeType.Changed, uri }]);
    }

    delete(uri: vscode.Uri, _options: { recursive: boolean }): void {
        const diskUri = getDxsDiskUri(uri);
        fs.unlinkSync(diskUri.fsPath);
        this._cache.delete(diskUri.fsPath);
    }

    rename(oldUri: vscode.Uri, newUri: vscode.Uri, _options: { overwrite: boolean }): void {
        const oldDiskUri = getDxsDiskUri(oldUri);
        const newDiskUri = getDxsDiskUri(newUri);
        fs.renameSync(oldDiskUri.fsPath, newDiskUri.fsPath);
        this._cache.delete(oldDiskUri.fsPath);
    }

    /**
     * Invalidate cache for a file
     */
    invalidateCache(uri: vscode.Uri): void {
        const diskUri = getDxsDiskUri(uri);
        this._cache.delete(diskUri.fsPath);
    }

    dispose(): void {
        this._emitter.dispose();
        this._cache.clear();
    }
}
