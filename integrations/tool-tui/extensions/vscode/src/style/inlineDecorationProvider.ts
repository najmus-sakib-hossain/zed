/**
 * Inline Decoration Provider for DX VS Code Extension
 * 
 * Provides smart inline expansion of grouped classnames (dxg-*) on the current cursor line.
 * When the cursor is on a line containing grouped classnames, the atomic classnames are
 * displayed inline as decorations without modifying the actual document content.
 * 
 * **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5, 5.6**
 */

import * as vscode from 'vscode';
import {
    GroupRegistry,
    DecorationStyle,
    DEFAULT_DECORATION_STYLE,
    GROUPED_CLASSNAME_PATTERN,
    isGroupedClass,
    expandGroupedClass,
    createGroupRegistry,
    loadGroupsIntoRegistry,
    addGroupToRegistry,
    parseGroupDefinitions,
    findGroupedClassnamesInLine,
    formatExpandedClassnames
} from './inlineDecorationCore';
import { isInlineExpansionEnabled } from './styleConfig';

// Re-export core functions for convenience
export {
    GroupRegistry,
    DecorationStyle,
    DEFAULT_DECORATION_STYLE,
    GROUPED_CLASSNAME_PATTERN,
    isGroupedClass,
    expandGroupedClass,
    createGroupRegistry,
    loadGroupsIntoRegistry,
    addGroupToRegistry,
    parseGroupDefinitions,
    findGroupedClassnamesInLine,
    formatExpandedClassnames
} from './inlineDecorationCore';

/**
 * Inline Decoration Provider
 * 
 * Manages inline decorations that show expanded atomic classnames for grouped classnames
 * on the current cursor line only.
 * 
 * **Validates: Requirements 5.1, 5.2, 5.3, 5.5, 5.6**
 */
export class InlineDecorationProvider implements vscode.Disposable {
    private decorationType: vscode.TextEditorDecorationType;
    private groupRegistry: GroupRegistry;
    private disposables: vscode.Disposable[] = [];
    private currentDecorations: Map<string, vscode.DecorationOptions[]> = new Map();
    private decorationStyle: DecorationStyle;

    constructor(style: DecorationStyle = DEFAULT_DECORATION_STYLE) {
        this.decorationStyle = style;
        this.groupRegistry = createGroupRegistry();

        // Create decoration type with after pseudo-element for inline expansion
        // **Validates: Requirements 5.5**
        this.decorationType = vscode.window.createTextEditorDecorationType({
            after: {
                color: this.decorationStyle.color,
                fontStyle: this.decorationStyle.fontStyle,
            }
        });
    }

    /**
     * Initialize the provider and set up event listeners
     * **Validates: Requirements 5.1, 5.2, 5.3**
     */
    public initialize(context: vscode.ExtensionContext): void {
        // Listen for cursor position changes
        this.disposables.push(
            vscode.window.onDidChangeTextEditorSelection((event) => {
                this.updateDecorations(event.textEditor);
            })
        );

        // Listen for active editor changes
        this.disposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                if (editor) {
                    this.updateDecorations(editor);
                }
            })
        );

        // Initial update for current editor
        const activeEditor = vscode.window.activeTextEditor;
        if (activeEditor) {
            this.updateDecorations(activeEditor);
        }

        // Add to context subscriptions
        context.subscriptions.push(this);
    }

    /**
     * Load group registry from dx-style output
     * **Validates: Requirements 5.1**
     */
    public loadGroupRegistry(groups: Map<string, string[]>): void {
        loadGroupsIntoRegistry(this.groupRegistry, groups);
    }

    /**
     * Add a group to the registry
     */
    public addGroup(groupedClass: string, atomicClasses: string[]): void {
        addGroupToRegistry(this.groupRegistry, groupedClass, atomicClasses);
    }

    /**
     * Get expanded classnames for a grouped classname
     * **Validates: Requirements 5.1, 5.4**
     */
    public expandGroupedClass(groupedClass: string): string[] {
        return expandGroupedClass(groupedClass, this.groupRegistry.groups);
    }

    /**
     * Check if a classname is a grouped classname
     * **Validates: Requirements 5.4**
     */
    public isGroupedClass(classname: string): boolean {
        return isGroupedClass(classname);
    }

    /**
     * Update decorations based on cursor position
     * **Validates: Requirements 5.1, 5.2, 5.3, 5.6**
     */
    public updateDecorations(editor: vscode.TextEditor): void {
        // Check if inline expansion is enabled
        if (!isInlineExpansionEnabled()) {
            this.clearDecorations(editor);
            return;
        }

        // Only process supported file types
        if (!this.isSupportedLanguage(editor.document.languageId)) {
            this.clearDecorations(editor);
            return;
        }

        // Get current cursor line
        const cursorLine = editor.selection.active.line;
        const lineText = editor.document.lineAt(cursorLine).text;

        // Find all grouped classnames on the current line
        const groupedClassnames = findGroupedClassnamesInLine(lineText);
        const decorations: vscode.DecorationOptions[] = [];

        for (const { classname, start, end } of groupedClassnames) {
            const atomicClasses = this.expandGroupedClass(classname);

            if (atomicClasses.length > 0) {
                const startPos = new vscode.Position(cursorLine, start);
                const endPos = new vscode.Position(cursorLine, end);
                const range = new vscode.Range(startPos, endPos);

                // Create decoration with expanded classnames shown after
                // **Validates: Requirements 5.5, 5.6**
                decorations.push({
                    range,
                    renderOptions: {
                        after: {
                            contentText: formatExpandedClassnames(atomicClasses),
                            color: this.decorationStyle.color,
                            fontStyle: this.decorationStyle.fontStyle,
                        }
                    }
                });
            }
        }

        // Apply decorations (this replaces previous decorations)
        // **Validates: Requirements 5.6** - Document content is NOT modified
        editor.setDecorations(this.decorationType, decorations);
        this.currentDecorations.set(editor.document.uri.toString(), decorations);
    }

    /**
     * Clear all decorations from an editor
     */
    public clearDecorations(editor?: vscode.TextEditor): void {
        if (editor) {
            editor.setDecorations(this.decorationType, []);
            this.currentDecorations.delete(editor.document.uri.toString());
        } else {
            // Clear from all editors
            for (const visibleEditor of vscode.window.visibleTextEditors) {
                visibleEditor.setDecorations(this.decorationType, []);
            }
            this.currentDecorations.clear();
        }
    }

    /**
     * Check if a language is supported for inline expansion
     */
    private isSupportedLanguage(languageId: string): boolean {
        const supportedLanguages = [
            'html',
            'javascriptreact',
            'typescriptreact',
            'vue',
            'svelte',
            'astro'
        ];
        return supportedLanguages.includes(languageId);
    }

    /**
     * Get the current group registry
     */
    public getGroupRegistry(): GroupRegistry {
        return this.groupRegistry;
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.clearDecorations();
        this.decorationType.dispose();
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}

/**
 * Global instance of the inline decoration provider
 */
let inlineDecorationProvider: InlineDecorationProvider | undefined;

/**
 * Get the inline decoration provider instance
 */
export function getInlineDecorationProvider(): InlineDecorationProvider | undefined {
    return inlineDecorationProvider;
}

/**
 * Register the inline decoration provider
 * **Validates: Requirements 5.1**
 */
export function registerInlineDecorationProvider(context: vscode.ExtensionContext): InlineDecorationProvider {
    inlineDecorationProvider = new InlineDecorationProvider();
    inlineDecorationProvider.initialize(context);

    console.log('DX Style: Inline decoration provider registered for HTML, JSX, TSX, Vue, Svelte, Astro');

    return inlineDecorationProvider;
}

/**
 * Load group registry from a JSON file or Binary Dawn format
 * This is called when dx-style output is available
 */
export async function loadGroupRegistryFromFile(filePath: string): Promise<void> {
    if (!inlineDecorationProvider) {
        console.warn('DX Style: Inline decoration provider not initialized');
        return;
    }

    try {
        const fs = await import('fs');
        const content = await fs.promises.readFile(filePath, 'utf-8');

        const groups = parseGroupDefinitions(content);
        inlineDecorationProvider.loadGroupRegistry(groups);
        console.log(`DX Style: Loaded ${groups.size} groups from ${filePath}`);
    } catch (error) {
        console.error(`DX Style: Failed to load group registry from ${filePath}:`, error);
    }
}
