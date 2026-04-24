import type { SchemaDef, Value } from "./types.js";
import { translateError } from "./error.js";
import { WasmSchema, valueFromWasm, valueToWasm } from "./wasm.js";

export class Schema {
  private constructor(private inner: WasmSchema) {}

  /**
   * Compiles a schema from a `SchemaDef` object or a JSON string.
   * Throws `BitspecError` on invalid input.
   */
  static compile(def: SchemaDef | string): Schema {
    const json = typeof def === "string" ? def : JSON.stringify(def);
    try {
      return new Schema(new WasmSchema(json));
    } catch (e) {
      throw translateError(e);
    }
  }

  /** Parses `bytes` and returns a map of field names to `Value`s. */
  parse(bytes: Uint8Array): Record<string, Value> {
    let raw: unknown;
    try {
      raw = this.inner.parse(bytes);
    } catch (e) {
      throw translateError(e);
    }
    const out: Record<string, Value> = {};
    // serde_wasm_bindgen serializes Rust maps as JS `Map` objects by default,
    // so we must iterate via the Map protocol rather than `Object.entries`.
    if (raw instanceof Map) {
      for (const [k, v] of raw as Map<string, unknown>) {
        out[k] = valueFromWasm(v);
      }
    } else {
      for (const [k, v] of Object.entries(raw as Record<string, unknown>)) {
        out[k] = valueFromWasm(v);
      }
    }
    return out;
  }

  /** Serializes a map of field names to `Value`s into raw bytes. */
  serialize(obj: Record<string, Value>): Uint8Array {
    const wasm: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj)) {
      wasm[k] = valueToWasm(v);
    }
    try {
      const result = this.inner.serialize(wasm);
      return new Uint8Array(result);
    } catch (e) {
      throw translateError(e);
    }
  }
}
