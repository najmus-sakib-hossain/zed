#!/usr/bin/env bun
/**
 * Cross-platform compatibility layer for OpenClaw
 * Makes OpenClaw work natively on Windows without WSL
 */

import os from 'os';
import path from 'path';
import fs from 'fs';

export type Platform = 'win32' | 'darwin' | 'linux';

/**
 * Get current platform
 */
export function getPlatform(): Platform {
    return os.platform() as Platform;
}

/**
 * Check if running on Windows
 */
export function isWindows(): boolean {
    return getPlatform() === 'win32';
}

/**
 * Check if running on macOS
 */
export function isMacOS(): boolean {
    return getPlatform() === 'darwin';
}

/**
 * Check if running on Linux
 */
export function isLinux(): boolean {
    return getPlatform() === 'linux';
}

/**
 * Get platform-specific paths
 */
export function getPlatformPaths() {
    const platform = getPlatform();

    if (platform === 'win32') {
        return {
            home: process.env.USERPROFILE || 'C:\\Users\\Default',
            config: path.join(process.env.APPDATA || 'C:\\Users\\Default\\AppData\\Roaming', 'dx'),
            cache: path.join(process.env.LOCALAPPDATA || 'C:\\Users\\Default\\AppData\\Local', 'dx'),
            temp: process.env.TEMP || 'C:\\Windows\\Temp',
        };
    }

    // Unix-like systems
    const home = process.env.HOME || '/tmp';
    return {
        home,
        config: path.join(home, '.config', 'dx'),
        cache: path.join(home, '.cache', 'dx'),
        temp: process.env.TMPDIR || '/tmp',
    };
}

/**
 * Normalize path separators for current platform
 */
export function normalizePath(p: string): string {
    if (isWindows()) {
        return p.replace(/\//g, '\\');
    }
    return p.replace(/\\/g, '/');
}

/**
 * Execute a shell command (cross-platform)
 */
export async function execCommand(command: string): Promise<{ stdout: string; stderr: string; code: number }> {
    const { spawn } = await import('child_process');

    return new Promise((resolve, reject) => {
        const shell = isWindows() ? 'cmd' : 'sh';
        const shellArg = isWindows() ? '/C' : '-c';

        const proc = spawn(shell, [shellArg, command], {
            stdio: ['pipe', 'pipe', 'pipe'],
        });

        let stdout = '';
        let stderr = '';

        proc.stdout?.on('data', (data) => {
            stdout += data.toString();
        });

        proc.stderr?.on('data', (data) => {
            stderr += data.toString();
        });

        proc.on('close', (code) => {
            resolve({ stdout, stderr, code: code || 0 });
        });

        proc.on('error', (err) => {
            reject(err);
        });
    });
}

/**
 * Remove directory recursively (cross-platform)
 * Replaces: execSync('rm -rf dir')
 */
export function removeDir(dir: string): void {
    if (fs.existsSync(dir)) {
        fs.rmSync(dir, { recursive: true, force: true });
    }
}

/**
 * Create directory recursively (cross-platform)
 * Replaces: execSync('mkdir -p dir')
 */
export function createDir(dir: string): void {
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
    }
}

/**
 * Copy file or directory (cross-platform)
 * Replaces: execSync('cp -r src dest')
 */
export function copy(src: string, dest: string): void {
    const stat = fs.statSync(src);

    if (stat.isDirectory()) {
        createDir(dest);
        const entries = fs.readdirSync(src);
        for (const entry of entries) {
            copy(path.join(src, entry), path.join(dest, entry));
        }
    } else {
        fs.copyFileSync(src, dest);
    }
}

/**
 * Find files matching pattern (cross-platform)
 * Replaces: execSync('find . -name "*.ts"')
 */
export function findFiles(dir: string, pattern: RegExp): string[] {
    const results: string[] = [];

    function walk(currentDir: string) {
        const entries = fs.readdirSync(currentDir, { withFileTypes: true });

        for (const entry of entries) {
            const fullPath = path.join(currentDir, entry.name);

            if (entry.isDirectory()) {
                walk(fullPath);
            } else if (pattern.test(entry.name)) {
                results.push(fullPath);
            }
        }
    }

    walk(dir);
    return results;
}

/**
 * Get environment variable with fallback
 */
export function getEnv(key: string, fallback: string = ''): string {
    return process.env[key] || fallback;
}

/**
 * Set environment variable
 */
export function setEnv(key: string, value: string): void {
    process.env[key] = value;
}

/**
 * Check if a command exists in PATH
 */
export async function commandExists(command: string): Promise<boolean> {
    try {
        const checkCmd = isWindows() ? `where ${command}` : `which ${command}`;
        const result = await execCommand(checkCmd);
        return result.code === 0;
    } catch {
        return false;
    }
}

/**
 * Get line ending for current platform
 */
export function getLineEnding(): string {
    return isWindows() ? '\r\n' : '\n';
}

/**
 * Ensure paths are initialized
 */
export function ensurePaths(): void {
    const paths = getPlatformPaths();
    createDir(paths.config);
    createDir(paths.cache);
}

// Initialize on import
ensurePaths();
