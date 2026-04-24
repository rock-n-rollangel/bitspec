/* tslint:disable */
/* eslint-disable */

/**
 * Compiled schema that can be used from JavaScript to parse binary data.
 *
 * A `WasmSchema` owns a compiled [`bitspec::schema::Schema`] plus any
 * per‑field transforms that should be applied to the raw values.
 *
 * Typical usage from JavaScript/TypeScript is:
 *
 * ```text
 * // const schema = new WasmSchema(schemaJson);
 * // const parsed = schema.parse(bytes);
 * // console.log(parsed.someField);
 * ```
 */
export class WasmSchema {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Creates a new compiled schema from a JSON definition.
     *
     * The `schema_json` string must deserialize into [`SchemaDef`], which
     * in turn describes:
     *
     * - **Fields**: their name, kind (scalar or fixed‑size array),
     *   signedness and assemble strategy.
     * - **Fragments**: the bit ranges that make up each field.
     * - **Transforms** (optional): how to post‑process raw values using
     *   `bitspec::transform` (base type, scale/offset, encodings, enums).
     *
     * On success this compiles the schema and prepares any transforms so
     * that it can be reused to parse many payloads efficiently.
     */
    constructor(schema_json: string);
    /**
     * Parses a binary payload according to this compiled schema.
     *
     * - `data` is the raw byte slice (for example a `Uint8Array` passed from JS).
     * - The return value is a JavaScript object (`JsValue`) where keys are
     *   field names and values have been converted through any configured
     *   transforms (see [`bitspec::schema::Schema::apply_transforms`]).
     *
     * On error a JavaScript object of shape `{ code, message }` is thrown
     * across the boundary, where `code` is a stable string identifier and
     * `message` is a human-readable description.
     */
    parse(data: Uint8Array): any;
    /**
     * Serializes a JavaScript object into bytes according to this schema.
     *
     * `obj` is a JS object whose keys match field names and whose values are
     * compatible with [`bitspec::value::Value`].
     */
    serialize(obj: any): Uint8Array;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmschema_free: (a: number, b: number) => void;
    readonly wasmschema_new: (a: number, b: number) => [number, number, number];
    readonly wasmschema_parse: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmschema_serialize: (a: number, b: any) => [number, number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
