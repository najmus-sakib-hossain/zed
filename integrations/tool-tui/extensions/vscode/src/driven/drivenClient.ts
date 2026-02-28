/**
 * DX Driven Client
 * 
 * Communicates with the dx-cli driven commands.
 * Requirements: 9.1-9.10
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import {
    EditorSyncStatus,
    SpecMetadata,
    SpecStatus,
    HookDefinition,
    SteeringFile,
    SteeringInclusionMode,
    DrivenTemplate,
    DrivenConfig,
} from './types';

/**
 * Client for interacting with driven CLI commands
 */
export class DrivenClient {
    private workspaceRoot: string | undefined;
    private drivenDir: string | undefined;

    constructor() {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            this.workspaceRoot = workspaceFolders[0].uri.fsPath;
            this.drivenDir = path.join(this.workspaceRoot, '.driven');
        }
    }

    /**
     * Check if driven is initialized in the workspace
     */
    isInitialized(): boolean {
        if (!this.drivenDir) return false;
        return fs.existsSync(this.drivenDir);
    }

    /**
     * Get sync status for all editors
     */
    async getSyncStatus(): Promise<EditorSyncStatus[]> {
        const editors = ['cursor', 'copilot', 'windsurf', 'claude', 'aider', 'cline'];
        const config = await this.loadConfig();

        return editors.map(editor => ({
            editor,
            enabled: config?.editors?.[editor] ?? false,
            status: config?.editors?.[editor] ? 'synced' : 'disabled',
        }));
    }

    /**
     * Get all specifications
     */
    async getSpecs(): Promise<SpecMetadata[]> {
        if (!this.drivenDir) return [];

        const specsDir = path.join(this.drivenDir, 'specs');
        if (!fs.existsSync(specsDir)) return [];

        const specs: SpecMetadata[] = [];
        const entries = await fs.promises.readdir(specsDir, { withFileTypes: true });

        for (const entry of entries) {
            if (entry.isDirectory()) {
                const specPath = path.join(specsDir, entry.name);
                const spec = await this.loadSpecMetadata(specPath, entry.name);
                if (spec) specs.push(spec);
            }
        }

        return specs.sort((a, b) => a.id.localeCompare(b.id));
    }

    /**
     * Load spec metadata from directory
     */
    private async loadSpecMetadata(
        specPath: string,
        dirName: string
    ): Promise<SpecMetadata | null> {
        try {
            const stat = await fs.promises.stat(specPath);
            const specFile = path.join(specPath, 'spec.md');
            const planFile = path.join(specPath, 'plan.md');
            const tasksFile = path.join(specPath, 'tasks.md');

            // Determine status based on which files exist
            let status: SpecStatus = 'draft';
            if (fs.existsSync(tasksFile)) {
                status = 'tasks-ready';
            } else if (fs.existsSync(planFile)) {
                status = 'planned';
            } else if (fs.existsSync(specFile)) {
                status = 'specified';
            }

            // Extract ID and name from directory name (e.g., "001-feature-name")
            const match = dirName.match(/^(\d+)-(.+)$/);
            const id = match ? match[1] : dirName;
            const name = match ? match[2].replace(/-/g, ' ') : dirName;

            return {
                id,
                name,
                path: specPath,
                status,
                created: stat.birthtime,
                modified: stat.mtime,
            };
        } catch {
            return null;
        }
    }

    /**
     * Get all hooks
     */
    async getHooks(): Promise<HookDefinition[]> {
        if (!this.drivenDir) return [];

        const hooksDir = path.join(this.drivenDir, 'hooks');
        if (!fs.existsSync(hooksDir)) return [];

        const hooks: HookDefinition[] = [];
        const files = await fs.promises.readdir(hooksDir);

        for (const file of files) {
            if (file.endsWith('.toml')) {
                const hookPath = path.join(hooksDir, file);
                const hook = await this.loadHook(hookPath);
                if (hook) hooks.push(hook);
            }
        }

        return hooks;
    }

    /**
     * Load hook from TOML file
     */
    private async loadHook(hookPath: string): Promise<HookDefinition | null> {
        try {
            const content = await fs.promises.readFile(hookPath, 'utf-8');
            // Simple TOML parsing for hook files
            const name = this.extractTomlValue(content, 'name') || path.basename(hookPath, '.toml');
            const description = this.extractTomlValue(content, 'description') || '';
            const enabled = this.extractTomlValue(content, 'enabled') === 'true';
            const triggerType = this.extractTomlValue(content, 'type', '[trigger]') as any || 'manual';
            const pattern = this.extractTomlValue(content, 'pattern', '[trigger]');
            const actionType = this.extractTomlValue(content, 'type', '[action]') as any || 'shell';
            const command = this.extractTomlValue(content, 'command', '[action]');
            const messageContent = this.extractTomlValue(content, 'content', '[action]');

            return {
                name,
                description,
                enabled,
                triggerType,
                trigger: { type: triggerType, pattern },
                action: { type: actionType, command, content: messageContent },
                configPath: hookPath,
            };
        } catch {
            return null;
        }
    }

    /**
     * Extract value from TOML content
     */
    private extractTomlValue(content: string, key: string, section?: string): string | undefined {
        const lines = content.split('\n');
        let inSection = !section;

        for (const line of lines) {
            const trimmed = line.trim();

            if (section && trimmed === section) {
                inSection = true;
                continue;
            }

            if (inSection && trimmed.startsWith('[') && trimmed !== section) {
                inSection = false;
                continue;
            }

            if (inSection) {
                const match = trimmed.match(new RegExp(`^${key}\\s*=\\s*"?([^"]*)"?`));
                if (match) return match[1];
            }
        }
        return undefined;
    }

    /**
     * Get all steering files
     */
    async getSteeringFiles(): Promise<SteeringFile[]> {
        if (!this.drivenDir) return [];

        const steeringDir = path.join(this.drivenDir, 'steering');
        if (!fs.existsSync(steeringDir)) return [];

        const files: SteeringFile[] = [];
        const entries = await fs.promises.readdir(steeringDir);

        for (const entry of entries) {
            if (entry.endsWith('.md')) {
                const filePath = path.join(steeringDir, entry);
                const steering = await this.loadSteeringFile(filePath, entry);
                if (steering) files.push(steering);
            }
        }

        return files;
    }

    /**
     * Load steering file metadata
     */
    private async loadSteeringFile(
        filePath: string,
        fileName: string
    ): Promise<SteeringFile | null> {
        try {
            const content = await fs.promises.readFile(filePath, 'utf-8');

            // Parse frontmatter
            const frontmatterMatch = content.match(/^---\s*\n([\s\S]*?)\n---/);
            let inclusionMode: SteeringInclusionMode = 'always';
            let fileMatchPattern: string | undefined;
            let contextKey: string | undefined;
            let description: string | undefined;

            if (frontmatterMatch) {
                const frontmatter = frontmatterMatch[1];
                const inclusionMatch = frontmatter.match(/inclusion:\s*(\w+)/);
                if (inclusionMatch) {
                    inclusionMode = inclusionMatch[1] as SteeringInclusionMode;
                }
                const patternMatch = frontmatter.match(/fileMatchPattern:\s*["']?([^"'\n]+)["']?/);
                if (patternMatch) {
                    fileMatchPattern = patternMatch[1];
                }
                const keyMatch = frontmatter.match(/contextKey:\s*["']?([^"'\n]+)["']?/);
                if (keyMatch) {
                    contextKey = keyMatch[1];
                }
                const descMatch = frontmatter.match(/description:\s*["']?([^"'\n]+)["']?/);
                if (descMatch) {
                    description = descMatch[1];
                }
            }

            return {
                name: fileName.replace('.md', ''),
                path: filePath,
                inclusionMode,
                fileMatchPattern,
                contextKey,
                description,
            };
        } catch {
            return null;
        }
    }

    /**
     * Get available templates
     */
    async getTemplates(): Promise<DrivenTemplate[]> {
        // Return built-in templates for now
        return [
            { id: 'rust-workspace', name: 'Rust Workspace', description: 'Multi-crate Rust workspace setup', category: 'project', tags: ['rust', 'workspace'] },
            { id: 'typescript-lib', name: 'TypeScript Library', description: 'TypeScript library with testing', category: 'project', tags: ['typescript', 'library'] },
            { id: 'architect', name: 'Architect Persona', description: 'System architect AI persona', category: 'persona', tags: ['architect', 'design'] },
            { id: 'developer', name: 'Developer Persona', description: 'Senior developer AI persona', category: 'persona', tags: ['developer', 'code'] },
            { id: 'rust-idioms', name: 'Rust Idioms', description: 'Rust best practices and idioms', category: 'standard', tags: ['rust', 'idioms'] },
            { id: 'code-review', name: 'Code Review', description: 'Code review workflow', category: 'workflow', tags: ['review', 'quality'] },
        ];
    }

    /**
     * Load driven configuration
     */
    async loadConfig(): Promise<DrivenConfig | null> {
        if (!this.drivenDir) return null;

        const configPath = path.join(this.drivenDir, 'config.toml');
        if (!fs.existsSync(configPath)) return null;

        try {
            const content = await fs.promises.readFile(configPath, 'utf-8');
            // Simple config parsing
            return {
                editors: {
                    cursor: this.extractTomlValue(content, 'cursor', '[editors]') === 'true',
                    copilot: this.extractTomlValue(content, 'copilot', '[editors]') === 'true',
                    windsurf: this.extractTomlValue(content, 'windsurf', '[editors]') === 'true',
                    claude: this.extractTomlValue(content, 'claude', '[editors]') === 'true',
                    aider: this.extractTomlValue(content, 'aider', '[editors]') === 'true',
                    cline: this.extractTomlValue(content, 'cline', '[editors]') === 'true',
                },
                sync: {
                    sourceOfTruth: this.extractTomlValue(content, 'source_of_truth', '[sync]') || '.driven/rules.drv',
                    watch: this.extractTomlValue(content, 'watch', '[sync]') === 'true',
                    debounceMs: parseInt(this.extractTomlValue(content, 'debounce_ms', '[sync]') || '500'),
                },
                spec: {
                    directory: this.extractTomlValue(content, 'directory', '[spec]') || '.driven/specs',
                    autoBranch: this.extractTomlValue(content, 'auto_branch', '[spec]') === 'true',
                    constitution: this.extractTomlValue(content, 'constitution', '[spec]'),
                },
                hooks: {
                    directory: this.extractTomlValue(content, 'directory', '[hooks]') || '.driven/hooks',
                    enabled: this.extractTomlValue(content, 'enabled', '[hooks]') !== 'false',
                },
            };
        } catch {
            return null;
        }
    }

    /**
     * Toggle hook enabled state
     */
    async toggleHook(hookName: string, enabled: boolean): Promise<boolean> {
        const hooks = await this.getHooks();
        const hook = hooks.find(h => h.name === hookName);
        if (!hook) return false;

        try {
            let content = await fs.promises.readFile(hook.configPath, 'utf-8');
            content = content.replace(
                /enabled\s*=\s*(true|false)/,
                `enabled = ${enabled}`
            );
            await fs.promises.writeFile(hook.configPath, content);
            return true;
        } catch {
            return false;
        }
    }

    /**
     * Run sync command
     */
    async runSync(): Promise<boolean> {
        return this.runCliCommand('driven', 'sync');
    }

    /**
     * Run init command
     */
    async runInit(interactive: boolean = false): Promise<boolean> {
        const args = interactive ? ['init', '-i'] : ['init'];
        return this.runCliCommand('driven', ...args);
    }

    /**
     * Run validate command
     */
    async runValidate(): Promise<boolean> {
        return this.runCliCommand('driven', 'validate');
    }

    /**
     * Run a CLI command
     */
    private async runCliCommand(subcommand: string, ...args: string[]): Promise<boolean> {
        const terminal = vscode.window.createTerminal('DX Driven');
        terminal.show();
        terminal.sendText(`dx ${subcommand} ${args.join(' ')}`);
        return true;
    }
}
