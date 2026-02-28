/**
 * DX Driven Types
 * 
 * Type definitions for the VS Code driven integration.
 * Requirements: 9.1-9.10
 */

/**
 * Rule sync status for an editor
 */
export interface EditorSyncStatus {
    /** Editor name (cursor, copilot, windsurf, claude, aider, cline) */
    editor: string;
    /** Whether sync is enabled for this editor */
    enabled: boolean;
    /** Last sync timestamp */
    lastSync?: Date;
    /** Sync status */
    status: 'synced' | 'pending' | 'error' | 'disabled';
    /** Error message if status is error */
    error?: string;
}

/**
 * Specification metadata
 */
export interface SpecMetadata {
    /** Spec ID (e.g., "001") */
    id: string;
    /** Spec name */
    name: string;
    /** Spec directory path */
    path: string;
    /** Current workflow status */
    status: SpecStatus;
    /** Associated git branch */
    branch?: string;
    /** Creation date */
    created: Date;
    /** Last modified date */
    modified: Date;
}

/**
 * Spec workflow status
 */
export type SpecStatus =
    | 'draft'
    | 'specified'
    | 'planned'
    | 'tasks-ready'
    | 'in-progress'
    | 'completed';

/**
 * Hook definition
 */
export interface HookDefinition {
    /** Hook name */
    name: string;
    /** Hook description */
    description: string;
    /** Whether hook is enabled */
    enabled: boolean;
    /** Trigger type */
    triggerType: HookTriggerType;
    /** Trigger configuration */
    trigger: HookTrigger;
    /** Action to execute */
    action: HookAction;
    /** File path to hook config */
    configPath: string;
}

/**
 * Hook trigger types
 */
export type HookTriggerType = 'file-save' | 'manual' | 'session' | 'message';

/**
 * Hook trigger configuration
 */
export interface HookTrigger {
    /** Trigger type */
    type: HookTriggerType;
    /** File pattern for file-save triggers */
    pattern?: string;
    /** Manual trigger button label */
    label?: string;
}

/**
 * Hook action configuration
 */
export interface HookAction {
    /** Action type */
    type: 'shell' | 'message';
    /** Shell command for shell actions */
    command?: string;
    /** Message content for message actions */
    content?: string;
}

/**
 * Steering file metadata
 */
export interface SteeringFile {
    /** File name */
    name: string;
    /** File path */
    path: string;
    /** Inclusion mode */
    inclusionMode: SteeringInclusionMode;
    /** File match pattern (for fileMatch mode) */
    fileMatchPattern?: string;
    /** Manual context key (for manual mode) */
    contextKey?: string;
    /** File description from frontmatter */
    description?: string;
}

/**
 * Steering inclusion modes
 */
export type SteeringInclusionMode = 'always' | 'fileMatch' | 'manual';

/**
 * Template metadata for driven templates
 */
export interface DrivenTemplate {
    /** Template ID */
    id: string;
    /** Template name */
    name: string;
    /** Template description */
    description: string;
    /** Template category */
    category: DrivenTemplateCategory;
    /** Template tags */
    tags: string[];
}

/**
 * Driven template categories
 */
export type DrivenTemplateCategory =
    | 'persona'
    | 'project'
    | 'standard'
    | 'workflow';

/**
 * Driven configuration
 */
export interface DrivenConfig {
    /** Enabled editors */
    editors: Record<string, boolean>;
    /** Sync configuration */
    sync: {
        sourceOfTruth: string;
        watch: boolean;
        debounceMs: number;
    };
    /** Spec configuration */
    spec: {
        directory: string;
        autoBranch: boolean;
        constitution?: string;
    };
    /** Hooks configuration */
    hooks: {
        directory: string;
        enabled: boolean;
    };
}
