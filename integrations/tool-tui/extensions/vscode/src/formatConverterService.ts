/**
 * Format Converter Service
 * 
 * Handles conversion between DX and other formats (JSON, YAML, TOML, TOON).
 * Uses WASM bindings when available, with TypeScript fallback.
 * 
 * Requirements: 2.1, 2.2
 */

/**
 * WASM format converter interface (when available)
 */
interface WasmFormatConverter {
    dx_to_json(dx: string): string;
    dx_to_yaml(dx: string): string;
    dx_to_toml(dx: string): string;
    dx_to_toon_wasm(dx: string): string;
}

/**
 * Format Converter Service
 * 
 * Provides format conversion between DX and other data formats.
 */
export class FormatConverterService {
    private wasmConverter: WasmFormatConverter | null = null;

    /**
     * Initialize WASM converter if available
     */
    async initWasm(wasmModule: any): Promise<void> {
        if (wasmModule && typeof wasmModule.dx_to_json === 'function') {
            this.wasmConverter = wasmModule;
        }
    }

    /**
     * Convert DX format to JSON
     * 
     * @param dxContent - DX format content
     * @returns JSON string
     */
    async dxToJson(dxContent: string): Promise<string> {
        if (this.wasmConverter) {
            try {
                return this.wasmConverter.dx_to_json(dxContent);
            } catch (error) {
                console.warn('WASM dx_to_json failed, using fallback:', error);
            }
        }
        return this.fallbackDxToJson(dxContent);
    }

    /**
     * Convert DX format to YAML
     * 
     * @param dxContent - DX format content
     * @returns YAML string
     */
    async dxToYaml(dxContent: string): Promise<string> {
        if (this.wasmConverter) {
            try {
                return this.wasmConverter.dx_to_yaml(dxContent);
            } catch (error) {
                console.warn('WASM dx_to_yaml failed, using fallback:', error);
            }
        }
        return this.fallbackDxToYaml(dxContent);
    }

    /**
     * Convert DX format to TOML
     * 
     * @param dxContent - DX format content
     * @returns TOML string
     */
    async dxToToml(dxContent: string): Promise<string> {
        if (this.wasmConverter) {
            try {
                return this.wasmConverter.dx_to_toml(dxContent);
            } catch (error) {
                console.warn('WASM dx_to_toml failed, using fallback:', error);
            }
        }
        return this.fallbackDxToToml(dxContent);
    }

    /**
     * Convert DX format to TOON
     * 
     * @param dxContent - DX format content
     * @returns TOON string
     */
    async dxToToon(dxContent: string): Promise<string> {
        if (this.wasmConverter) {
            try {
                return this.wasmConverter.dx_to_toon_wasm(dxContent);
            } catch (error) {
                console.warn('WASM dx_to_toon failed, using fallback:', error);
            }
        }
        return this.fallbackDxToToon(dxContent);
    }

    /**
     * Convert JSON to DX format
     * 
     * @param jsonContent - JSON string
     * @returns DX format string
     */
    async jsonToDx(jsonContent: string): Promise<string> {
        // Parse JSON and convert to DX
        try {
            const obj = JSON.parse(jsonContent);
            return this.objectToDx(obj);
        } catch (error) {
            throw new Error(`JSON parse error: ${error}`);
        }
    }

    /**
     * Convert YAML to DX format
     * 
     * @param yamlContent - YAML string
     * @returns DX format string
     */
    async yamlToDx(yamlContent: string): Promise<string> {
        // Simple YAML to DX conversion
        // For full YAML support, use a proper YAML parser
        const lines = yamlContent.split('\n');
        const result: string[] = [];

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed.startsWith('#')) continue;

            const colonIndex = trimmed.indexOf(':');
            if (colonIndex > 0) {
                const key = trimmed.substring(0, colonIndex).trim();
                const value = trimmed.substring(colonIndex + 1).trim();
                result.push(`${key}:${value}`);
            }
        }

        return result.join('\n');
    }

    /**
     * Convert TOML to DX format
     * 
     * @param tomlContent - TOML string
     * @returns DX format string
     */
    async tomlToDx(tomlContent: string): Promise<string> {
        // Simple TOML to DX conversion
        const lines = tomlContent.split('\n');
        const result: string[] = [];
        let currentSection = '';

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed.startsWith('#')) continue;

            // Section header
            if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
                currentSection = trimmed.slice(1, -1);
                continue;
            }

            // Key-value pair
            const eqIndex = trimmed.indexOf('=');
            if (eqIndex > 0) {
                const key = trimmed.substring(0, eqIndex).trim();
                let value = trimmed.substring(eqIndex + 1).trim();

                // Remove quotes from strings
                if ((value.startsWith('"') && value.endsWith('"')) ||
                    (value.startsWith("'") && value.endsWith("'"))) {
                    value = value.slice(1, -1);
                }

                const fullKey = currentSection ? `${currentSection}.${key}` : key;
                result.push(`${fullKey}:${value}`);
            }
        }

        return result.join('\n');
    }

    /**
     * Convert TOON to DX format
     * 
     * @param toonContent - TOON string
     * @returns DX format string
     */
    async toonToDx(toonContent: string): Promise<string> {
        // Simple TOON to DX conversion
        const lines = toonContent.split('\n');
        const result: string[] = [];

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed) continue;

            // TOON format: key "value" or key value
            const spaceIndex = trimmed.indexOf(' ');
            if (spaceIndex > 0) {
                const key = trimmed.substring(0, spaceIndex);
                let value = trimmed.substring(spaceIndex + 1).trim();

                // Remove quotes
                if (value.startsWith('"') && value.endsWith('"')) {
                    value = value.slice(1, -1);
                }

                result.push(`${key}:${value}`);
            }
        }

        return result.join('\n');
    }

    /**
     * Convert CSV to DX format
     * 
     * @param csvContent - CSV string
     * @returns DX format string
     */
    async csvToDx(csvContent: string): Promise<string> {
        const lines = csvContent.split('\n').filter(l => l.trim());
        if (lines.length === 0) return '';

        // First line is headers
        const headers = this.parseCsvLine(lines[0]);
        const result: string[] = [];

        // Create table header
        result.push(`data=${headers.join(' ')}`);

        // Add rows
        for (let i = 1; i < lines.length; i++) {
            const values = this.parseCsvLine(lines[i]);
            result.push(values.join(' '));
        }

        return result.join('\n');
    }

    // ========================================================================
    // Fallback implementations (TypeScript-based)
    // ========================================================================

    private fallbackDxToJson(dxContent: string): string {
        const obj = this.parseDxToObject(dxContent);
        return JSON.stringify(obj, null, 2);
    }

    private fallbackDxToYaml(dxContent: string): string {
        const obj = this.parseDxToObject(dxContent);
        return this.objectToYaml(obj, 0);
    }

    private fallbackDxToToml(dxContent: string): string {
        const obj = this.parseDxToObject(dxContent);
        return this.objectToToml(obj);
    }

    private fallbackDxToToon(dxContent: string): string {
        const obj = this.parseDxToObject(dxContent);
        return this.objectToToon(obj, 0);
    }

    /**
     * Parse DX content to a JavaScript object
     */
    private parseDxToObject(dxContent: string): Record<string, any> {
        const result: Record<string, any> = {};
        const lines = dxContent.split('\n');

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('#')) continue;

            // Handle key:value pairs
            const colonIndex = trimmed.indexOf(':');
            if (colonIndex > 0) {
                const key = trimmed.substring(0, colonIndex).trim();
                const value = trimmed.substring(colonIndex + 1).trim();
                this.setNestedValue(result, key, this.parseValue(value));
            }

            // Handle arrays with >
            const arrowIndex = trimmed.indexOf('>');
            if (arrowIndex > 0) {
                const key = trimmed.substring(0, arrowIndex).trim();
                const values = trimmed.substring(arrowIndex + 1).split('|').map(v => this.parseValue(v.trim()));
                this.setNestedValue(result, key, values);
            }
        }

        return result;
    }

    /**
     * Parse a value string to appropriate type
     */
    private parseValue(value: string): any {
        // Boolean
        if (value === '+' || value === 'true') return true;
        if (value === '-' || value === 'false') return false;

        // Null
        if (value === '~' || value === 'null') return null;

        // Number
        if (/^-?\d+$/.test(value)) return parseInt(value, 10);
        if (/^-?\d+\.\d+$/.test(value)) return parseFloat(value);

        // String (remove quotes if present)
        if ((value.startsWith('"') && value.endsWith('"')) ||
            (value.startsWith("'") && value.endsWith("'"))) {
            return value.slice(1, -1);
        }

        return value;
    }

    /**
     * Set a nested value in an object using dot notation
     */
    private setNestedValue(obj: Record<string, any>, path: string, value: any): void {
        const parts = path.split('.');
        let current = obj;

        for (let i = 0; i < parts.length - 1; i++) {
            const part = parts[i];
            if (!(part in current)) {
                current[part] = {};
            }
            current = current[part];
        }

        current[parts[parts.length - 1]] = value;
    }

    /**
     * Convert object to DX format
     */
    private objectToDx(obj: Record<string, any>, prefix = ''): string {
        const lines: string[] = [];

        for (const [key, value] of Object.entries(obj)) {
            const fullKey = prefix ? `${prefix}.${key}` : key;

            if (value === null) {
                lines.push(`${fullKey}:~`);
            } else if (typeof value === 'boolean') {
                lines.push(`${fullKey}:${value ? '+' : '-'}`);
            } else if (typeof value === 'number') {
                lines.push(`${fullKey}:${value}`);
            } else if (typeof value === 'string') {
                lines.push(`${fullKey}:${value}`);
            } else if (Array.isArray(value)) {
                const items = value.map(v => String(v)).join('|');
                lines.push(`${fullKey}>${items}`);
            } else if (typeof value === 'object') {
                lines.push(this.objectToDx(value, fullKey));
            }
        }

        return lines.join('\n');
    }

    /**
     * Convert object to YAML format
     */
    private objectToYaml(obj: Record<string, any>, indent: number): string {
        const lines: string[] = [];
        const indentStr = '  '.repeat(indent);

        for (const [key, value] of Object.entries(obj)) {
            if (value === null) {
                lines.push(`${indentStr}${key}: null`);
            } else if (typeof value === 'boolean') {
                lines.push(`${indentStr}${key}: ${value}`);
            } else if (typeof value === 'number') {
                lines.push(`${indentStr}${key}: ${value}`);
            } else if (typeof value === 'string') {
                // Quote strings with special characters
                if (value.includes(':') || value.includes('#') || value.includes('\n')) {
                    lines.push(`${indentStr}${key}: "${value.replace(/"/g, '\\"')}"`);
                } else {
                    lines.push(`${indentStr}${key}: ${value}`);
                }
            } else if (Array.isArray(value)) {
                lines.push(`${indentStr}${key}:`);
                for (const item of value) {
                    lines.push(`${indentStr}  - ${item}`);
                }
            } else if (typeof value === 'object') {
                lines.push(`${indentStr}${key}:`);
                lines.push(this.objectToYaml(value, indent + 1));
            }
        }

        return lines.join('\n');
    }

    /**
     * Convert object to TOML format
     */
    private objectToToml(obj: Record<string, any>, section = ''): string {
        const lines: string[] = [];
        const nested: [string, Record<string, any>][] = [];

        for (const [key, value] of Object.entries(obj)) {
            if (value === null) {
                lines.push(`${key} = ""`);
            } else if (typeof value === 'boolean') {
                lines.push(`${key} = ${value}`);
            } else if (typeof value === 'number') {
                lines.push(`${key} = ${value}`);
            } else if (typeof value === 'string') {
                lines.push(`${key} = "${value.replace(/"/g, '\\"')}"`);
            } else if (Array.isArray(value)) {
                const items = value.map(v => typeof v === 'string' ? `"${v}"` : String(v));
                lines.push(`${key} = [${items.join(', ')}]`);
            } else if (typeof value === 'object') {
                nested.push([key, value]);
            }
        }

        // Add nested sections
        for (const [key, value] of nested) {
            const sectionName = section ? `${section}.${key}` : key;
            lines.push('');
            lines.push(`[${sectionName}]`);
            lines.push(this.objectToToml(value, sectionName));
        }

        return lines.join('\n');
    }

    /**
     * Convert object to TOON format
     */
    private objectToToon(obj: Record<string, any>, indent: number): string {
        const lines: string[] = [];
        const indentStr = '  '.repeat(indent);

        for (const [key, value] of Object.entries(obj)) {
            if (value === null) {
                lines.push(`${indentStr}${key} null`);
            } else if (typeof value === 'boolean') {
                lines.push(`${indentStr}${key} ${value}`);
            } else if (typeof value === 'number') {
                lines.push(`${indentStr}${key} ${value}`);
            } else if (typeof value === 'string') {
                lines.push(`${indentStr}${key} "${value}"`);
            } else if (Array.isArray(value)) {
                const items = value.map(v => typeof v === 'string' ? `"${v}"` : String(v));
                lines.push(`${indentStr}${key}[${value.length}]: ${items.join(', ')}`);
            } else if (typeof value === 'object') {
                lines.push(`${indentStr}${key}`);
                lines.push(this.objectToToon(value, indent + 1));
            }
        }

        return lines.join('\n');
    }

    /**
     * Parse a CSV line handling quoted values
     */
    private parseCsvLine(line: string): string[] {
        const result: string[] = [];
        let current = '';
        let inQuotes = false;

        for (let i = 0; i < line.length; i++) {
            const char = line[i];

            if (char === '"') {
                inQuotes = !inQuotes;
            } else if (char === ',' && !inQuotes) {
                result.push(current.trim());
                current = '';
            } else {
                current += char;
            }
        }

        result.push(current.trim());
        return result;
    }
}
