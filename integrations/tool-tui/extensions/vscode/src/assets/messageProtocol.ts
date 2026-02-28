/**
 * Message Protocol for Asset Picker Webview
 *
 * Defines the message types for communication between the VS Code extension host
 * and the future dx-www based asset picker webview UI.
 *
 * @module assets/messageProtocol
 */

import { IconAsset, FontAsset, MediaAsset, MediaTool, AssetSamples } from './assetTypes';

/**
 * Base message interface for all webview messages.
 */
export interface BaseMessage {
    /** Unique message type identifier */
    type: string;
    /** Optional request ID for correlating responses */
    requestId?: string;
}

// ============================================================================
// Request Messages (Webview -> Extension Host)
// ============================================================================

/**
 * Request to search for icons.
 */
export interface SearchIconsRequest extends BaseMessage {
    type: 'searchIcons';
    query: string;
    limit?: number;
    prefix?: string;
}

/**
 * Request to search for fonts.
 */
export interface SearchFontsRequest extends BaseMessage {
    type: 'searchFonts';
    query: string;
    limit?: number;
    provider?: string;
    category?: string;
}

/**
 * Request to search for media.
 */
export interface SearchMediaRequest extends BaseMessage {
    type: 'searchMedia';
    query: string;
    limit?: number;
    mediaType?: string;
    provider?: string;
}

/**
 * Request to get sample assets for UI preview.
 */
export interface GetSamplesRequest extends BaseMessage {
    type: 'getSamples';
}

/**
 * Request to get available media tools.
 */
export interface GetMediaToolsRequest extends BaseMessage {
    type: 'getMediaTools';
    mediaType?: string;
}

/**
 * Request to generate icon component code.
 */
export interface GenerateIconComponentRequest extends BaseMessage {
    type: 'generateIconComponent';
    prefix: string;
    id: string;
    framework: string;
    typescript?: boolean;
}

/**
 * Request to download a media asset.
 */
export interface DownloadMediaRequest extends BaseMessage {
    type: 'downloadMedia';
    assetId: string;
    outputDir?: string;
}

/**
 * Request to insert an asset reference into the editor.
 */
export interface InsertAssetReferenceRequest extends BaseMessage {
    type: 'insertAssetReference';
    assetType: 'icon' | 'font' | 'media';
    provider: string;
    id: string;
    framework?: string;
}

/**
 * Request to copy text to clipboard.
 */
export interface CopyToClipboardRequest extends BaseMessage {
    type: 'copyToClipboard';
    text: string;
}

/**
 * Request to close the webview panel.
 */
export interface CloseWebviewRequest extends BaseMessage {
    type: 'closeWebview';
}

/**
 * Union type for all request messages from webview.
 */
export type WebviewRequest =
    | SearchIconsRequest
    | SearchFontsRequest
    | SearchMediaRequest
    | GetSamplesRequest
    | GetMediaToolsRequest
    | GenerateIconComponentRequest
    | DownloadMediaRequest
    | InsertAssetReferenceRequest
    | CopyToClipboardRequest
    | CloseWebviewRequest;

// ============================================================================
// Response Messages (Extension Host -> Webview)
// ============================================================================

/**
 * Base response interface.
 */
export interface BaseResponse extends BaseMessage {
    /** Whether the request succeeded */
    success: boolean;
    /** Error message if failed */
    error?: string;
}

/**
 * Response with icon search results.
 */
export interface SearchIconsResponse extends BaseResponse {
    type: 'searchIconsResponse';
    icons: IconAsset[];
    total: number;
}

/**
 * Response with font search results.
 */
export interface SearchFontsResponse extends BaseResponse {
    type: 'searchFontsResponse';
    fonts: FontAsset[];
    total: number;
}

/**
 * Response with media search results.
 */
export interface SearchMediaResponse extends BaseResponse {
    type: 'searchMediaResponse';
    media: MediaAsset[];
    total: number;
}

/**
 * Response with sample assets.
 */
export interface GetSamplesResponse extends BaseResponse {
    type: 'getSamplesResponse';
    samples: AssetSamples;
}

/**
 * Response with media tools.
 */
export interface GetMediaToolsResponse extends BaseResponse {
    type: 'getMediaToolsResponse';
    tools: MediaTool[];
}

/**
 * Response with generated component code.
 */
export interface GenerateIconComponentResponse extends BaseResponse {
    type: 'generateIconComponentResponse';
    code: string;
}

/**
 * Response for download completion.
 */
export interface DownloadMediaResponse extends BaseResponse {
    type: 'downloadMediaResponse';
    filePath: string;
}

/**
 * Generic acknowledgment response.
 */
export interface AckResponse extends BaseResponse {
    type: 'ack';
}

/**
 * Union type for all response messages to webview.
 */
export type WebviewResponse =
    | SearchIconsResponse
    | SearchFontsResponse
    | SearchMediaResponse
    | GetSamplesResponse
    | GetMediaToolsResponse
    | GenerateIconComponentResponse
    | DownloadMediaResponse
    | AckResponse;

// ============================================================================
// Push Messages (Extension Host -> Webview, unsolicited)
// ============================================================================

/**
 * Notification that the webview should update its state.
 */
export interface StateUpdateMessage extends BaseMessage {
    type: 'stateUpdate';
    state: WebviewState;
}

/**
 * Notification of a loading state change.
 */
export interface LoadingMessage extends BaseMessage {
    type: 'loading';
    isLoading: boolean;
    message?: string;
}

/**
 * Notification of an error.
 */
export interface ErrorMessage extends BaseMessage {
    type: 'error';
    error: string;
    code?: string;
    hint?: string;
}

/**
 * Union type for push messages.
 */
export type PushMessage = StateUpdateMessage | LoadingMessage | ErrorMessage;

// ============================================================================
// Webview State
// ============================================================================

/**
 * Current state of the asset picker webview.
 */
export interface WebviewState {
    /** Currently active tab */
    activeTab: 'icons' | 'fonts' | 'media';
    /** Current search query */
    searchQuery: string;
    /** Selected filters */
    filters: {
        iconPrefix?: string;
        fontProvider?: string;
        fontCategory?: string;
        mediaType?: string;
        mediaProvider?: string;
    };
    /** Currently selected asset (if any) */
    selectedAsset?: {
        type: 'icon' | 'font' | 'media';
        id: string;
    };
    /** Target framework for code generation */
    targetFramework: string;
    /** Whether to generate TypeScript */
    useTypeScript: boolean;
}

/**
 * Default initial state for the webview.
 */
export const DEFAULT_WEBVIEW_STATE: WebviewState = {
    activeTab: 'icons',
    searchQuery: '',
    filters: {},
    targetFramework: 'react',
    useTypeScript: true,
};

// ============================================================================
// Type Guards
// ============================================================================

/**
 * Type guard for request messages.
 */
export function isWebviewRequest(message: BaseMessage): message is WebviewRequest {
    return [
        'searchIcons',
        'searchFonts',
        'searchMedia',
        'getSamples',
        'getMediaTools',
        'generateIconComponent',
        'downloadMedia',
        'insertAssetReference',
        'copyToClipboard',
        'closeWebview',
    ].includes(message.type);
}

/**
 * Type guard for response messages.
 */
export function isWebviewResponse(message: BaseMessage): message is WebviewResponse {
    return [
        'searchIconsResponse',
        'searchFontsResponse',
        'searchMediaResponse',
        'getSamplesResponse',
        'getMediaToolsResponse',
        'generateIconComponentResponse',
        'downloadMediaResponse',
        'ack',
    ].includes(message.type);
}

/**
 * Type guard for push messages.
 */
export function isPushMessage(message: BaseMessage): message is PushMessage {
    return ['stateUpdate', 'loading', 'error'].includes(message.type);
}
