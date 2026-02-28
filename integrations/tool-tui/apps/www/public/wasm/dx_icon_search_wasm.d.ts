/* tslint:disable */
/* eslint-disable */

export class IconSearch {
    free(): void;
    [Symbol.dispose](): void;
    addIcon(name: string, pack: string): void;
    cacheSize(): number;
    clearCache(): void;
    loadIcons(json_data: string): void;
    constructor();
    search(query: string, limit: number): any;
    totalIcons(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_iconsearch_free: (a: number, b: number) => void;
    readonly iconsearch_addIcon: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly iconsearch_cacheSize: (a: number) => number;
    readonly iconsearch_clearCache: (a: number) => void;
    readonly iconsearch_loadIcons: (a: number, b: number, c: number) => [number, number];
    readonly iconsearch_new: () => number;
    readonly iconsearch_search: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly iconsearch_totalIcons: (a: number) => number;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
