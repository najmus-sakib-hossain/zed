/**
 * DX Generator Template Registry
 * 
 * Manages template discovery, loading, and generation.
 * Requirements: 2.1, 2.3, 5.1, 5.4
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { TemplateMetadata, GenerateResult, ParameterSchema } from './types';

/**
 * Template registry for managing templates
 */
export class TemplateRegistry {
    private templates: Map<string, TemplateMetadata> = new Map();
    private templatePaths: string[] = [];
    private workspaceRoot: string | undefined;

    constructor() {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            this.workspaceRoot = workspaceFolders[0].uri.fsPath;
            this.templatePaths = [
                path.join(this.workspaceRoot, '.dx', 'templates'),
            ];
        }
    }

    /**
     * Initialize the registry by discovering templates
     */
    async initialize(): Promise<void> {
        await this.discoverTemplates();
    }

    /**
     * Discover templates from configured paths
     */
    private async discoverTemplates(): Promise<void> {
        this.templates.clear();

        for (const templatePath of this.templatePaths) {
            if (!fs.existsSync(templatePath)) {
                continue;
            }

            const files = await fs.promises.readdir(templatePath);
            for (const file of files) {
                if (file.endsWith('.dxt') || file.endsWith('.dxt.hbs')) {
                    const fullPath = path.join(templatePath, file);
                    const metadata = await this.loadTemplateMetadata(fullPath);
                    if (metadata) {
                        this.templates.set(metadata.id, metadata);
                    }
                }
            }
        }
    }


    /**
     * Load template metadata from a file
     */
    private async loadTemplateMetadata(
        filePath: string
    ): Promise<TemplateMetadata | null> {
        try {
            const content = await fs.promises.readFile(filePath, 'utf-8');
            const fileName = path.basename(filePath, path.extname(filePath));
            const id = fileName.replace('.dxt', '');

            // Try to parse metadata from template header
            const metadata = this.parseTemplateHeader(content, id);
            return metadata;
        } catch (error) {
            console.error(`Failed to load template ${filePath}:`, error);
            return null;
        }
    }

    /**
     * Parse template header for metadata
     * Expects format:
     * {{!--
     * @name: Component Template
     * @description: Creates a React component
     * @version: 1.0.0
     * @param name: PascalCase - Component name
     * @param props: string - Props interface name
     * --}}
     */
    private parseTemplateHeader(
        content: string,
        defaultId: string
    ): TemplateMetadata {
        const metadata: TemplateMetadata = {
            id: defaultId,
            name: defaultId,
            description: '',
            version: '1.0.0',
            tags: [],
            parameters: [],
            outputPattern: `{{name}}.tsx`,
            dependencies: [],
        };

        // Look for metadata block
        const headerMatch = content.match(/\{\{!--\s*([\s\S]*?)\s*--\}\}/);
        if (headerMatch) {
            const headerContent = headerMatch[1];
            const lines = headerContent.split('\n');

            for (const line of lines) {
                const trimmed = line.trim();

                if (trimmed.startsWith('@name:')) {
                    metadata.name = trimmed.substring(6).trim();
                } else if (trimmed.startsWith('@description:')) {
                    metadata.description = trimmed.substring(13).trim();
                } else if (trimmed.startsWith('@version:')) {
                    metadata.version = trimmed.substring(9).trim();
                } else if (trimmed.startsWith('@author:')) {
                    metadata.author = trimmed.substring(8).trim();
                } else if (trimmed.startsWith('@tags:')) {
                    metadata.tags = trimmed.substring(6).trim().split(',').map(t => t.trim());
                } else if (trimmed.startsWith('@output:')) {
                    metadata.outputPattern = trimmed.substring(8).trim();
                } else if (trimmed.startsWith('@param')) {
                    const param = this.parseParamLine(trimmed);
                    if (param) {
                        metadata.parameters.push(param);
                    }
                }
            }
        }

        return metadata;
    }

    /**
     * Parse a @param line into ParameterSchema
     * Format: @param name: Type - Description
     */
    private parseParamLine(line: string): ParameterSchema | null {
        const match = line.match(/@param\s+(\w+):\s*(\w+)(?:\s*-\s*(.*))?/);
        if (!match) {
            return null;
        }

        const [, name, type, description] = match;
        const required = !type.startsWith('?');
        const valueType = type.replace('?', '') as ParameterSchema['valueType'];

        return {
            name,
            description: description || '',
            valueType,
            required,
            examples: [],
        };
    }

    /**
     * Get a template by ID
     */
    async getTemplate(id: string): Promise<TemplateMetadata | null> {
        // Refresh templates if not found
        if (!this.templates.has(id)) {
            await this.discoverTemplates();
        }
        return this.templates.get(id) || null;
    }

    /**
     * List all available templates
     */
    async listTemplates(): Promise<TemplateMetadata[]> {
        await this.discoverTemplates();
        return Array.from(this.templates.values());
    }

    /**
     * Search templates by query
     */
    async searchTemplates(query: string): Promise<TemplateMetadata[]> {
        const templates = await this.listTemplates();
        const lowerQuery = query.toLowerCase();

        return templates.filter((t) =>
            t.name.toLowerCase().includes(lowerQuery) ||
            t.description.toLowerCase().includes(lowerQuery) ||
            t.tags.some((tag) => tag.toLowerCase().includes(lowerQuery))
        );
    }


    /**
     * Generate content from a template
     */
    async generate(
        templateId: string,
        params: Record<string, string>
    ): Promise<GenerateResult> {
        const startTime = Date.now();

        try {
            const template = await this.getTemplate(templateId);
            if (!template) {
                return {
                    success: false,
                    error: `Template '${templateId}' not found`,
                };
            }

            // Find template file
            const templateFile = await this.findTemplateFile(templateId);
            if (!templateFile) {
                return {
                    success: false,
                    error: `Template file for '${templateId}' not found`,
                };
            }

            // Load template content
            const templateContent = await fs.promises.readFile(templateFile, 'utf-8');

            // Remove metadata header
            const contentWithoutHeader = templateContent.replace(
                /\{\{!--[\s\S]*?--\}\}\s*/,
                ''
            );

            // Render template with parameters
            const content = this.renderTemplate(contentWithoutHeader, params);

            const timeUs = (Date.now() - startTime) * 1000;
            const bytes = Buffer.byteLength(content, 'utf-8');
            const templateBytes = Buffer.byteLength(contentWithoutHeader, 'utf-8');
            const tokensSaved = Math.floor((bytes - templateBytes) / 4);

            return {
                success: true,
                content,
                bytes,
                timeUs,
                tokensSaved: Math.max(0, tokensSaved),
            };
        } catch (error) {
            return {
                success: false,
                error: `Generation failed: ${error}`,
            };
        }
    }

    /**
     * Find the template file for a given ID
     */
    private async findTemplateFile(templateId: string): Promise<string | null> {
        for (const templatePath of this.templatePaths) {
            const candidates = [
                path.join(templatePath, `${templateId}.dxt`),
                path.join(templatePath, `${templateId}.dxt.hbs`),
                path.join(templatePath, `${templateId}.hbs`),
            ];

            for (const candidate of candidates) {
                if (fs.existsSync(candidate)) {
                    return candidate;
                }
            }
        }
        return null;
    }

    /**
     * Simple template rendering with placeholder replacement
     * Supports: {{ name }}, {{ name | transform }}
     */
    private renderTemplate(
        template: string,
        params: Record<string, string>
    ): string {
        let result = template;

        // Replace simple placeholders: {{ name }}
        result = result.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, key) => {
            return params[key] || `{{ ${key} }}`;
        });

        // Replace placeholders with transforms: {{ name | transform }}
        result = result.replace(
            /\{\{\s*(\w+)\s*\|\s*(\w+)\s*\}\}/g,
            (_, key, transform) => {
                const value = params[key];
                if (!value) {
                    return `{{ ${key} | ${transform} }}`;
                }
                return this.applyTransform(value, transform);
            }
        );

        return result;
    }

    /**
     * Apply a transform to a value
     */
    private applyTransform(value: string, transform: string): string {
        switch (transform.toLowerCase()) {
            case 'lowercase':
            case 'lower':
                return value.toLowerCase();
            case 'uppercase':
            case 'upper':
                return value.toUpperCase();
            case 'pascalcase':
            case 'pascal':
                return this.toPascalCase(value);
            case 'camelcase':
            case 'camel':
                return this.toCamelCase(value);
            case 'snakecase':
            case 'snake':
            case 'snake_case':
                return this.toSnakeCase(value);
            case 'kebabcase':
            case 'kebab':
            case 'kebab-case':
                return this.toKebabCase(value);
            case 'capitalize':
                return value.charAt(0).toUpperCase() + value.slice(1);
            default:
                return value;
        }
    }

    private toPascalCase(str: string): string {
        return str
            .replace(/[-_\s]+(.)?/g, (_, c) => (c ? c.toUpperCase() : ''))
            .replace(/^(.)/, (c) => c.toUpperCase());
    }

    private toCamelCase(str: string): string {
        const pascal = this.toPascalCase(str);
        return pascal.charAt(0).toLowerCase() + pascal.slice(1);
    }

    private toSnakeCase(str: string): string {
        return str
            .replace(/([A-Z])/g, '_$1')
            .replace(/[-\s]+/g, '_')
            .toLowerCase()
            .replace(/^_/, '');
    }

    private toKebabCase(str: string): string {
        return str
            .replace(/([A-Z])/g, '-$1')
            .replace(/[_\s]+/g, '-')
            .toLowerCase()
            .replace(/^-/, '');
    }

    /**
     * Refresh templates from disk
     */
    async refresh(): Promise<void> {
        await this.discoverTemplates();
    }
}
