/**
 * Color Core Module - Pure Functions for Color Manipulation
 * 
 * This module contains pure functions for color parsing, formatting, and manipulation
 * that do not depend on VS Code APIs. This allows for testing outside of VS Code.
 * 
 * **Validates: Requirements 4.1, 4.3, 4.4, 4.5**
 */

/**
 * Color value representation supporting multiple formats
 */
export interface ColorValue {
    /** HEX color string (e.g., "#ff0000") */
    hex: string;
    /** RGB color components */
    rgb: { r: number; g: number; b: number };
    /** HSL color components */
    hsl: { h: number; s: number; l: number };
    /** OKLCH color components (if applicable) */
    oklch: { l: number; c: number; h: number } | null;
    /** Alpha/opacity value (0-1) */
    alpha: number;
}

/**
 * Color classname patterns that indicate color-related utilities
 */
export const COLOR_CLASSNAME_PATTERNS = [
    /^bg-(.+)$/,           // background color
    /^text-(.+)$/,         // text color
    /^border-(.+)$/,       // border color
    /^ring-(.+)$/,         // ring color
    /^outline-(.+)$/,      // outline color
    /^fill-(.+)$/,         // SVG fill color
    /^stroke-(.+)$/,       // SVG stroke color
    /^accent-(.+)$/,       // accent color
    /^caret-(.+)$/,        // caret color
    /^decoration-(.+)$/,   // text decoration color
    /^shadow-(.+)$/,       // shadow color (when color value)
    /^from-(.+)$/,         // gradient from color
    /^via-(.+)$/,          // gradient via color
    /^to-(.+)$/,           // gradient to color
];

/**
 * Named color values (subset of common Tailwind-like colors)
 */
export const NAMED_COLORS: Record<string, string> = {
    // Grayscale
    'black': '#000000',
    'white': '#ffffff',
    'transparent': 'transparent',
    'current': 'currentColor',

    // Gray scale
    'slate-50': '#f8fafc', 'slate-100': '#f1f5f9', 'slate-200': '#e2e8f0',
    'slate-300': '#cbd5e1', 'slate-400': '#94a3b8', 'slate-500': '#64748b',
    'slate-600': '#475569', 'slate-700': '#334155', 'slate-800': '#1e293b',
    'slate-900': '#0f172a', 'slate-950': '#020617',

    'gray-50': '#f9fafb', 'gray-100': '#f3f4f6', 'gray-200': '#e5e7eb',
    'gray-300': '#d1d5db', 'gray-400': '#9ca3af', 'gray-500': '#6b7280',
    'gray-600': '#4b5563', 'gray-700': '#374151', 'gray-800': '#1f2937',
    'gray-900': '#111827', 'gray-950': '#030712',

    // Red
    'red-50': '#fef2f2', 'red-100': '#fee2e2', 'red-200': '#fecaca',
    'red-300': '#fca5a5', 'red-400': '#f87171', 'red-500': '#ef4444',
    'red-600': '#dc2626', 'red-700': '#b91c1c', 'red-800': '#991b1b',
    'red-900': '#7f1d1d', 'red-950': '#450a0a',

    // Orange
    'orange-50': '#fff7ed', 'orange-100': '#ffedd5', 'orange-200': '#fed7aa',
    'orange-300': '#fdba74', 'orange-400': '#fb923c', 'orange-500': '#f97316',
    'orange-600': '#ea580c', 'orange-700': '#c2410c', 'orange-800': '#9a3412',
    'orange-900': '#7c2d12', 'orange-950': '#431407',

    // Yellow
    'yellow-50': '#fefce8', 'yellow-100': '#fef9c3', 'yellow-200': '#fef08a',
    'yellow-300': '#fde047', 'yellow-400': '#facc15', 'yellow-500': '#eab308',
    'yellow-600': '#ca8a04', 'yellow-700': '#a16207', 'yellow-800': '#854d0e',
    'yellow-900': '#713f12', 'yellow-950': '#422006',

    // Green
    'green-50': '#f0fdf4', 'green-100': '#dcfce7', 'green-200': '#bbf7d0',
    'green-300': '#86efac', 'green-400': '#4ade80', 'green-500': '#22c55e',
    'green-600': '#16a34a', 'green-700': '#15803d', 'green-800': '#166534',
    'green-900': '#14532d', 'green-950': '#052e16',

    // Blue
    'blue-50': '#eff6ff', 'blue-100': '#dbeafe', 'blue-200': '#bfdbfe',
    'blue-300': '#93c5fd', 'blue-400': '#60a5fa', 'blue-500': '#3b82f6',
    'blue-600': '#2563eb', 'blue-700': '#1d4ed8', 'blue-800': '#1e40af',
    'blue-900': '#1e3a8a', 'blue-950': '#172554',

    // Indigo
    'indigo-50': '#eef2ff', 'indigo-100': '#e0e7ff', 'indigo-200': '#c7d2fe',
    'indigo-300': '#a5b4fc', 'indigo-400': '#818cf8', 'indigo-500': '#6366f1',
    'indigo-600': '#4f46e5', 'indigo-700': '#4338ca', 'indigo-800': '#3730a3',
    'indigo-900': '#312e81', 'indigo-950': '#1e1b4b',

    // Purple
    'purple-50': '#faf5ff', 'purple-100': '#f3e8ff', 'purple-200': '#e9d5ff',
    'purple-300': '#d8b4fe', 'purple-400': '#c084fc', 'purple-500': '#a855f7',
    'purple-600': '#9333ea', 'purple-700': '#7e22ce', 'purple-800': '#6b21a8',
    'purple-900': '#581c87', 'purple-950': '#3b0764',

    // Pink
    'pink-50': '#fdf2f8', 'pink-100': '#fce7f3', 'pink-200': '#fbcfe8',
    'pink-300': '#f9a8d4', 'pink-400': '#f472b6', 'pink-500': '#ec4899',
    'pink-600': '#db2777', 'pink-700': '#be185d', 'pink-800': '#9d174d',
    'pink-900': '#831843', 'pink-950': '#500724',
};

/**
 * Parse a HEX color string to RGB components
 */
export function parseHex(hex: string): { r: number; g: number; b: number } | null {
    const cleanHex = hex.replace(/^#/, '');
    let r: number, g: number, b: number;

    if (cleanHex.length === 3) {
        r = parseInt(cleanHex[0] + cleanHex[0], 16);
        g = parseInt(cleanHex[1] + cleanHex[1], 16);
        b = parseInt(cleanHex[2] + cleanHex[2], 16);
    } else if (cleanHex.length === 6) {
        r = parseInt(cleanHex.slice(0, 2), 16);
        g = parseInt(cleanHex.slice(2, 4), 16);
        b = parseInt(cleanHex.slice(4, 6), 16);
    } else if (cleanHex.length === 8) {
        r = parseInt(cleanHex.slice(0, 2), 16);
        g = parseInt(cleanHex.slice(2, 4), 16);
        b = parseInt(cleanHex.slice(4, 6), 16);
    } else {
        return null;
    }

    if (isNaN(r) || isNaN(g) || isNaN(b)) {
        return null;
    }

    return { r, g, b };
}

/**
 * Parse an RGB/RGBA color string
 */
export function parseRgb(rgb: string): { r: number; g: number; b: number; a?: number } | null {
    const match = rgb.match(/rgba?\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+))?\s*\)/i);
    if (!match) return null;

    const r = parseInt(match[1], 10);
    const g = parseInt(match[2], 10);
    const b = parseInt(match[3], 10);
    const a = match[4] ? parseFloat(match[4]) : undefined;

    if (r < 0 || r > 255 || g < 0 || g > 255 || b < 0 || b > 255) return null;
    if (a !== undefined && (a < 0 || a > 1)) return null;

    return { r, g, b, a };
}

/**
 * Parse an HSL/HSLA color string
 */
export function parseHsl(hsl: string): { h: number; s: number; l: number; a?: number } | null {
    const match = hsl.match(/hsla?\s*\(\s*([\d.]+)\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%\s*(?:,\s*([\d.]+))?\s*\)/i);
    if (!match) return null;

    const h = parseFloat(match[1]);
    const s = parseFloat(match[2]);
    const l = parseFloat(match[3]);
    const a = match[4] ? parseFloat(match[4]) : undefined;

    if (h < 0 || h > 360 || s < 0 || s > 100 || l < 0 || l > 100) return null;
    if (a !== undefined && (a < 0 || a > 1)) return null;

    return { h, s, l, a };
}

/**
 * Parse an OKLCH color string
 */
export function parseOklch(oklch: string): { l: number; c: number; h: number; a?: number } | null {
    const match = oklch.match(/oklch\s*\(\s*([\d.]+)%?\s+([\d.]+)\s+([\d.]+)\s*(?:\/\s*([\d.]+))?\s*\)/i);
    if (!match) return null;

    return {
        l: parseFloat(match[1]),
        c: parseFloat(match[2]),
        h: parseFloat(match[3]),
        a: match[4] ? parseFloat(match[4]) : undefined
    };
}


/**
 * Convert RGB to HSL
 */
export function rgbToHsl(r: number, g: number, b: number): { h: number; s: number; l: number } {
    r /= 255; g /= 255; b /= 255;

    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const l = (max + min) / 2;

    if (max === min) {
        return { h: 0, s: 0, l: Math.round(l * 100) };
    }

    const d = max - min;
    const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);

    let h: number;
    switch (max) {
        case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
        case g: h = ((b - r) / d + 2) / 6; break;
        default: h = ((r - g) / d + 4) / 6; break;
    }

    return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) };
}

/**
 * Convert HSL to RGB
 */
export function hslToRgb(h: number, s: number, l: number): { r: number; g: number; b: number } {
    h /= 360; s /= 100; l /= 100;

    if (s === 0) {
        const gray = Math.round(l * 255);
        return { r: gray, g: gray, b: gray };
    }

    const hue2rgb = (p: number, q: number, t: number): number => {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1 / 6) return p + (q - p) * 6 * t;
        if (t < 1 / 2) return q;
        if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
        return p;
    };

    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;

    return {
        r: Math.round(hue2rgb(p, q, h + 1 / 3) * 255),
        g: Math.round(hue2rgb(p, q, h) * 255),
        b: Math.round(hue2rgb(p, q, h - 1 / 3) * 255)
    };
}

/**
 * Convert RGB to HEX string
 */
export function rgbToHex(r: number, g: number, b: number): string {
    const toHex = (n: number): string => {
        const hex = Math.max(0, Math.min(255, Math.round(n))).toString(16);
        return hex.length === 1 ? '0' + hex : hex;
    };
    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

/**
 * Approximate OKLCH to RGB conversion
 */
function oklchToRgbApprox(l: number, c: number, h: number): { r: number; g: number; b: number } {
    const hslL = l;
    const hslS = Math.min(100, c * 100);
    const hslH = h;
    return hslToRgb(hslH, hslS, hslL);
}

/**
 * Check if a classname is color-related
 */
export function isColorClass(classname: string): boolean {
    const baseClassname = classname.includes(':') ? classname.split(':').pop()! : classname;
    for (const pattern of COLOR_CLASSNAME_PATTERNS) {
        if (pattern.test(baseClassname)) return true;
    }
    return false;
}

/**
 * Extract the color value part from a classname
 */
function extractColorValue(classname: string): string | null {
    const baseClassname = classname.includes(':') ? classname.split(':').pop()! : classname;
    for (const pattern of COLOR_CLASSNAME_PATTERNS) {
        const match = baseClassname.match(pattern);
        if (match && match[1]) return match[1];
    }
    return null;
}

/**
 * Parse a color string in any supported format
 */
export function parseColor(colorString: string): ColorValue | null {
    const trimmed = colorString.trim();

    // Try HEX format
    if (trimmed.startsWith('#') || /^[0-9a-fA-F]{3,8}$/.test(trimmed)) {
        const hex = trimmed.startsWith('#') ? trimmed : `#${trimmed}`;
        const rgb = parseHex(hex);
        if (rgb) {
            const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);
            return { hex: rgbToHex(rgb.r, rgb.g, rgb.b), rgb, hsl, oklch: null, alpha: 1 };
        }
    }

    // Try RGB/RGBA format
    if (trimmed.toLowerCase().startsWith('rgb')) {
        const rgb = parseRgb(trimmed);
        if (rgb) {
            const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);
            return { hex: rgbToHex(rgb.r, rgb.g, rgb.b), rgb: { r: rgb.r, g: rgb.g, b: rgb.b }, hsl, oklch: null, alpha: rgb.a ?? 1 };
        }
    }

    // Try HSL/HSLA format
    if (trimmed.toLowerCase().startsWith('hsl')) {
        const hsl = parseHsl(trimmed);
        if (hsl) {
            const rgb = hslToRgb(hsl.h, hsl.s, hsl.l);
            return { hex: rgbToHex(rgb.r, rgb.g, rgb.b), rgb, hsl: { h: hsl.h, s: hsl.s, l: hsl.l }, oklch: null, alpha: hsl.a ?? 1 };
        }
    }

    // Try OKLCH format
    if (trimmed.toLowerCase().startsWith('oklch')) {
        const oklch = parseOklch(trimmed);
        if (oklch) {
            const rgb = oklchToRgbApprox(oklch.l, oklch.c, oklch.h);
            const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);
            return { hex: rgbToHex(rgb.r, rgb.g, rgb.b), rgb, hsl, oklch: { l: oklch.l, c: oklch.c, h: oklch.h }, alpha: oklch.a ?? 1 };
        }
    }

    return null;
}

/**
 * Parse a color from a classname
 */
export function parseColorFromClassname(classname: string): ColorValue | null {
    const colorValue = extractColorValue(classname);
    if (!colorValue) return null;

    // Check for named colors first
    if (NAMED_COLORS[colorValue]) {
        const hex = NAMED_COLORS[colorValue];
        if (hex === 'transparent' || hex === 'currentColor') return null;
        return parseColor(hex);
    }

    // Check for arbitrary value syntax
    if (colorValue.startsWith('[') && colorValue.endsWith(']')) {
        return parseColor(colorValue.slice(1, -1));
    }

    // Check for opacity modifier
    const opacityMatch = colorValue.match(/^(.+)\/(\d+)$/);
    if (opacityMatch) {
        const baseColor = opacityMatch[1];
        const opacity = parseInt(opacityMatch[2], 10) / 100;
        if (NAMED_COLORS[baseColor]) {
            const color = parseColor(NAMED_COLORS[baseColor]);
            if (color) { color.alpha = opacity; return color; }
        }
    }

    return null;
}


/**
 * Format a color value to a specific format
 */
export function formatColor(color: ColorValue, format: 'hex' | 'rgb' | 'hsl' | 'oklch'): string {
    switch (format) {
        case 'hex': return color.hex;
        case 'rgb':
            if (color.alpha < 1) return `rgba(${color.rgb.r}, ${color.rgb.g}, ${color.rgb.b}, ${color.alpha})`;
            return `rgb(${color.rgb.r}, ${color.rgb.g}, ${color.rgb.b})`;
        case 'hsl':
            if (color.alpha < 1) return `hsla(${color.hsl.h}, ${color.hsl.s}%, ${color.hsl.l}%, ${color.alpha})`;
            return `hsl(${color.hsl.h}, ${color.hsl.s}%, ${color.hsl.l}%)`;
        case 'oklch':
            if (color.oklch) {
                if (color.alpha < 1) return `oklch(${color.oklch.l}% ${color.oklch.c} ${color.oklch.h} / ${color.alpha})`;
                return `oklch(${color.oklch.l}% ${color.oklch.c} ${color.oklch.h})`;
            }
            return color.hex;
        default: return color.hex;
    }
}

/**
 * Find the closest named color for a given color value
 */
export function findClosestNamedColor(color: ColorValue): string | null {
    let closestName: string | null = null;
    let closestDistance = Infinity;

    for (const [name, hex] of Object.entries(NAMED_COLORS)) {
        if (hex === 'transparent' || hex === 'currentColor') continue;

        const namedRgb = parseHex(hex);
        if (!namedRgb) continue;

        const distance = Math.sqrt(
            Math.pow(color.rgb.r - namedRgb.r, 2) +
            Math.pow(color.rgb.g - namedRgb.g, 2) +
            Math.pow(color.rgb.b - namedRgb.b, 2)
        );

        if (distance < closestDistance) {
            closestDistance = distance;
            closestName = name;
        }
    }

    return closestDistance < 30 ? closestName : null;
}

/**
 * Generate a new classname with an updated color
 */
export function updateClassnameColor(classname: string, newColor: ColorValue): string {
    const baseClassname = classname.includes(':') ? classname.split(':').pop()! : classname;

    let prefix = '';
    for (const pattern of COLOR_CLASSNAME_PATTERNS) {
        const match = baseClassname.match(pattern);
        if (match) {
            prefix = baseClassname.slice(0, baseClassname.length - match[1].length);
            break;
        }
    }

    if (!prefix) return classname;

    const namedColor = findClosestNamedColor(newColor);
    if (namedColor) {
        const variantPrefix = classname.includes(':') ? classname.slice(0, classname.lastIndexOf(':') + 1) : '';
        if (newColor.alpha < 1) {
            const opacity = Math.round(newColor.alpha * 100);
            return `${variantPrefix}${prefix}${namedColor}/${opacity}`;
        }
        return `${variantPrefix}${prefix}${namedColor}`;
    }

    const variantPrefix = classname.includes(':') ? classname.slice(0, classname.lastIndexOf(':') + 1) : '';
    return `${variantPrefix}${prefix}[${newColor.hex}]`;
}

/**
 * Create a color swatch HTML for hover display
 */
export function createColorSwatchHtml(color: ColorValue): string {
    const bgColor = color.alpha < 1
        ? `rgba(${color.rgb.r}, ${color.rgb.g}, ${color.rgb.b}, ${color.alpha})`
        : color.hex;
    return `<span style="display:inline-block;width:14px;height:14px;background:${bgColor};border:1px solid #888;border-radius:2px;vertical-align:middle;margin-right:4px;"></span>`;
}
