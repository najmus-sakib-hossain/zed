/**
 * Cache Manager for DX Serializer VS Code Extension
 * 
 * Manages cache files in .dx/cache directory:
 * - Human format cache: {filename}.human (text, Human V3 format)
 * - Machine format cache: {filename}.machine (binary, optimized for compilers)
 * 
 * Requirements: 4.1, 4.2, 4.3, 4.4, 4.5
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { DxDocument } from './llmParser';
import { formatDocument } from './humanFormatter';
import { serializeToBinary } from './machineFormat';

// ============================================================================
// Types
// ============================================================================

export interface CacheResult {
    success: boolean;
    humanPath?: string;
    machinePath?: string;
    error?: string;
}

export interface CacheConfig {
    cacheDir: string;
    humanExtension: string;
    machineExtension: string;
}

// ============================================================================
// Default Configuration
// ============================================================================

export const DEFAULT_CACHE_CONFIG: CacheConfig = {
    cacheDir: '.dx/cache',
    humanExtension: '.human',
    machineExtension: '.machine',
};

// ============================================================================
// Path Utilities
// ============================================================================

/**
 * Get the workspace root for a file
 */
export function getWorkspaceRoot(filePath: string): string | undefined {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(filePath));
    return workspaceFolder?.uri.fsPath;
}

/**
 * Get the cache directory path for a workspace
 */
export function getCacheDir(workspaceRoot: string, config: CacheConfig = DEFAULT_CACHE_CONFIG): string {
    return path.join(workspaceRoot, config.cacheDir);
}

/**
 * Get the relative path of a file from workspace root
 */
export function getRelativePath(filePath: string, workspaceRoot: string): string {
    return path.relative(workspaceRoot, filePath);
}

/**
 * Get the cache file paths for a source file
 * Preserves subdirectory structure in cache
 */
export function getCachePaths(
    filePath: string,
    workspaceRoot: string,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): { humanPath: string; machinePath: string } {
    const relativePath = getRelativePath(filePath, workspaceRoot);
    const baseName = path.basename(relativePath, path.extname(relativePath));
    const dirName = path.dirname(relativePath);

    const cacheDir = getCacheDir(workspaceRoot, config);
    const cacheSubDir = path.join(cacheDir, dirName);

    return {
        humanPath: path.join(cacheSubDir, baseName + config.humanExtension),
        machinePath: path.join(cacheSubDir, baseName + config.machineExtension),
    };
}

// ============================================================================
// Cache Operations
// ============================================================================

/**
 * Ensure the cache directory exists
 */
export async function ensureCacheDir(
    workspaceRoot: string,
    subDir: string = '',
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): Promise<void> {
    const cacheDir = getCacheDir(workspaceRoot, config);
    const targetDir = subDir ? path.join(cacheDir, subDir) : cacheDir;

    try {
        await vscode.workspace.fs.createDirectory(vscode.Uri.file(targetDir));
    } catch {
        // Directory may already exist
    }
}

/**
 * Write human format cache file
 * Requirement: 4.2
 */
export async function writeHumanCache(
    filePath: string,
    document: DxDocument,
    workspaceRoot: string,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): Promise<string> {
    const { humanPath } = getCachePaths(filePath, workspaceRoot, config);
    const humanContent = formatDocument(document);

    // Ensure directory exists
    const dirName = path.dirname(humanPath);
    await ensureCacheDir(workspaceRoot, path.relative(getCacheDir(workspaceRoot, config), dirName), config);

    // Write file
    const uri = vscode.Uri.file(humanPath);
    await vscode.workspace.fs.writeFile(uri, Buffer.from(humanContent, 'utf-8'));

    return humanPath;
}

/**
 * Write machine format cache file (binary)
 * Requirement: 4.3
 */
export async function writeMachineCache(
    filePath: string,
    document: DxDocument,
    workspaceRoot: string,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): Promise<string> {
    const { machinePath } = getCachePaths(filePath, workspaceRoot, config);

    // Serialize to binary format (optimized for compilers)
    const binaryContent = serializeToBinary(document);

    // Ensure directory exists
    const dirName = path.dirname(machinePath);
    await ensureCacheDir(workspaceRoot, path.relative(getCacheDir(workspaceRoot, config), dirName), config);

    // Write binary file
    const uri = vscode.Uri.file(machinePath);
    await vscode.workspace.fs.writeFile(uri, binaryContent);

    return machinePath;
}

/**
 * Write both human and machine cache files
 * Requirements: 4.1, 4.2, 4.3
 */
export async function writeCache(
    filePath: string,
    document: DxDocument,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): Promise<CacheResult> {
    const workspaceRoot = getWorkspaceRoot(filePath);
    if (!workspaceRoot) {
        return {
            success: false,
            error: 'No workspace folder found for file',
        };
    }

    try {
        const humanPath = await writeHumanCache(filePath, document, workspaceRoot, config);
        const machinePath = await writeMachineCache(filePath, document, workspaceRoot, config);

        return {
            success: true,
            humanPath,
            machinePath,
        };
    } catch (error) {
        return {
            success: false,
            error: `Cache write error: ${error instanceof Error ? error.message : String(error)}`,
        };
    }
}

/**
 * Delete cache files for a source file
 * Requirement: 4.5
 */
export async function deleteCache(
    filePath: string,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): Promise<CacheResult> {
    const workspaceRoot = getWorkspaceRoot(filePath);
    if (!workspaceRoot) {
        return {
            success: false,
            error: 'No workspace folder found for file',
        };
    }

    const { humanPath, machinePath } = getCachePaths(filePath, workspaceRoot, config);

    try {
        // Try to delete human cache
        try {
            await vscode.workspace.fs.delete(vscode.Uri.file(humanPath));
        } catch {
            // File may not exist
        }

        // Try to delete machine cache
        try {
            await vscode.workspace.fs.delete(vscode.Uri.file(machinePath));
        } catch {
            // File may not exist
        }

        return {
            success: true,
            humanPath,
            machinePath,
        };
    } catch (error) {
        return {
            success: false,
            error: `Cache delete error: ${error instanceof Error ? error.message : String(error)}`,
        };
    }
}

/**
 * Check if cache files exist for a source file
 */
export async function cacheExists(
    filePath: string,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): Promise<{ human: boolean; machine: boolean }> {
    const workspaceRoot = getWorkspaceRoot(filePath);
    if (!workspaceRoot) {
        return { human: false, machine: false };
    }

    const { humanPath, machinePath } = getCachePaths(filePath, workspaceRoot, config);

    let human = false;
    let machine = false;

    try {
        await vscode.workspace.fs.stat(vscode.Uri.file(humanPath));
        human = true;
    } catch {
        // File doesn't exist
    }

    try {
        await vscode.workspace.fs.stat(vscode.Uri.file(machinePath));
        machine = true;
    } catch {
        // File doesn't exist
    }

    return { human, machine };
}
