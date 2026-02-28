/**
 * dx-client-host.js - JavaScript Host Functions for dx-client WASM Runtime
 * 
 * Provides the host function implementations that the WASM module imports.
 * Handles DOM manipulation, template caching, and event dispatch.
 */

/**
 * Event type IDs (must match WASM constants)
 */
const EVENT_TYPES = [
    'click', 'input', 'change', 'submit', 'focus', 'blur',
    'keydown', 'keyup', 'mouseenter', 'mouseleave', 'scroll'
];

/**
 * DxClientHost - Host function provider for dx-client WASM
 */
export class DxClientHost {
    constructor() {
        /** @type {Map<number, Element|Text>} Node ID to DOM element */
        this.nodes = new Map();

        /** @type {Map<number, DocumentFragment>} Template ID to cached template */
        this.templates = new Map();

        /** @type {Map<number, Function>} Handler ID to callback */
        this.handlers = new Map();

        /** @type {Map<number, Set<number>>} Node ID to dependent state slots */
        this.dependencies = new Map();

        /** @type {Set<number>} Dirty state slots */
        this.dirtySlots = new Set();

        /** @type {number} Next node ID */
        this.nextNodeId = 1;

        /** @type {Element|null} Root element */
        this.root = null;

        /** @type {WebAssembly.Instance|null} WASM instance */
        this.instance = null;

        /** @type {WebAssembly.Memory|null} WASM memory */
        this.memory = null;

        /** @type {TextDecoder} UTF-8 decoder */
        this.decoder = new TextDecoder('utf-8');

        /** @type {TextEncoder} UTF-8 encoder */
        this.encoder = new TextEncoder();
    }

    /**
     * Set the root element for rendering
     * @param {Element} root 
     */
    setRoot(root) {
        this.root = root;
        this.nodes.set(0, root);
    }

    /**
     * Set the WASM instance
     * @param {WebAssembly.Instance} instance 
     */
    setInstance(instance) {
        this.instance = instance;
        this.memory = instance.exports.memory;
    }

    /**
     * Read a string from WASM memory
     * @param {number} ptr 
     * @param {number} len 
     * @returns {string}
     */
    readString(ptr, len) {
        if (!this.memory || len === 0) return '';
        const bytes = new Uint8Array(this.memory.buffer, ptr, len);
        return this.decoder.decode(bytes);
    }

    /**
     * Get the import object for WASM instantiation
     * @returns {WebAssembly.Imports}
     */
    getImports() {
        return {
            env: {
                host_clone_template: this.host_clone_template.bind(this),
                host_cache_template: this.host_cache_template.bind(this),
                host_append: this.host_append.bind(this),
                host_remove: this.host_remove.bind(this),
                host_set_text: this.host_set_text.bind(this),
                host_set_attr: this.host_set_attr.bind(this),
                host_toggle_class: this.host_toggle_class.bind(this),
                host_listen: this.host_listen.bind(this),
                host_notify_state_change: this.host_notify_state_change.bind(this),
                host_log: this.host_log.bind(this),
            }
        };
    }

    // ========================================================================
    // Template Operations (Task 9.1)
    // ========================================================================

    /**
     * Clone a cached template and return a new node ID
     * @param {number} templateId 
     * @returns {number} New node ID
     */
    host_clone_template(templateId) {
        const template = this.templates.get(templateId);
        if (!template) {
            console.warn(`Template ${templateId} not found`);
            return 0;
        }

        const clone = template.cloneNode(true);
        const nodeId = this.nextNodeId++;

        // If template has a single child, use that; otherwise wrap in fragment
        if (clone.childNodes.length === 1) {
            this.nodes.set(nodeId, clone.firstChild);
        } else {
            // Create a wrapper div for multiple children
            const wrapper = document.createElement('div');
            wrapper.appendChild(clone);
            this.nodes.set(nodeId, wrapper);
        }

        return nodeId;
    }

    /**
     * Cache a template from HTML string
     * @param {number} templateId 
     * @param {number} htmlPtr 
     * @param {number} htmlLen 
     */
    host_cache_template(templateId, htmlPtr, htmlLen) {
        const html = this.readString(htmlPtr, htmlLen);
        const template = document.createElement('template');
        template.innerHTML = html;
        this.templates.set(templateId, template.content);
    }

    // ========================================================================
    // DOM Operations (Task 9.2)
    // ========================================================================

    /**
     * Append a child node to a parent
     * @param {number} parentId 
     * @param {number} childId 
     */
    host_append(parentId, childId) {
        const parent = this.nodes.get(parentId);
        const child = this.nodes.get(childId);

        if (!parent || !child) {
            console.warn(`host_append: parent=${parentId} child=${childId} not found`);
            return;
        }

        parent.appendChild(child);
    }

    /**
     * Remove a node from the DOM
     * @param {number} nodeId 
     */
    host_remove(nodeId) {
        const node = this.nodes.get(nodeId);
        if (!node) {
            console.warn(`host_remove: node=${nodeId} not found`);
            return;
        }

        node.remove();
        this.nodes.delete(nodeId);
        this.dependencies.delete(nodeId);
    }

    /**
     * Set text content of a node
     * @param {number} nodeId 
     * @param {number} textPtr 
     * @param {number} textLen 
     */
    host_set_text(nodeId, textPtr, textLen) {
        const node = this.nodes.get(nodeId);
        if (!node) {
            console.warn(`host_set_text: node=${nodeId} not found`);
            return;
        }

        const text = this.readString(textPtr, textLen);
        node.textContent = text;
    }

    /**
     * Set an attribute on a node
     * @param {number} nodeId 
     * @param {number} keyPtr 
     * @param {number} keyLen 
     * @param {number} valPtr 
     * @param {number} valLen 
     */
    host_set_attr(nodeId, keyPtr, keyLen, valPtr, valLen) {
        const node = this.nodes.get(nodeId);
        if (!node || !(node instanceof Element)) {
            console.warn(`host_set_attr: node=${nodeId} not found or not element`);
            return;
        }

        const key = this.readString(keyPtr, keyLen);
        const val = this.readString(valPtr, valLen);

        // Handle special attributes
        if (key === 'value' && 'value' in node) {
            node.value = val;
        } else if (key === 'checked' && 'checked' in node) {
            node.checked = val === 'true' || val === '1';
        } else if (key === 'disabled' && 'disabled' in node) {
            node.disabled = val === 'true' || val === '1';
        } else {
            node.setAttribute(key, val);
        }
    }

    /**
     * Toggle a class on a node
     * @param {number} nodeId 
     * @param {number} classPtr 
     * @param {number} classLen 
     * @param {number} enable 
     */
    host_toggle_class(nodeId, classPtr, classLen, enable) {
        const node = this.nodes.get(nodeId);
        if (!node || !(node instanceof Element)) {
            console.warn(`host_toggle_class: node=${nodeId} not found or not element`);
            return;
        }

        const className = this.readString(classPtr, classLen);
        node.classList.toggle(className, enable !== 0);
    }

    // ========================================================================
    // Event Handling (Task 9.3)
    // ========================================================================

    /**
     * Register an event listener on a node
     * @param {number} nodeId 
     * @param {number} eventType 
     * @param {number} handlerId 
     */
    host_listen(nodeId, eventType, handlerId) {
        const node = this.nodes.get(nodeId);
        if (!node) {
            console.warn(`host_listen: node=${nodeId} not found`);
            return;
        }

        const eventName = EVENT_TYPES[eventType] || 'click';

        const handler = (event) => {
            // Prevent default for form submissions
            if (eventName === 'submit') {
                event.preventDefault();
            }

            // Call WASM event handler
            if (this.instance?.exports?.on_event) {
                this.instance.exports.on_event(handlerId);
            }
        };

        node.addEventListener(eventName, handler);
        this.handlers.set(handlerId, handler);
    }

    // ========================================================================
    // State Management (Task 9.5)
    // ========================================================================

    /**
     * Mark a state slot as dirty
     * @param {number} slotId 
     */
    host_notify_state_change(slotId) {
        this.dirtySlots.add(slotId);

        // Schedule update on next frame
        if (this.dirtySlots.size === 1) {
            requestAnimationFrame(() => this.flushUpdates());
        }
    }

    /**
     * Track a dependency between a node and a state slot
     * @param {number} nodeId 
     * @param {number} slotId 
     */
    trackDependency(nodeId, slotId) {
        if (!this.dependencies.has(nodeId)) {
            this.dependencies.set(nodeId, new Set());
        }
        this.dependencies.get(nodeId).add(slotId);
    }

    /**
     * Flush pending updates for dirty state slots
     */
    flushUpdates() {
        const dirty = new Set(this.dirtySlots);
        this.dirtySlots.clear();

        // Find all nodes that depend on dirty slots
        const nodesToUpdate = new Set();
        for (const [nodeId, deps] of this.dependencies) {
            for (const slotId of dirty) {
                if (deps.has(slotId)) {
                    nodesToUpdate.add(nodeId);
                    break;
                }
            }
        }

        // Trigger re-render for affected nodes
        // (actual re-render logic would be driven by WASM)
        if (nodesToUpdate.size > 0 && this.instance?.exports?.update_dirty_nodes) {
            // Pass dirty node IDs to WASM for re-rendering
            for (const nodeId of nodesToUpdate) {
                this.instance.exports.update_dirty_nodes(nodeId);
            }
        }
    }

    // ========================================================================
    // Debug
    // ========================================================================

    /**
     * Log a value from WASM
     * @param {number} val 
     */
    host_log(val) {
        console.log('[dx-client]', val);
    }

    // ========================================================================
    // Utilities
    // ========================================================================

    /**
     * Reset the runtime state
     */
    reset() {
        this.nodes.clear();
        this.templates.clear();
        this.handlers.clear();
        this.dependencies.clear();
        this.dirtySlots.clear();
        this.nextNodeId = 1;

        if (this.root) {
            this.nodes.set(0, this.root);
        }
    }

    /**
     * Get statistics about the runtime
     * @returns {{nodes: number, templates: number, handlers: number}}
     */
    getStats() {
        return {
            nodes: this.nodes.size,
            templates: this.templates.size,
            handlers: this.handlers.size,
        };
    }
}
