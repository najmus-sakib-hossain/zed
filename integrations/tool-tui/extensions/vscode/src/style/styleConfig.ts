/**
 * Style Configuration Module for DX VS Code Extension
 * 
 * Manages configuration options for dx-style features including auto-grouping
 * and inline expansion settings.
 * 
 * **Validates: Requirements 6.1, 6.3, 6.4**
 */

import * as vscode from 'vscode';

/**
 * Style configuration options
 */
export interface StyleConfig {
    /** Path to the generated CSS output file */
    outputPath: string;
    /** Auto-grouping configuration */
    autoGrouping: {
        /** Whether auto-grouping is enabled */
        enabled: boolean;
        /** Minimum character savings required to apply auto-grouping */
        minSavings: number;
    };
    /** Inline expansion configuration */
    inlineExpansion: {
        /** Whether inline expansion is enabled */
        enabled: boolean;
    };
}

/**
 * Default style configuration
 */
export const DEFAULT_STYLE_CONFIG: StyleConfig = {
    outputPath: 'dist/styles.css',
    autoGrouping: {
        enabled: false,
        minSavings: 10
    },
    inlineExpansion: {
        enabled: true
    }
};

/**
 * Load style configuration from VS Code settings
 * **Validates: Requirements 6.3**
 */
export function loadStyleConfig(): StyleConfig {
    const config = vscode.workspace.getConfiguration('dx.style');

    return {
        outputPath: config.get<string>('outputPath', DEFAULT_STYLE_CONFIG.outputPath),
        autoGrouping: {
            enabled: config.get<boolean>('autoGrouping.enabled', DEFAULT_STYLE_CONFIG.autoGrouping.enabled),
            minSavings: config.get<number>('autoGrouping.minSavings', DEFAULT_STYLE_CONFIG.autoGrouping.minSavings)
        },
        inlineExpansion: {
            enabled: config.get<boolean>('inlineExpansion.enabled', DEFAULT_STYLE_CONFIG.inlineExpansion.enabled)
        }
    };
}

/**
 * Check if auto-grouping is enabled
 * **Validates: Requirements 6.1, 6.4**
 */
export function isAutoGroupingEnabled(): boolean {
    const config = loadStyleConfig();
    return config.autoGrouping.enabled;
}

/**
 * Check if inline expansion is enabled
 */
export function isInlineExpansionEnabled(): boolean {
    const config = loadStyleConfig();
    return config.inlineExpansion.enabled;
}

/**
 * Get the minimum savings required for auto-grouping
 */
export function getAutoGroupingMinSavings(): number {
    const config = loadStyleConfig();
    return config.autoGrouping.minSavings;
}

/**
 * Get the output CSS file path
 */
export function getOutputPath(): string {
    const config = loadStyleConfig();
    return config.outputPath;
}

/**
 * Check if grouping should be applied based on character savings
 * **Validates: Requirements 6.2**
 * 
 * @param originalLength - Length of original atomic classnames
 * @param groupedLength - Length of grouped classname
 * @returns true if grouping should be applied
 */
export function shouldApplyGrouping(originalLength: number, groupedLength: number): boolean {
    if (!isAutoGroupingEnabled()) {
        return false;
    }

    const savings = originalLength - groupedLength;
    return savings >= getAutoGroupingMinSavings();
}

/**
 * Configuration change listener
 */
let configChangeDisposable: vscode.Disposable | undefined;

/**
 * Register configuration change listener
 */
export function registerConfigChangeListener(
    context: vscode.ExtensionContext,
    onConfigChange: (config: StyleConfig) => void
): void {
    configChangeDisposable = vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration('dx.style')) {
            const newConfig = loadStyleConfig();
            onConfigChange(newConfig);
            console.log('DX Style: Configuration updated');
        }
    });

    context.subscriptions.push(configChangeDisposable);
}

/**
 * Dispose configuration change listener
 */
export function disposeConfigChangeListener(): void {
    if (configChangeDisposable) {
        configChangeDisposable.dispose();
        configChangeDisposable = undefined;
    }
}
