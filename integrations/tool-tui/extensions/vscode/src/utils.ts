/**
 * Utility functions for DX Serializer VS Code Extension
 * 
 * Provides helper functions for:
 * - File type detection (isExactlyDxFile)
 * - URI conversion (getDiskUri, getLensUri)
 * - Debouncing (debounce)
 */

import * as vscode from 'vscode';

/**
 * The URI scheme for the DX Lens virtual file system
 */
export const DX_LENS_SCHEME = 'dxlens';

/**
 * The URI scheme for the SR Lens virtual file system (shows human format for .sr files)
 */
export const DXS_LENS_SCHEME = 'dxslens';

/**
 * Check if a URI points to a DX file:
 * - Files ending with exactly .sr
 * - Files named exactly "dx" (no extension, no prefix, no suffix)
 * 
 * @param uri - The URI to check
 * @returns true if the URI points to a DX file
 * 
 * Requirements: 4.1-4.7
 */
export function isExactlyDxFile(uri: vscode.Uri): boolean {
    // Must be a file scheme, dxlens scheme, or dxslens scheme
    if (uri.scheme !== 'file' && uri.scheme !== DX_LENS_SCHEME && uri.scheme !== DXS_LENS_SCHEME) {
        return false;
    }

    const path = uri.fsPath || uri.path;

    // Must have a path
    if (!path) {
        return false;
    }

    // Get the filename from the path
    const filename = path.split(/[/\\]/).pop() || '';

    // Check if filename is exactly "dx" (no extension, no prefix, no suffix)
    if (filename === 'dx') {
        return true;
    }

    // Check if filename ends with .sr
    if (filename.endsWith('.sr')) {
        return true;
    }

    return false;
}

/**
 * Check if a URI points to a SR file (LLM format source file)
 * - Files ending with exactly .sr, dx
 * 
 * @param uri - The URI to check
 * @returns true if the URI points to a SR file
 */
export function isDxsFile(uri: vscode.Uri): boolean {
    // Must be a file scheme (not git, untitled, etc.)
    if (uri.scheme !== 'file' && uri.scheme !== DXS_LENS_SCHEME) {
        return false;
    }

    const path = uri.fsPath || uri.path;
    if (!path) {
        return false;
    }

    const filename = path.split(/[/\\]/).pop() || '';
    return filename.endsWith('.sr');
}

/**
 * Convert a disk URI (file://) to a SR Lens URI (dxslens://)
 * 
 * @param diskUri - The file system URI
 * @returns The corresponding SR Lens virtual file system URI
 */
export function getDxsLensUri(diskUri: vscode.Uri): vscode.Uri {
    if (diskUri.scheme === DXS_LENS_SCHEME) {
        return diskUri;
    }
    return diskUri.with({ scheme: DXS_LENS_SCHEME });
}

/**
 * Convert a SR Lens URI (dxslens://) to a disk URI (file://)
 * 
 * @param lensUri - The SR Lens virtual file system URI
 * @returns The corresponding file system URI
 */
export function getDxsDiskUri(lensUri: vscode.Uri): vscode.Uri {
    if (lensUri.scheme === 'file') {
        return lensUri;
    }
    return vscode.Uri.file(lensUri.path);
}

/**
 * Check if a path string represents exactly a .sr file or a file named exactly "dx"
 * 
 * @param path - The file path to check
 * @returns true if the path is a .sr file or named exactly "dx"
 */
export function isExactlyDxPath(path: string): boolean {
    if (!path) {
        return false;
    }

    // Get the filename from the path
    const filename = path.split(/[/\\]/).pop() || '';

    // Check if filename is exactly "dx" (no extension, no prefix, no suffix)
    if (filename === 'dx') {
        return true;
    }

    // Check if filename ends with .sr
    if (filename.endsWith('.sr')) {
        return true;
    }

    return false;
}

/**
 * Convert a DX Lens URI (dxlens://) to a disk URI (file://)
 * 
 * @param lensUri - The DX Lens virtual file system URI
 * @returns The corresponding file system URI
 */
export function getDiskUri(lensUri: vscode.Uri): vscode.Uri {
    if (lensUri.scheme === 'file') {
        return lensUri;
    }

    // Convert dxlens:// to file://
    return vscode.Uri.file(lensUri.path);
}

/**
 * Convert a disk URI (file://) to a DX Lens URI (dxlens://)
 * 
 * @param diskUri - The file system URI
 * @returns The corresponding DX Lens virtual file system URI
 */
export function getLensUri(diskUri: vscode.Uri): vscode.Uri {
    if (diskUri.scheme === DX_LENS_SCHEME) {
        return diskUri;
    }

    // Convert file:// to dxlens://
    return diskUri.with({ scheme: DX_LENS_SCHEME });
}

/**
 * Create a debounced version of a function
 * 
 * The debounced function delays invoking the provided function until
 * after the specified delay has elapsed since the last time it was invoked.
 * 
 * @param fn - The function to debounce
 * @param delay - The delay in milliseconds
 * @returns A debounced version of the function
 */
export function debounce<T extends (...args: unknown[]) => unknown>(
    fn: T,
    delay: number
): (...args: Parameters<T>) => void {
    let timeoutId: NodeJS.Timeout | undefined;

    return function debounced(...args: Parameters<T>): void {
        if (timeoutId !== undefined) {
            clearTimeout(timeoutId);
        }

        timeoutId = setTimeout(() => {
            fn(...args);
            timeoutId = undefined;
        }, delay);
    };
}

/**
 * Create a debounced function that can be cancelled
 * 
 * @param fn - The function to debounce
 * @param delay - The delay in milliseconds
 * @returns An object with the debounced function and a cancel method
 */
export function debounceCancellable<T extends (...args: unknown[]) => unknown>(
    fn: T,
    delay: number
): { call: (...args: Parameters<T>) => void; cancel: () => void } {
    let timeoutId: NodeJS.Timeout | undefined;

    return {
        call(...args: Parameters<T>): void {
            if (timeoutId !== undefined) {
                clearTimeout(timeoutId);
            }

            timeoutId = setTimeout(() => {
                fn(...args);
                timeoutId = undefined;
            }, delay);
        },

        cancel(): void {
            if (timeoutId !== undefined) {
                clearTimeout(timeoutId);
                timeoutId = undefined;
            }
        },
    };
}

/**
 * Get the document key for a URI (used for Map keys)
 * 
 * @param uri - The URI to get a key for
 * @returns A string key for the URI
 */
export function getDocumentKey(uri: vscode.Uri): string {
    // Normalize to file path for consistent keys
    const diskUri = getDiskUri(uri);
    return diskUri.fsPath.toLowerCase();
}

/**
 * Check if two URIs refer to the same file
 * 
 * @param uri1 - First URI
 * @param uri2 - Second URI
 * @returns true if both URIs refer to the same file
 */
export function isSameFile(uri1: vscode.Uri, uri2: vscode.Uri): boolean {
    return getDocumentKey(uri1) === getDocumentKey(uri2);
}
