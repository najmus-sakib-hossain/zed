/**
 * Color Picker Module for DX VS Code Extension
 * 
 * Provides VS Code color picker integration for dx-style color-related classnames.
 * Pure color manipulation functions are in colorCore.ts.
 * 
 * **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5**
 */

import * as vscode from 'vscode';
import {
    ColorValue,
    isColorClass,
    parseColorFromClassname,
    updateClassnameColor,
    rgbToHsl,
    rgbToHex
} from './colorCore';

// Re-export core functions for convenience
export * from './colorCore';

/**
 * Convert VS Code Color to ColorValue
 */
export function vscodeColorToColorValue(color: vscode.Color): ColorValue {
    const r = Math.round(color.red * 255);
    const g = Math.round(color.green * 255);
    const b = Math.round(color.blue * 255);
    const hsl = rgbToHsl(r, g, b);

    return {
        hex: rgbToHex(r, g, b),
        rgb: { r, g, b },
        hsl,
        oklch: null,
        alpha: color.alpha
    };
}

/**
 * Convert ColorValue to VS Code Color
 */
export function colorValueToVscodeColor(color: ColorValue): vscode.Color {
    return new vscode.Color(
        color.rgb.r / 255,
        color.rgb.g / 255,
        color.rgb.b / 255,
        color.alpha
    );
}

/**
 * Color Provider for VS Code
 * 
 * Provides color decorations and color picker integration for dx-style classnames.
 * 
 * **Validates: Requirements 4.1, 4.2, 4.3**
 */
export class ColorProvider implements vscode.DocumentColorProvider {
    /**
     * Provide color information for the document
     * 
     * **Validates: Requirements 4.1, 4.5**
     */
    provideDocumentColors(
        document: vscode.TextDocument,
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.ColorInformation[]> {
        const colors: vscode.ColorInformation[] = [];
        const text = document.getText();

        // Find all class attributes
        const classAttrPattern = /(?:class|className)\s*=\s*["']([^"']+)["']/g;
        const templateClassPattern = /(?:class|className)\s*=\s*\{`([^`]+)`\}/g;

        const processClassValue = (match: RegExpExecArray, valueIndex: number) => {
            const classValue = match[valueIndex];
            const valueStart = match.index + match[0].indexOf(classValue);

            // Split into individual classnames
            const classnames = classValue.split(/\s+/);
            let offset = 0;

            for (const classname of classnames) {
                if (!classname) {
                    offset++;
                    continue;
                }

                const classnameStart = classValue.indexOf(classname, offset);

                if (isColorClass(classname)) {
                    const color = parseColorFromClassname(classname);
                    if (color) {
                        const startPos = document.positionAt(valueStart + classnameStart);
                        const endPos = document.positionAt(valueStart + classnameStart + classname.length);
                        const range = new vscode.Range(startPos, endPos);

                        colors.push(new vscode.ColorInformation(
                            range,
                            colorValueToVscodeColor(color)
                        ));
                    }
                }

                offset = classnameStart + classname.length;
            }
        };

        // Process standard class attributes
        let match;
        while ((match = classAttrPattern.exec(text)) !== null) {
            processClassValue(match, 1);
        }

        // Process template literal class attributes
        while ((match = templateClassPattern.exec(text)) !== null) {
            processClassValue(match, 1);
        }

        return colors;
    }

    /**
     * Provide color presentations for the color picker
     * 
     * **Validates: Requirements 4.2, 4.3**
     */
    provideColorPresentations(
        color: vscode.Color,
        context: { document: vscode.TextDocument; range: vscode.Range },
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.ColorPresentation[]> {
        const originalText = context.document.getText(context.range);
        const newColor = vscodeColorToColorValue(color);

        // Generate the new classname
        const newClassname = updateClassnameColor(originalText, newColor);

        const presentation = new vscode.ColorPresentation(newClassname);
        presentation.textEdit = new vscode.TextEdit(context.range, newClassname);

        return [presentation];
    }
}

/**
 * Register the color provider for supported file types
 * 
 * **Validates: Requirements 4.2**
 */
export function registerColorProvider(context: vscode.ExtensionContext): void {
    const colorProvider = new ColorProvider();

    // Supported languages for dx-style classnames
    const supportedLanguages = [
        { scheme: 'file', language: 'html' },
        { scheme: 'file', language: 'javascriptreact' },
        { scheme: 'file', language: 'typescriptreact' },
        { scheme: 'file', language: 'vue' },
        { scheme: 'file', language: 'svelte' },
        { scheme: 'file', language: 'astro' },
    ];

    for (const selector of supportedLanguages) {
        context.subscriptions.push(
            vscode.languages.registerColorProvider(selector, colorProvider)
        );
    }

    console.log('DX Style: Color provider registered for HTML, JSX, TSX, Vue, Svelte, Astro');
}
