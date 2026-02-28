/**
 * dx-loader.js - WASM Loader and Bootstrap for dx-client
 * 
 * Handles fetching, compiling, and instantiating the dx-client WASM module.
 * Provides a simple API for initializing and rendering HTIP streams.
 */

import { DxClientHost } from './dx-client-host.js';

/**
 * DxLoader - WASM module loader and runtime manager
 */
export class DxLoader {
    constructor() {
        /** @type {DxClientHost} */
        this.host = new DxClientHost();

        /** @type {WebAssembly.Instance|null} */
        this.instance = null;

        /** @type {WebAssembly.Module|null} */
        this.module = null;

        /** @type {boolean} */
        this.initialized = false;
    }

    /**
     * Load and initialize the WASM module
     * @param {string|URL} wasmUrl - URL to the WASM file
     * @param {Element} rootElement - Root DOM element for rendering
     * @returns {Promise<void>}
     */
    async init(wasmUrl, rootElement) {
        if (this.initialized) {
            console.warn('DxLoader already initialized');
            return;
        }

        this.host.setRoot(rootElement);

        // Fetch and compile WASM
        const imports = this.host.getImports();

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            // Streaming compilation (preferred)
            const response = await fetch(wasmUrl);
            const result = await WebAssembly.instantiateStreaming(response, imports);
            this.module = result.module;
            this.instance = result.instance;
        } else {
            // Fallback for older browsers
            const response = await fetch(wasmUrl);
            const bytes = await response.arrayBuffer();
            this.module = await WebAssembly.compile(bytes);
            this.instance = await WebAssembly.instantiate(this.module, imports);
        }

        this.host.setInstance(this.instance);

        // Initialize WASM runtime
        if (this.instance.exports.init) {
            this.instance.exports.init();
        }

        this.initialized = true;
    }

    /**
     * Render an HTIP stream
     * @param {ArrayBuffer|Uint8Array} htipData - HTIP binary data
     * @returns {number} 0 on success, error code otherwise
     */
    render(htipData) {
        if (!this.initialized) {
            throw new Error('DxLoader not initialized. Call init() first.');
        }

        const data = htipData instanceof ArrayBuffer
            ? new Uint8Array(htipData)
            : htipData;

        // Allocate memory in WASM and copy data
        const memory = this.instance.exports.memory;
        const ptr = this._allocateAndCopy(data, memory);

        // Call render_stream
        const result = this.instance.exports.render_stream(ptr, data.length);

        return result;
    }

    /**
     * Fetch and render an HTIP stream from a URL
     * @param {string|URL} htipUrl - URL to the HTIP file
     * @returns {Promise<number>} 0 on success, error code otherwise
     */
    async renderFromUrl(htipUrl) {
        const response = await fetch(htipUrl);
        const data = await response.arrayBuffer();
        return this.render(data);
    }

    /**
     * Reset the runtime state
     */
    reset() {
        if (this.instance?.exports?.reset) {
            this.instance.exports.reset();
        }
        this.host.reset();
    }

    /**
     * Get runtime statistics
     * @returns {{nodes: number, templates: number, handlers: number}}
     */
    getStats() {
        const hostStats = this.host.getStats();

        if (this.instance?.exports) {
            return {
                ...hostStats,
                wasmNodes: this.instance.exports.get_node_count?.() ?? 0,
                wasmTemplates: this.instance.exports.get_template_count?.() ?? 0,
            };
        }

        return hostStats;
    }

    /**
     * Allocate memory in WASM and copy data
     * @private
     * @param {Uint8Array} data 
     * @param {WebAssembly.Memory} memory 
     * @returns {number} Pointer to allocated memory
     */
    _allocateAndCopy(data, memory) {
        // For simplicity, write to a fixed offset in linear memory
        // A production implementation would use a proper allocator
        const BUFFER_OFFSET = 1024; // Skip first 1KB for stack/globals

        const view = new Uint8Array(memory.buffer, BUFFER_OFFSET, data.length);
        view.set(data);

        return BUFFER_OFFSET;
    }
}

/**
 * Create and initialize a DxLoader instance
 * @param {string|URL} wasmUrl - URL to the WASM file
 * @param {string|Element} root - Root element or selector
 * @returns {Promise<DxLoader>}
 */
export async function createDxRuntime(wasmUrl, root) {
    const rootElement = typeof root === 'string'
        ? document.querySelector(root)
        : root;

    if (!rootElement) {
        throw new Error(`Root element not found: ${root}`);
    }

    const loader = new DxLoader();
    await loader.init(wasmUrl, rootElement);
    return loader;
}

/**
 * Auto-initialize if data attributes are present
 */
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', async () => {
        const script = document.querySelector('script[data-dx-wasm]');
        if (script) {
            const wasmUrl = script.getAttribute('data-dx-wasm');
            const rootSelector = script.getAttribute('data-dx-root') || '#app';

            try {
                const runtime = await createDxRuntime(wasmUrl, rootSelector);

                // Expose globally for debugging
                window.__dx_runtime = runtime;

                // Auto-render if HTIP URL provided
                const htipUrl = script.getAttribute('data-dx-htip');
                if (htipUrl) {
                    await runtime.renderFromUrl(htipUrl);
                }
            } catch (err) {
                console.error('[dx-loader] Initialization failed:', err);
            }
        }
    });
}

export default DxLoader;
