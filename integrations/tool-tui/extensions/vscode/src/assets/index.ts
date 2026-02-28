/**
 * DX Unified Assets Module
 *
 * Exports all asset types, bridge, and utilities for the VS Code extension
 * to interact with dx-icon, dx-font, and dx-media crates through the CLI.
 *
 * @module assets
 */

// Export all types
export {
    IconAsset,
    FontAsset,
    MediaAsset,
    MediaType,
    AssetSamples,
    MediaTool,
    ToolCategory,
    AssetReference,
    parseAssetReference,
    formatAssetReference,
    SuccessResponse,
    ErrorResponse,
    CLIResponse,
    isErrorResponse,
    isSuccessResponse,
    ProviderHealth,
    HealthReport,
} from './assetTypes';

// Export bridge
export {
    AssetBridge,
    AssetBridgeError,
    ErrorCodes,
    AssetBridgeOptions,
    IconSearchOptions,
    FontSearchOptions,
    MediaSearchOptions,
} from './assetBridge';

// Export message protocol types for future webview communication
export * from './messageProtocol';
