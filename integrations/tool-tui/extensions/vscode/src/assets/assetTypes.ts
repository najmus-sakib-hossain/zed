/**
 * Asset Types for DX Unified Assets
 *
 * TypeScript interfaces for icon, font, and media assets used by the
 * VS Code extension to communicate with the dx CLI.
 *
 * @module assets/assetTypes
 */

/**
 * Icon asset from dx-icon crate.
 * Represents a single icon from one of the 225+ icon sets.
 */
export interface IconAsset {
    /** Unique identifier within the icon set */
    id: string;
    /** Icon set prefix (e.g., "mdi", "heroicons") */
    prefix: string;
    /** Human-readable icon name */
    name: string;
    /** Full name of the icon set */
    setName: string;
    /** Icon width in pixels */
    width: number;
    /** Icon height in pixels */
    height: number;
    /** SVG path data (the inner content of the SVG) */
    body: string;
}

/**
 * Font asset from dx-font crate.
 * Represents a font family from providers like Google Fonts.
 */
export interface FontAsset {
    /** Unique font identifier */
    id: string;
    /** Human-readable font name */
    name: string;
    /** Font provider (e.g., "google", "bunny") */
    provider: string;
    /** Font category (e.g., "sans-serif", "serif", "monospace") */
    category: string;
    /** Available font weights/styles */
    variants: string[];
    /** Available character subsets (e.g., "latin", "cyrillic") */
    subsets: string[];
    /** Optional preview URL */
    previewUrl?: string;
}

/**
 * Media asset from dx-media crate.
 * Represents an image, video, audio, or other media file.
 */
export interface MediaAsset {
    /** Unique asset identifier (format: "provider:id") */
    id: string;
    /** Human-readable asset name/title */
    name: string;
    /** Media provider (e.g., "openverse", "unsplash") */
    provider: string;
    /** Media type */
    type: MediaType;
    /** Thumbnail URL for preview */
    thumbnailUrl?: string;
    /** Direct download URL */
    downloadUrl: string;
    /** License information */
    license?: string;
    /** Image/video width in pixels */
    width?: number;
    /** Image/video height in pixels */
    height?: number;
    /** File size in bytes */
    fileSize?: number;
    /** MIME type */
    mimeType?: string;
}

/**
 * Media type enumeration.
 */
export type MediaType = 'image' | 'video' | 'audio' | '3d' | 'gif' | 'document';

/**
 * Collection of sample assets for UI previews.
 */
export interface AssetSamples {
    /** Sample icons from top icon sets */
    icons: IconAsset[];
    /** Sample fonts from each provider */
    fonts: FontAsset[];
    /** Sample media from each provider type */
    media: MediaAsset[];
    /** ISO timestamp when samples were generated */
    generatedAt: string;
}

/**
 * Media processing tool information.
 */
export interface MediaTool {
    /** Tool name/identifier */
    name: string;
    /** Human-readable description */
    description: string;
    /** Tool category */
    category: ToolCategory;
    /** External dependency required (e.g., "FFmpeg", "ImageMagick") */
    requiredDependency?: string;
    /** Supported input file types */
    inputTypes: string[];
    /** Possible output file types */
    outputTypes: string[];
}

/**
 * Tool category enumeration.
 */
export type ToolCategory = 'image' | 'video' | 'audio' | 'document' | 'archive';

/**
 * Asset reference for code generation.
 * Format: "<type>:<provider>:<id>"
 */
export interface AssetReference {
    /** Asset type (icon, font, media) */
    type: 'icon' | 'font' | 'media';
    /** Provider/source identifier */
    provider: string;
    /** Asset identifier */
    id: string;
}

/**
 * Parse an asset reference string into its components.
 * @param ref Reference string in format "type:provider:id"
 * @returns Parsed AssetReference or null if invalid
 */
export function parseAssetReference(ref: string): AssetReference | null {
    const parts = ref.split(':');
    if (parts.length < 3) {
        return null;
    }

    const type = parts[0] as AssetReference['type'];
    if (!['icon', 'font', 'media'].includes(type)) {
        return null;
    }

    return {
        type,
        provider: parts[1],
        id: parts.slice(2).join(':'), // Handle IDs that may contain colons
    };
}

/**
 * Format an asset reference into a string.
 * @param ref Asset reference object
 * @returns Reference string in format "type:provider:id"
 */
export function formatAssetReference(ref: AssetReference): string {
    return `${ref.type}:${ref.provider}:${ref.id}`;
}

/**
 * CLI response wrapper for successful operations.
 */
export interface SuccessResponse<T> {
    success: true;
    total?: number;
    results?: T[];
    result?: T;
    message?: string;
}

/**
 * CLI response wrapper for errors.
 */
export interface ErrorResponse {
    error: string;
    code: string;
    hint?: string;
    details?: unknown;
}

/**
 * Union type for CLI responses.
 */
export type CLIResponse<T> = SuccessResponse<T> | ErrorResponse;

/**
 * Type guard to check if response is an error.
 */
export function isErrorResponse(response: CLIResponse<unknown>): response is ErrorResponse {
    return 'error' in response;
}

/**
 * Type guard to check if response is successful.
 */
export function isSuccessResponse<T>(response: CLIResponse<T>): response is SuccessResponse<T> {
    return 'success' in response && response.success === true;
}

/**
 * Provider health status.
 */
export interface ProviderHealth {
    /** Provider name */
    name: string;
    /** Whether the provider is healthy/reachable */
    healthy: boolean;
    /** Response time in milliseconds */
    responseTimeMs: number;
    /** Error message if unhealthy */
    error?: string;
}

/**
 * Health check report for all providers.
 */
export interface HealthReport {
    /** Individual provider results */
    providers: ProviderHealth[];
    /** Total providers checked */
    totalProviders: number;
    /** Number of healthy providers */
    healthyCount: number;
    /** Total check time in milliseconds */
    totalTimeMs: number;
}
