/**
 * DX Generator Types
 * 
 * Type definitions for the VS Code generator integration.
 * Requirements: 2.1, 2.3, 2.4
 */

/**
 * Template metadata from the registry
 */
export interface TemplateMetadata {
    /** Unique template identifier */
    id: string;
    /** Human-readable name */
    name: string;
    /** Description */
    description: string;
    /** Version (semver) */
    version: string;
    /** Author information */
    author?: string;
    /** Category tags */
    tags: string[];
    /** Parameter schema */
    parameters: ParameterSchema[];
    /** Output file pattern */
    outputPattern: string;
    /** Dependencies on other templates */
    dependencies: string[];
}

/**
 * Parameter schema for documentation and validation
 */
export interface ParameterSchema {
    name: string;
    description: string;
    valueType: PlaceholderValueType;
    required: boolean;
    default?: string | number | boolean;
    examples: string[];
}

/**
 * Supported placeholder value types
 */
export type PlaceholderValueType =
    | 'string'
    | 'PascalCase'
    | 'camelCase'
    | 'snake_case'
    | 'kebab-case'
    | 'UPPER_CASE'
    | 'lowercase'
    | 'integer'
    | 'float'
    | 'boolean'
    | 'date'
    | 'array';

/**
 * Trigger definition for auto-generation
 */
export interface TriggerDefinition {
    /** Regex pattern to match (e.g., /\/\/gen:(\w+)/) */
    pattern: RegExp;
    /** Template ID to invoke */
    templateId: string;
    /** Optional function to extract params from match */
    paramExtractor?: (match: RegExpMatchArray) => Record<string, string>;
}

/**
 * Result of trigger detection
 */
export interface TriggerMatch {
    /** The trigger definition that matched */
    trigger: TriggerDefinition;
    /** The regex match result */
    match: RegExpMatchArray;
    /** Extracted parameters */
    params: Record<string, string>;
    /** Start position of the trigger in the line */
    startIndex: number;
    /** End position of the trigger in the line */
    endIndex: number;
}

/**
 * Generation request for the generator
 */
export interface GenerateRequest {
    /** Template identifier */
    template: string;
    /** Template parameters */
    parameters: Record<string, string | number | boolean | string[]>;
    /** Output path (optional) */
    output?: string;
    /** Dry run flag */
    dryRun: boolean;
}

/**
 * Generation result
 */
export interface GenerateResult {
    /** Whether generation succeeded */
    success: boolean;
    /** Generated content */
    content?: string;
    /** Output path (if written) */
    outputPath?: string;
    /** Bytes generated */
    bytes?: number;
    /** Time taken (microseconds) */
    timeUs?: number;
    /** Estimated tokens saved */
    tokensSaved?: number;
    /** Error message if failed */
    error?: string;
}

/**
 * Token savings statistics
 */
export interface TokenSavings {
    /** Total tokens saved in this session */
    sessionTokens: number;
    /** Total tokens saved all time */
    totalTokens: number;
    /** Number of generations */
    generationCount: number;
}
