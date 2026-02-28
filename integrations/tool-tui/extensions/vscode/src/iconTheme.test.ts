/**
 * Tests for DX Icon Theme Path References
 * 
 * Feature: dx-icon-theme-consolidation
 * 
 * Tests Property 1 from the design document:
 * - Property 1: All Referenced Paths Resolve to Existing Files
 * 
 * **Validates: Requirements 2.3, 4.4, 6.4**
 */

import * as fc from 'fast-check';
import * as fs from 'fs';
import * as path from 'path';

// ============================================================================
// Types
// ============================================================================

interface FontSource {
    path: string;
    format: string;
}

interface FontDefinition {
    id: string;
    src: FontSource[];
}

interface IconDefinition {
    iconPath?: string;
    fontId?: string;
    fontCharacter?: string;
}

interface FileIconTheme {
    fonts: FontDefinition[];
    iconDefinitions: Record<string, IconDefinition>;
}

interface ProductIconDefinition {
    fontId: string;
    fontCharacter: string;
}

interface ProductIconTheme {
    fonts: FontDefinition[];
    iconDefinitions: Record<string, ProductIconDefinition>;
}

// ============================================================================
// Path Utilities
// ============================================================================

function getThemeBasePath(): string {
    // Resolve from the test file location to the theme directory
    return path.resolve(__dirname, '..', 'themes', 'icons');
}

function getProductIconThemeBasePath(): string {
    // Resolve from the test file location to the product icon theme directory
    return path.resolve(__dirname, '..', 'themes', 'product-icons');
}

function loadThemeJson(): FileIconTheme {
    const themePath = path.join(getThemeBasePath(), 'dx-file-icon-theme.json');
    const content = fs.readFileSync(themePath, 'utf-8');
    return JSON.parse(content);
}

function loadProductIconThemeJson(): ProductIconTheme {
    const themePath = path.join(getProductIconThemeBasePath(), 'dx-product-icon-theme.json');
    const content = fs.readFileSync(themePath, 'utf-8');
    return JSON.parse(content);
}

function extractFontPaths(theme: FileIconTheme): string[] {
    const paths: string[] = [];
    for (const font of theme.fonts) {
        for (const src of font.src) {
            paths.push(src.path);
        }
    }
    return paths;
}

function extractIconPaths(theme: FileIconTheme): string[] {
    const paths: string[] = [];
    for (const [, def] of Object.entries(theme.iconDefinitions)) {
        if (def.iconPath) {
            paths.push(def.iconPath);
        }
    }
    return paths;
}

function extractProductIconFontPaths(theme: ProductIconTheme): string[] {
    const paths: string[] = [];
    for (const font of theme.fonts) {
        for (const src of font.src) {
            paths.push(src.path);
        }
    }
    return paths;
}

function resolveThemePath(relativePath: string): string {
    const basePath = getThemeBasePath();
    // Remove leading ./ if present
    const cleanPath = relativePath.replace(/^\.\//, '');
    return path.join(basePath, cleanPath);
}

function resolveProductIconThemePath(relativePath: string): string {
    const basePath = getProductIconThemeBasePath();
    // Remove leading ./ if present
    const cleanPath = relativePath.replace(/^\.\//, '');
    return path.join(basePath, cleanPath);
}

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Icon Theme...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'Theme JSON file exists',
            test: () => {
                const themePath = path.join(getThemeBasePath(), 'dx-file-icon-theme.json');
                return fs.existsSync(themePath);
            }
        },
        {
            name: 'Theme JSON is valid JSON',
            test: () => {
                try {
                    loadThemeJson();
                    return true;
                } catch {
                    return false;
                }
            }
        },
        {
            name: 'Theme has fonts array',
            test: () => {
                const theme = loadThemeJson();
                return Array.isArray(theme.fonts) && theme.fonts.length > 0;
            }
        },
        {
            name: 'Theme has iconDefinitions object',
            test: () => {
                const theme = loadThemeJson();
                return typeof theme.iconDefinitions === 'object' && 
                       Object.keys(theme.iconDefinitions).length > 0;
            }
        },
        {
            name: 'All font paths are relative (start with ./)',
            test: () => {
                const theme = loadThemeJson();
                const fontPaths = extractFontPaths(theme);
                return fontPaths.every(p => p.startsWith('./'));
            }
        },
        {
            name: 'All icon paths are relative (start with ./)',
            test: () => {
                const theme = loadThemeJson();
                const iconPaths = extractIconPaths(theme);
                return iconPaths.every(p => p.startsWith('./'));
            }
        },
    ];

    for (const { name, test } of tests) {
        try {
            if (test()) {
                console.log(`  ✓ ${name}`);
                passed++;
            } else {
                console.log(`  ✗ ${name}`);
                failed++;
            }
        } catch (error) {
            console.log(`  ✗ ${name}: ${error}`);
            failed++;
        }
    }

    console.log(`\nUnit tests: ${passed} passed, ${failed} failed`);

    if (failed > 0) {
        throw new Error(`${failed} unit tests failed`);
    }
}

/**
 * Unit tests for Product Icon Theme
 */
export function runProductIconThemeUnitTests(): void {
    console.log('Running unit tests for Product Icon Theme...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        {
            name: 'Product Icon Theme JSON file exists',
            test: () => {
                const themePath = path.join(getProductIconThemeBasePath(), 'dx-product-icon-theme.json');
                return fs.existsSync(themePath);
            }
        },
        {
            name: 'Product Icon Theme JSON is valid JSON',
            test: () => {
                try {
                    loadProductIconThemeJson();
                    return true;
                } catch {
                    return false;
                }
            }
        },
        {
            name: 'Product Icon Theme has fonts array',
            test: () => {
                const theme = loadProductIconThemeJson();
                return Array.isArray(theme.fonts) && theme.fonts.length > 0;
            }
        },
        {
            name: 'Product Icon Theme has two font sources (macos-modern and intellij-icons)',
            test: () => {
                const theme = loadProductIconThemeJson();
                const fontIds = theme.fonts.map(f => f.id);
                return fontIds.includes('macos-modern') && fontIds.includes('intellij-icons');
            }
        },
        {
            name: 'Product Icon Theme has iconDefinitions object',
            test: () => {
                const theme = loadProductIconThemeJson();
                return typeof theme.iconDefinitions === 'object' && 
                       Object.keys(theme.iconDefinitions).length > 0;
            }
        },
        {
            name: 'All product icon font paths are relative (start with ./)',
            test: () => {
                const theme = loadProductIconThemeJson();
                const fontPaths = extractProductIconFontPaths(theme);
                return fontPaths.every(p => p.startsWith('./'));
            }
        },
        {
            name: 'Product Icon Theme has activity bar icons',
            test: () => {
                const theme = loadProductIconThemeJson();
                const activityBarIcons = ['explorer-view-icon', 'search-view-icon', 'run-view-icon', 
                                          'source-control-view-icon', 'extensions-view-icon'];
                return activityBarIcons.every(icon => icon in theme.iconDefinitions);
            }
        },
        {
            name: 'Product Icon Theme has symbol icons',
            test: () => {
                const theme = loadProductIconThemeJson();
                const symbolIcons = ['symbol-class', 'symbol-method', 'symbol-function', 
                                     'symbol-variable', 'symbol-interface'];
                return symbolIcons.every(icon => icon in theme.iconDefinitions);
            }
        },
    ];

    for (const { name, test } of tests) {
        try {
            if (test()) {
                console.log(`  ✓ ${name}`);
                passed++;
            } else {
                console.log(`  ✗ ${name}`);
                failed++;
            }
        } catch (error) {
            console.log(`  ✗ ${name}: ${error}`);
            failed++;
        }
    }

    console.log(`\nProduct Icon Theme unit tests: ${passed} passed, ${failed} failed`);

    if (failed > 0) {
        throw new Error(`${failed} product icon theme unit tests failed`);
    }
}

// ============================================================================
// Property Tests
// ============================================================================

/**
 * Property 1: All Referenced Paths Resolve to Existing Files
 * All font src paths and iconPath values in the theme JSON SHALL resolve to existing files.
 * 
 * **Validates: Requirements 2.3, 6.4**
 */
export function testAllPathsResolveToExistingFiles(): void {
    const theme = loadThemeJson();
    const fontPaths = extractFontPaths(theme);
    const iconPaths = extractIconPaths(theme);
    const allPaths = [...fontPaths, ...iconPaths];

    // Use fast-check to verify each path exists
    fc.assert(
        fc.property(
            fc.constantFrom(...allPaths),
            (relativePath: string) => {
                const absolutePath = resolveThemePath(relativePath);
                const exists = fs.existsSync(absolutePath);
                if (!exists) {
                    console.error(`Missing file: ${relativePath} (resolved to: ${absolutePath})`);
                }
                return exists;
            }
        ),
        { numRuns: allPaths.length }
    );
    console.log(`✓ Property 1: All Referenced Paths Resolve to Existing Files (${allPaths.length} paths verified)`);
}

/**
 * Additional property: Font files have correct extensions
 */
export function testFontFilesHaveCorrectExtensions(): void {
    const theme = loadThemeJson();
    
    fc.assert(
        fc.property(
            fc.constantFrom(...theme.fonts),
            (font: FontDefinition) => {
                for (const src of font.src) {
                    if (src.format === 'woff2' && !src.path.endsWith('.woff2')) {
                        return false;
                    }
                    if (src.format === 'woff' && !src.path.endsWith('.woff')) {
                        return false;
                    }
                }
                return true;
            }
        ),
        { numRuns: theme.fonts.length }
    );
    console.log('✓ Property: Font files have correct extensions');
}

/**
 * Additional property: Icon paths point to SVG files
 */
export function testIconPathsAreSvgFiles(): void {
    const theme = loadThemeJson();
    const iconPaths = extractIconPaths(theme);

    if (iconPaths.length === 0) {
        console.log('✓ Property: Icon paths are SVG files (no icon paths to verify)');
        return;
    }

    fc.assert(
        fc.property(
            fc.constantFrom(...iconPaths),
            (iconPath: string) => {
                return iconPath.endsWith('.svg');
            }
        ),
        { numRuns: iconPaths.length }
    );
    console.log(`✓ Property: Icon paths are SVG files (${iconPaths.length} paths verified)`);
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property tests for Icon Theme...\n');

    testAllPathsResolveToExistingFiles();
    testFontFilesHaveCorrectExtensions();
    testIconPathsAreSvgFiles();

    console.log('\n✓ All File Icon Theme property tests passed!');
}

/**
 * Property 1: All Referenced Paths Resolve to Existing Files (Product Icon Theme)
 * All font src paths in the product icon theme JSON SHALL resolve to existing files.
 * 
 * **Validates: Requirements 4.4, 6.4**
 */
export function testProductIconThemeAllPathsResolveToExistingFiles(): void {
    const theme = loadProductIconThemeJson();
    const fontPaths = extractProductIconFontPaths(theme);

    // Use fast-check to verify each path exists
    fc.assert(
        fc.property(
            fc.constantFrom(...fontPaths),
            (relativePath: string) => {
                const absolutePath = resolveProductIconThemePath(relativePath);
                const exists = fs.existsSync(absolutePath);
                if (!exists) {
                    console.error(`Missing file: ${relativePath} (resolved to: ${absolutePath})`);
                }
                return exists;
            }
        ),
        { numRuns: fontPaths.length }
    );
    console.log(`✓ Property 1 (Product Icon Theme): All Referenced Paths Resolve to Existing Files (${fontPaths.length} paths verified)`);
}

/**
 * Additional property: Product icon font files have correct extensions
 */
export function testProductIconFontFilesHaveCorrectExtensions(): void {
    const theme = loadProductIconThemeJson();
    
    fc.assert(
        fc.property(
            fc.constantFrom(...theme.fonts),
            (font: FontDefinition) => {
                for (const src of font.src) {
                    if (src.format === 'woff2' && !src.path.endsWith('.woff2')) {
                        return false;
                    }
                    if (src.format === 'woff' && !src.path.endsWith('.woff')) {
                        return false;
                    }
                }
                return true;
            }
        ),
        { numRuns: theme.fonts.length }
    );
    console.log('✓ Property: Product icon font files have correct extensions');
}

/**
 * Additional property: All icon definitions reference valid font IDs
 */
export function testProductIconDefinitionsReferenceValidFontIds(): void {
    const theme = loadProductIconThemeJson();
    const validFontIds = theme.fonts.map(f => f.id);
    const iconDefinitions = Object.entries(theme.iconDefinitions);

    fc.assert(
        fc.property(
            fc.constantFrom(...iconDefinitions),
            ([iconName, def]: [string, ProductIconDefinition]) => {
                const isValid = validFontIds.includes(def.fontId);
                if (!isValid) {
                    console.error(`Icon "${iconName}" references invalid fontId: ${def.fontId}`);
                }
                return isValid;
            }
        ),
        { numRuns: iconDefinitions.length }
    );
    console.log(`✓ Property: All icon definitions reference valid font IDs (${iconDefinitions.length} icons verified)`);
}

export function runAllProductIconThemePropertyTests(): void {
    console.log('Running Property tests for Product Icon Theme...\n');

    testProductIconThemeAllPathsResolveToExistingFiles();
    testProductIconFontFilesHaveCorrectExtensions();
    testProductIconDefinitionsReferenceValidFontIds();

    console.log('\n✓ All Product Icon Theme property tests passed!');
}

// ============================================================================
// Theme Registration Property Tests
// ============================================================================

interface PackageJson {
    contributes?: {
        iconThemes?: Array<{ id: string; label: string; path: string }>;
        productIconThemes?: Array<{ id: string; label: string; path: string }>;
    };
}

function getExtensionBasePath(): string {
    // Resolve from the test file location to the extension root
    return path.resolve(__dirname, '..');
}

function loadPackageJson(): PackageJson {
    const packagePath = path.join(getExtensionBasePath(), 'package.json');
    const content = fs.readFileSync(packagePath, 'utf-8');
    return JSON.parse(content);
}

/**
 * Property 2: Theme Registration Completeness
 * For any theme (file icon theme or product icon theme) included in the extension,
 * there SHALL exist a corresponding entry in package.json's contributes section
 * with a valid path to the theme JSON file.
 * 
 * **Validates: Requirements 2.4, 4.5**
 */
export function testThemeRegistrationCompleteness(): void {
    const packageJson = loadPackageJson();
    const extensionBasePath = getExtensionBasePath();
    
    const iconThemes = packageJson.contributes?.iconThemes || [];
    const productIconThemes = packageJson.contributes?.productIconThemes || [];
    
    const allThemes = [
        ...iconThemes.map(t => ({ ...t, type: 'iconTheme' })),
        ...productIconThemes.map(t => ({ ...t, type: 'productIconTheme' }))
    ];

    if (allThemes.length === 0) {
        throw new Error('No themes registered in package.json');
    }

    fc.assert(
        fc.property(
            fc.constantFrom(...allThemes),
            (theme: { id: string; label: string; path: string; type: string }) => {
                // Remove leading ./ if present and resolve path
                const cleanPath = theme.path.replace(/^\.\//, '');
                const absolutePath = path.join(extensionBasePath, cleanPath);
                const exists = fs.existsSync(absolutePath);
                
                if (!exists) {
                    console.error(`Missing theme file for ${theme.type} "${theme.id}": ${theme.path} (resolved to: ${absolutePath})`);
                }
                
                // Also verify it's valid JSON
                if (exists) {
                    try {
                        const content = fs.readFileSync(absolutePath, 'utf-8');
                        JSON.parse(content);
                    } catch (e) {
                        console.error(`Invalid JSON in theme file for ${theme.type} "${theme.id}": ${theme.path}`);
                        return false;
                    }
                }
                
                return exists;
            }
        ),
        { numRuns: allThemes.length }
    );
    console.log(`✓ Property 2: Theme Registration Completeness (${allThemes.length} themes verified)`);
}

/**
 * Additional property: All registered themes have required fields
 */
export function testThemeRegistrationHasRequiredFields(): void {
    const packageJson = loadPackageJson();
    
    const iconThemes = packageJson.contributes?.iconThemes || [];
    const productIconThemes = packageJson.contributes?.productIconThemes || [];
    
    const allThemes = [
        ...iconThemes.map(t => ({ ...t, type: 'iconTheme' })),
        ...productIconThemes.map(t => ({ ...t, type: 'productIconTheme' }))
    ];

    if (allThemes.length === 0) {
        throw new Error('No themes registered in package.json');
    }

    fc.assert(
        fc.property(
            fc.constantFrom(...allThemes),
            (theme: { id: string; label: string; path: string; type: string }) => {
                const hasId = typeof theme.id === 'string' && theme.id.length > 0;
                const hasLabel = typeof theme.label === 'string' && theme.label.length > 0;
                const hasPath = typeof theme.path === 'string' && theme.path.length > 0;
                
                if (!hasId) {
                    console.error(`Theme missing id: ${JSON.stringify(theme)}`);
                }
                if (!hasLabel) {
                    console.error(`Theme missing label: ${JSON.stringify(theme)}`);
                }
                if (!hasPath) {
                    console.error(`Theme missing path: ${JSON.stringify(theme)}`);
                }
                
                return hasId && hasLabel && hasPath;
            }
        ),
        { numRuns: allThemes.length }
    );
    console.log(`✓ Property: All registered themes have required fields (${allThemes.length} themes verified)`);
}

export function runThemeRegistrationPropertyTests(): void {
    console.log('Running Property tests for Theme Registration...\n');

    testThemeRegistrationCompleteness();
    testThemeRegistrationHasRequiredFields();

    console.log('\n✓ All Theme Registration property tests passed!');
}

// Run tests if this file is executed directly
if (require.main === module) {
    try {
        // File Icon Theme tests
        runUnitTests();
        console.log('');
        runAllPropertyTests();
        console.log('');
        
        // Product Icon Theme tests
        runProductIconThemeUnitTests();
        console.log('');
        runAllProductIconThemePropertyTests();
        console.log('');
        
        // Theme Registration tests
        runThemeRegistrationPropertyTests();
    } catch (error) {
        console.error('Tests failed:', error);
        process.exit(1);
    }
}
