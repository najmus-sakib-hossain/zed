import * as vscode from 'vscode';

// Color definitions matching our theme - vibrant and eye-catching
const COLORS = {
    heading1: '#00D9FF',      // Bright Cyan - stands out dramatically
    heading2: '#FF6B9D',      // Hot Pink - vibrant and energetic
    heading3: '#FFD93D',      // Golden Yellow - warm and bright
    heading4: '#A78BFA',      // Soft Purple - elegant
    heading5: '#34D399',      // Emerald Green - fresh
    heading6: '#FB923C',      // Bright Orange - energetic
    bold: '#FF5370',          // Bright Red - strong emphasis
    italic: '#82AAFF',        // Sky Blue - soft but visible
    code: '#C3E88D',          // Light Green - code-like
    link: '#89DDFF',          // Bright Cyan Blue - clickable feel
    linkText: '#FFCB6B',      // Amber - warm link text
    image: '#F78C6C',         // Coral Orange - media feel
    listBullet: '#C792EA',    // Bright Purple - stands out
    checkbox: '#00E676',      // Neon Green - checkbox feel
    quote: '#697098',         // Muted Purple - subtle but visible
    punctuation: '#4A5568',   // Dark Gray - subtle
    emoji: '#FFC107',         // Bright Yellow - fun
    table: '#64B5F6'          // Light Blue - structured
};

// Decoration types for each element
const decorationTypes = {
    heading1: vscode.window.createTextEditorDecorationType({
        color: COLORS.heading1,
        fontWeight: 'bold'
    }),
    heading2: vscode.window.createTextEditorDecorationType({
        color: COLORS.heading2,
        fontWeight: 'bold'
    }),
    heading3: vscode.window.createTextEditorDecorationType({
        color: COLORS.heading3,
        fontWeight: 'bold'
    }),
    heading4: vscode.window.createTextEditorDecorationType({
        color: COLORS.heading4,
        fontWeight: 'bold'
    }),
    heading5: vscode.window.createTextEditorDecorationType({
        color: COLORS.heading5,
        fontWeight: 'bold'
    }),
    heading6: vscode.window.createTextEditorDecorationType({
        color: COLORS.heading6,
        fontWeight: 'bold'
    }),
    bold: vscode.window.createTextEditorDecorationType({
        color: COLORS.bold,
        fontWeight: 'bold'
    }),
    italic: vscode.window.createTextEditorDecorationType({
        color: COLORS.italic,
        fontStyle: 'italic'
    }),
    code: vscode.window.createTextEditorDecorationType({
        color: COLORS.code
    }),
    link: vscode.window.createTextEditorDecorationType({
        color: COLORS.link,
        textDecoration: 'underline'
    }),
    linkText: vscode.window.createTextEditorDecorationType({
        color: COLORS.linkText
    }),
    image: vscode.window.createTextEditorDecorationType({
        color: COLORS.image
    }),
    listBullet: vscode.window.createTextEditorDecorationType({
        color: COLORS.listBullet,
        fontWeight: 'bold'
    }),
    checkbox: vscode.window.createTextEditorDecorationType({
        color: COLORS.checkbox,
        fontWeight: 'bold'
    }),
    quote: vscode.window.createTextEditorDecorationType({
        color: COLORS.quote,
        fontStyle: 'italic'
    }),
    punctuation: vscode.window.createTextEditorDecorationType({
        color: COLORS.punctuation
    }),
    emoji: vscode.window.createTextEditorDecorationType({
        color: COLORS.emoji
    }),
    table: vscode.window.createTextEditorDecorationType({
        color: COLORS.table
    })
};

export function activateMarkdownColorizer(context: vscode.ExtensionContext) {
    let timeout: NodeJS.Timeout | undefined = undefined;

    // Colorize active editor
    let activeEditor = vscode.window.activeTextEditor;
    console.log('dx-markdown-colorizer: Activating colorizer');
    if (activeEditor) {
        console.log('dx-markdown-colorizer: Active editor language:', activeEditor.document.languageId);
        console.log('dx-markdown-colorizer: Active editor URI:', activeEditor.document.uri.toString());
        if (activeEditor.document.languageId === 'dx-markdown') {
            console.log('dx-markdown-colorizer: Triggering initial decoration');
            triggerUpdateDecorations();
        }
    }

    vscode.window.onDidChangeActiveTextEditor(editor => {
        activeEditor = editor;
        if (editor) {
            console.log('dx-markdown-colorizer: Editor changed, language:', editor.document.languageId);
            if (editor.document.languageId === 'dx-markdown') {
                console.log('dx-markdown-colorizer: Triggering decoration on editor change');
                triggerUpdateDecorations();
            }
        }
    }, null, context.subscriptions);

    vscode.workspace.onDidChangeTextDocument(event => {
        if (activeEditor && event.document === activeEditor.document) {
            console.log('dx-markdown-colorizer: Document changed, language:', event.document.languageId);
            if (event.document.languageId === 'dx-markdown') {
                triggerUpdateDecorations();
            }
        }
    }, null, context.subscriptions);
    
    // Listen for language mode changes
    vscode.workspace.onDidOpenTextDocument(doc => {
        if (doc.languageId === 'dx-markdown') {
            console.log('dx-markdown-colorizer: Document opened with dx-markdown language');
            // Small delay to ensure editor is ready
            setTimeout(() => {
                if (activeEditor && activeEditor.document === doc) {
                    console.log('dx-markdown-colorizer: Triggering decoration after language change');
                    triggerUpdateDecorations();
                }
            }, 100);
        }
    }, null, context.subscriptions);
    
    // Set manual trigger function
    manualTrigger = triggerUpdateDecorations;

    function triggerUpdateDecorations() {
        if (timeout) {
            clearTimeout(timeout);
            timeout = undefined;
        }
        timeout = setTimeout(updateDecorations, 100);
    }

    function updateDecorations() {
        if (!activeEditor) {
            console.log('dx-markdown-colorizer: No active editor');
            return;
        }
        
        console.log('dx-markdown-colorizer: updateDecorations called for language:', activeEditor.document.languageId);
        
        if (activeEditor.document.languageId !== 'dx-markdown') {
            console.log('dx-markdown-colorizer: Not dx, skipping');
            return;
        }

        const text = activeEditor.document.getText();
        const lines = text.split('\n');
        
        console.log('dx-markdown-colorizer: Processing', lines.length, 'lines');

        // Arrays to hold decoration ranges
        const decorations: { [key: string]: vscode.DecorationOptions[] } = {
            heading1: [],
            heading2: [],
            heading3: [],
            heading4: [],
            heading5: [],
            heading6: [],
            bold: [],
            italic: [],
            code: [],
            link: [],
            linkText: [],
            image: [],
            listBullet: [],
            checkbox: [],
            quote: [],
            punctuation: [],
            emoji: [],
            table: []
        };

        lines.forEach((line, lineIndex) => {
            // Headings
            const headingMatch = line.match(/^(#{1,6})\s+(.+)$/);
            if (headingMatch) {
                const level = headingMatch[1].length;
                const headingKey = `heading${level}` as keyof typeof decorations;
                const start = new vscode.Position(lineIndex, 0);
                const end = new vscode.Position(lineIndex, line.length);
                decorations[headingKey].push({ range: new vscode.Range(start, end) });
                
                // Punctuation for #
                const punctStart = new vscode.Position(lineIndex, 0);
                const punctEnd = new vscode.Position(lineIndex, headingMatch[1].length);
                decorations.punctuation.push({ range: new vscode.Range(punctStart, punctEnd) });
                return;
            }

            // Blockquotes
            if (line.startsWith('>')) {
                const start = new vscode.Position(lineIndex, 0);
                const end = new vscode.Position(lineIndex, line.length);
                decorations.quote.push({ range: new vscode.Range(start, end) });
            }

            // List bullets
            const listMatch = line.match(/^(\s*)([-*+]|\d+\.)\s/);
            if (listMatch) {
                const start = new vscode.Position(lineIndex, listMatch[1].length);
                const end = new vscode.Position(lineIndex, listMatch[1].length + listMatch[2].length);
                decorations.listBullet.push({ range: new vscode.Range(start, end) });
            }

            // Checkboxes
            const checkboxMatch = line.match(/^(\s*[-*+]\s+)(\[[ xX]\])/);
            if (checkboxMatch) {
                const start = new vscode.Position(lineIndex, checkboxMatch[1].length);
                const end = new vscode.Position(lineIndex, checkboxMatch[1].length + checkboxMatch[2].length);
                decorations.checkbox.push({ range: new vscode.Range(start, end) });
            }

            // Inline code
            let codeRegex = /`([^`]+)`/g;
            let match;
            while ((match = codeRegex.exec(line)) !== null) {
                const start = new vscode.Position(lineIndex, match.index);
                const end = new vscode.Position(lineIndex, match.index + match[0].length);
                decorations.code.push({ range: new vscode.Range(start, end) });
            }

            // Bold
            let boldRegex = /(\*\*|__)(?=\S)(.+?)(?<=\S)\1/g;
            while ((match = boldRegex.exec(line)) !== null) {
                const start = new vscode.Position(lineIndex, match.index);
                const end = new vscode.Position(lineIndex, match.index + match[0].length);
                decorations.bold.push({ range: new vscode.Range(start, end) });
            }

            // Italic
            let italicRegex = /(\*|_)(?=\S)(.+?)(?<=\S)\1/g;
            while ((match = italicRegex.exec(line)) !== null) {
                // Skip if it's part of bold
                if (line[match.index - 1] === '*' || line[match.index - 1] === '_') {
                    continue;
                }
                const start = new vscode.Position(lineIndex, match.index);
                const end = new vscode.Position(lineIndex, match.index + match[0].length);
                decorations.italic.push({ range: new vscode.Range(start, end) });
            }

            // Links
            let linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
            while ((match = linkRegex.exec(line)) !== null) {
                // Link text
                const textStart = new vscode.Position(lineIndex, match.index + 1);
                const textEnd = new vscode.Position(lineIndex, match.index + 1 + match[1].length);
                decorations.linkText.push({ range: new vscode.Range(textStart, textEnd) });
                
                // Link URL
                const urlStart = new vscode.Position(lineIndex, match.index + match[1].length + 3);
                const urlEnd = new vscode.Position(lineIndex, match.index + match[0].length - 1);
                decorations.link.push({ range: new vscode.Range(urlStart, urlEnd) });
            }

            // Images
            let imageRegex = /!\[([^\]]*)\]\(([^)]+)\)/g;
            while ((match = imageRegex.exec(line)) !== null) {
                const start = new vscode.Position(lineIndex, match.index);
                const end = new vscode.Position(lineIndex, match.index + match[0].length);
                decorations.image.push({ range: new vscode.Range(start, end) });
            }

            // Emoji
            let emojiRegex = /:[a-zA-Z0-9_+-]+:/g;
            while ((match = emojiRegex.exec(line)) !== null) {
                const start = new vscode.Position(lineIndex, match.index);
                const end = new vscode.Position(lineIndex, match.index + match[0].length);
                decorations.emoji.push({ range: new vscode.Range(start, end) });
            }

            // Table separators
            if (line.match(/^\|([:\-\s|]+)\|$/)) {
                const start = new vscode.Position(lineIndex, 0);
                const end = new vscode.Position(lineIndex, line.length);
                decorations.table.push({ range: new vscode.Range(start, end) });
            }
        });

        // Apply all decorations
        Object.keys(decorations).forEach(key => {
            const decorationType = decorationTypes[key as keyof typeof decorationTypes];
            const ranges = decorations[key];
            activeEditor!.setDecorations(decorationType, ranges);
            if (ranges.length > 0) {
                console.log(`dx-markdown-colorizer: Applied ${ranges.length} ${key} decorations`);
            }
        });
        
        console.log('dx-markdown-colorizer: All decorations applied');
    }
}

export function deactivateMarkdownColorizer() {
    // Dispose all decoration types
    Object.values(decorationTypes).forEach(type => type.dispose());
}

// Export a function to manually trigger decoration update
let manualTrigger: (() => void) | null = null;

export function triggerMarkdownColorization() {
    if (manualTrigger) {
        console.log('dx-markdown-colorizer: Manual trigger called');
        manualTrigger();
    }
}
