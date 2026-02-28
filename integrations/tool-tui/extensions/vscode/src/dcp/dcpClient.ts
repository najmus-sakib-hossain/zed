/**
 * DX DCP Client
 * 
 * Communicates with the dx-cli DCP commands and servers.
 * Requirements: 11.1-11.10
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import {
    DcpServerStatus,
    DcpServerMode,
    DcpTool,
    DcpResource,
    DcpMetrics,
    DcpInvocationResult,
    McpCompatibilityStatus,
    DcpConfig,
} from './types';

/**
 * Client for interacting with DCP CLI commands and servers
 */
export class DcpClient {
    private workspaceRoot: string | undefined;
    private dcpDir: string | undefined;
    private serverProcess: any = null;

    constructor() {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            this.workspaceRoot = workspaceFolders[0].uri.fsPath;
            this.dcpDir = path.join(this.workspaceRoot, '.dcp');
        }
    }

    /**
     * Get server status
     */
    async getServerStatus(): Promise<DcpServerStatus[]> {
        // Return mock status for now - in production this would query actual servers
        const config = await this.loadConfig();

        return [{
            name: 'default',
            port: config?.port || 9877,
            running: false,
            mode: config?.mode || 'hybrid',
        }];
    }

    /**
     * Get registered tools
     */
    async getTools(): Promise<DcpTool[]> {
        if (!this.dcpDir) return [];

        const toolsDir = path.join(this.dcpDir, 'tools');
        if (!fs.existsSync(toolsDir)) return [];

        const tools: DcpTool[] = [];
        const files = await fs.promises.readdir(toolsDir);

        for (const file of files) {
            if (file.endsWith('.json')) {
                const toolPath = path.join(toolsDir, file);
                const tool = await this.loadTool(toolPath);
                if (tool) tools.push(tool);
            }
        }

        return tools;
    }

    /**
     * Load tool from JSON file
     */
    private async loadTool(toolPath: string): Promise<DcpTool | null> {
        try {
            const content = await fs.promises.readFile(toolPath, 'utf-8');
            const data = JSON.parse(content);

            return {
                id: data.id || path.basename(toolPath, '.json'),
                name: data.name || data.id,
                description: data.description || '',
                inputSchema: data.inputSchema || { type: 'object' },
                outputSchema: data.outputSchema,
                capabilities: data.capabilities || 0,
                signed: data.signed || false,
                version: data.version,
            };
        } catch {
            return null;
        }
    }

    /**
     * Get available resources
     */
    async getResources(): Promise<DcpResource[]> {
        if (!this.dcpDir) return [];

        const resourcesDir = path.join(this.dcpDir, 'resources');
        if (!fs.existsSync(resourcesDir)) return [];

        const resources: DcpResource[] = [];
        const files = await fs.promises.readdir(resourcesDir);

        for (const file of files) {
            if (file.endsWith('.json')) {
                const resourcePath = path.join(resourcesDir, file);
                const resource = await this.loadResource(resourcePath);
                if (resource) resources.push(resource);
            }
        }

        return resources;
    }

    /**
     * Load resource from JSON file
     */
    private async loadResource(resourcePath: string): Promise<DcpResource | null> {
        try {
            const content = await fs.promises.readFile(resourcePath, 'utf-8');
            const data = JSON.parse(content);

            return {
                uri: data.uri || `file://${resourcePath}`,
                name: data.name || path.basename(resourcePath, '.json'),
                description: data.description,
                mimeType: data.mimeType,
                access: data.access || 'read',
            };
        } catch {
            return null;
        }
    }

    /**
     * Get performance metrics
     */
    async getMetrics(): Promise<DcpMetrics> {
        // Return mock metrics - in production this would query the server
        return {
            avgLatencyUs: 0,
            p99LatencyUs: 0,
            messagesPerSecond: 0,
            avgMessageSize: 0,
            totalMessages: 0,
            errorCount: 0,
        };
    }

    /**
     * Get MCP compatibility status
     */
    async getMcpCompatibility(): Promise<McpCompatibilityStatus> {
        const config = await this.loadConfig();

        return {
            available: config?.mcpCompat ?? true,
            version: '2024-11-05',
            suggestions: config?.mcpCompat ? [] : [
                'Enable MCP compatibility in dx.toml [dcp] section',
                'Run `dx dcp convert` to migrate MCP schemas',
            ],
        };
    }

    /**
     * Load DCP configuration
     */
    async loadConfig(): Promise<DcpConfig | null> {
        if (!this.workspaceRoot) return null;

        const configPath = path.join(this.workspaceRoot, 'dx.toml');
        if (!fs.existsSync(configPath)) return null;

        try {
            const content = await fs.promises.readFile(configPath, 'utf-8');

            // Simple TOML parsing for [dcp] section
            const dcpMatch = content.match(/\[dcp\]([\s\S]*?)(?=\[|$)/);
            if (!dcpMatch) return null;

            const dcpSection = dcpMatch[1];

            return {
                port: parseInt(this.extractTomlValue(dcpSection, 'port') || '9877'),
                mode: (this.extractTomlValue(dcpSection, 'mode') || 'hybrid') as DcpServerMode,
                mcpCompat: this.extractTomlValue(dcpSection, 'mcp_compat') !== 'false',
                toolsPath: this.extractTomlValue(dcpSection, 'tools_path'),
                metricsEnabled: this.extractTomlValue(dcpSection, 'metrics') !== 'false',
            };
        } catch {
            return null;
        }
    }

    /**
     * Extract value from TOML content
     */
    private extractTomlValue(content: string, key: string): string | undefined {
        const match = content.match(new RegExp(`^${key}\\s*=\\s*"?([^"\\n]*)"?`, 'm'));
        return match ? match[1].trim() : undefined;
    }

    /**
     * Start DCP server
     */
    async startServer(port?: number): Promise<boolean> {
        const config = await this.loadConfig();
        const serverPort = port || config?.port || 9877;

        const terminal = vscode.window.createTerminal('DX DCP Server');
        terminal.show();
        terminal.sendText(`dx dcp serve --port ${serverPort}`);

        return true;
    }

    /**
     * Stop DCP server
     */
    async stopServer(): Promise<boolean> {
        // In production, this would send a shutdown signal to the server
        vscode.window.showInformationMessage('DCP server stop requested');
        return true;
    }

    /**
     * Invoke a tool
     */
    async invokeTool(toolId: string, args: Record<string, any>): Promise<DcpInvocationResult> {
        const argsJson = JSON.stringify(args);

        // Use terminal for now - in production this would use the DCP protocol
        const terminal = vscode.window.createTerminal('DX DCP');
        terminal.show();
        terminal.sendText(`dx dcp tools invoke ${toolId} '${argsJson}'`);

        return {
            success: true,
            result: 'Invocation sent to terminal',
        };
    }

    /**
     * Register a new tool
     */
    async registerTool(schemaPath: string): Promise<boolean> {
        const terminal = vscode.window.createTerminal('DX DCP');
        terminal.show();
        terminal.sendText(`dx dcp tools register "${schemaPath}"`);
        return true;
    }

    /**
     * Sign a tool definition
     */
    async signTool(toolId: string): Promise<boolean> {
        const terminal = vscode.window.createTerminal('DX DCP');
        terminal.show();
        terminal.sendText(`dx dcp tools sign ${toolId}`);
        return true;
    }

    /**
     * Run benchmarks
     */
    async runBenchmarks(): Promise<void> {
        const terminal = vscode.window.createTerminal('DX DCP Bench');
        terminal.show();
        terminal.sendText('dx dcp bench');
    }

    /**
     * Convert MCP schema to DCP
     */
    async convertFromMcp(inputPath: string, outputPath: string): Promise<boolean> {
        const terminal = vscode.window.createTerminal('DX DCP');
        terminal.show();
        terminal.sendText(`dx dcp convert --input "${inputPath}" --output "${outputPath}"`);
        return true;
    }
}
