/**
 * Style Hover Provider for DX VS Code Extension
 * 
 * Provides hover information for dx-style classnames including:
 * - MDN documentation and CSS property info
 * - Browser compatibility information
 * - Links to full MDN documentation
 * 
 * **Validates: Requirements 2.1, 2.2, 2.3, 2.5**
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { getMDNInfoForClassname } from './classnameMapping';
import { MDNPropertyInfo, BrowserSupport } from './mdnData';
import { getOutputMappingManager, LineInfo } from './outputMapping';

/**
 * Regex pattern to match class attribute values in HTML/JSX/TSX
 * Matches: class="...", className="...", class='...', className='...'
 */
const CLASS_ATTR_PATTERN = /(?:class|className)\s*=\s*["']([^"']+)["']/g;

/**
 * Regex pattern to match template literal class attributes
 * Matches: className={`...`}
 */
const TEMPLATE_CLASS_PATTERN = /(?:class|className)\s*=\s*\{`([^`]+)`\}/g;

/**
 * Check if a position is within a class attribute value
 */
function isInClassAttribute(line: string, position: number): { inClass: boolean; classValue: string; startOffset: number } {
    // Reset regex lastIndex
    CLASS_ATTR_PATTERN.lastIndex = 0;
    TEMPLATE_CLASS_PATTERN.lastIndex = 0;

    // Check standard class attributes
    let match;
    while ((match = CLASS_ATTR_PATTERN.exec(line)) !== null) {
        const valueStart = match.index + match[0].indexOf(match[1]);
        const valueEnd = valueStart + match[1].length;

        if (position >= valueStart && position <= valueEnd) {
            return { inClass: true, classValue: match[1], startOffset: valueStart };
        }
    }

    // Check template literal class attributes
    TEMPLATE_CLASS_PATTERN.lastIndex = 0;
    while ((match = TEMPLATE_CLASS_PATTERN.exec(line)) !== null) {
        const valueStart = match.index + match[0].indexOf(match[1]);
        const valueEnd = valueStart + match[1].length;

        if (position >= valueStart && position <= valueEnd) {
            return { inClass: true, classValue: match[1], startOffset: valueStart };
        }
    }

    return { inClass: false, classValue: '', startOffset: 0 };
}

/**
 * Find the classname at a specific position within a class attribute value
 */
function findClassnameAtPosition(classValue: string, offsetInValue: number): { classname: string; start: number; end: number } | null {
    const classnames = classValue.split(/\s+/);
    let currentOffset = 0;

    for (const classname of classnames) {
        // Skip empty strings from multiple spaces
        if (!classname) {
            currentOffset++;
            continue;
        }

        // Find actual position of this classname in the value
        const classnameStart = classValue.indexOf(classname, currentOffset);
        const classnameEnd = classnameStart + classname.length;

        if (offsetInValue >= classnameStart && offsetInValue <= classnameEnd) {
            return { classname, start: classnameStart, end: classnameEnd };
        }

        currentOffset = classnameEnd;
    }

    return null;
}

/**
 * Format browser support information for display
 */
function formatBrowserSupport(support: BrowserSupport): string {
    const browsers: string[] = [];

    if (support.chrome) {
        browsers.push(`Chrome ${support.chrome}+`);
    }
    if (support.firefox) {
        browsers.push(`Firefox ${support.firefox}+`);
    }
    if (support.safari) {
        browsers.push(`Safari ${support.safari}+`);
    }
    if (support.edge) {
        browsers.push(`Edge ${support.edge}+`);
    }

    return browsers.join(' · ');
}

/**
 * Create hover content for a dx-style classname
 * 
 * **Validates: Requirements 2.1, 2.2, 2.3, 2.5, 3.1, 3.3, 3.4**
 */
function createHoverContent(
    classname: string,
    mdnInfo: MDNPropertyInfo | null,
    lineInfo: LineInfo | null,
    outputPath: string | null
): vscode.MarkdownString {
    const md = new vscode.MarkdownString();
    md.isTrusted = true;
    md.supportHtml = true;

    // Header with classname
    md.appendMarkdown(`### \`${classname}\`\n\n`);

    // MDN Info section
    if (mdnInfo) {
        md.appendMarkdown(`**CSS Property:** \`${mdnInfo.propertyName}\`\n\n`);
        md.appendMarkdown(`${mdnInfo.description}\n\n`);
        md.appendMarkdown(`**Syntax:** \`${mdnInfo.syntax}\`\n\n`);

        // Browser support
        md.appendMarkdown(`---\n\n`);
        const browserSupportText = formatBrowserSupport(mdnInfo.browserSupport);

        if (mdnInfo.browserSupport.hasLimitedSupport) {
            md.appendMarkdown(`⚠️ **Limited Support:** ${browserSupportText}\n\n`);
        } else {
            md.appendMarkdown(`✅ **Browser Support:** ${browserSupportText}\n\n`);
        }

        // MDN link
        md.appendMarkdown(`[📖 MDN Documentation](${mdnInfo.mdnUrl})\n\n`);
    }

    // Generated CSS section
    if (lineInfo && outputPath) {
        md.appendMarkdown(`---\n\n`);
        md.appendMarkdown(`**Generated CSS** *(${path.basename(outputPath)}:${lineInfo.startLine})*\n\n`);
        md.appendMarkdown('```css\n');
        md.appendMarkdown(lineInfo.css);
        md.appendMarkdown('\n```\n\n');

        // Command link to open full viewer
        const encodedClassname = encodeURIComponent(classname);
        md.appendMarkdown(`[🔍 Open in CSS Viewer](command:dx.style.showCSSViewer?${encodedClassname})`);
    } else if (!mdnInfo) {
        // No MDN info and no generated CSS
        md.appendMarkdown(`*No CSS information available for this classname.*\n\n`);
        md.appendMarkdown(`*Run your build process to generate the output CSS file.*`);
    }

    return md;
}


/**
 * Style Hover Provider
 * 
 * Provides hover information for dx-style classnames in HTML, JSX, TSX, and Vue files.
 * Includes MDN documentation and generated CSS preview.
 * 
 * **Validates: Requirements 2.1, 2.3, 3.1, 3.3, 3.4**
 */
export class StyleHoverProvider implements vscode.HoverProvider {
    provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        _token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.Hover> {
        const line = document.lineAt(position.line).text;
        const charPosition = position.character;

        // Check if we're in a class attribute
        const classInfo = isInClassAttribute(line, charPosition);
        if (!classInfo.inClass) {
            return null;
        }

        // Find the specific classname at cursor position
        const offsetInValue = charPosition - classInfo.startOffset;
        const classnameInfo = findClassnameAtPosition(classInfo.classValue, offsetInValue);
        if (!classnameInfo) {
            return null;
        }

        // Get MDN info for the classname
        const mdnInfo = getMDNInfoForClassname(classnameInfo.classname);

        // Get generated CSS from output mapping
        const mappingManager = getOutputMappingManager();
        const lineInfo = mappingManager.getLineInfo(classnameInfo.classname);
        const outputPath = mappingManager.getOutputPath();

        // Only show hover if we have either MDN info or generated CSS
        if (!mdnInfo && !lineInfo) {
            return null;
        }

        // Create hover content with both MDN info and generated CSS
        const hoverContent = createHoverContent(
            classnameInfo.classname,
            mdnInfo || null,
            lineInfo,
            outputPath
        );

        // Calculate the range for the classname
        const rangeStart = classInfo.startOffset + classnameInfo.start;
        const rangeEnd = classInfo.startOffset + classnameInfo.end;
        const range = new vscode.Range(
            position.line, rangeStart,
            position.line, rangeEnd
        );

        return new vscode.Hover(hoverContent, range);
    }
}

/**
 * Register the style hover provider for supported file types
 * 
 * **Validates: Requirements 2.1**
 */
export function registerStyleHoverProvider(context: vscode.ExtensionContext): void {
    const hoverProvider = new StyleHoverProvider();

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
            vscode.languages.registerHoverProvider(selector, hoverProvider)
        );
    }

    console.log('DX Style: Hover provider registered for HTML, JSX, TSX, Vue, Svelte, Astro');
}
