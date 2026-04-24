import wasmInit, { WasmSchema } from "../wasm/bitspec_wasm.js";
import type { Value } from "./types.js";

let initialized: Promise<void> | null = null;

/**
 * Resolves the input passed to the wasm-bindgen init function.
 *
 * In a browser (the default `wasm-pack --target web` target) the generated
 * init resolves the `.wasm` URL via `import.meta.url` and `fetch`s it — which
 * works. In Node.js, `fetch` on a `file:` URL is unimplemented, so we detect
 * Node and hand the init the raw bytes of `bitspec_wasm_bg.wasm` instead.
 */
async function resolveWasmInput(): Promise<unknown> {
  // Detect Node.js (including Vitest's Node environment).
  // `process.versions.node` is only present in Node-compatible runtimes.
  const isNode =
    typeof process !== "undefined" &&
    process.versions != null &&
    process.versions.node != null;
  if (!isNode) return undefined;
  const { readFile } = await import("node:fs/promises");
  const { fileURLToPath } = await import("node:url");
  const wasmUrl = new URL("../wasm/bitspec_wasm_bg.wasm", import.meta.url);
  const path = wasmUrl.protocol === "file:" ? fileURLToPath(wasmUrl) : wasmUrl;
  return await readFile(path as string);
}

/**
 * Initialize the WASM module. Must be awaited once before constructing any `Schema`.
 * Safe to call multiple times; subsequent calls return the same promise.
 */
export function init(): Promise<void> {
  if (initialized === null) {
    initialized = (async () => {
      const input = await resolveWasmInput();
      if (input === undefined) {
        await wasmInit();
      } else {
        // wasm-bindgen accepts a BufferSource for direct instantiation.
        await wasmInit({ module_or_path: input as BufferSource });
      }
    })();
  }
  return initialized;
}

export { WasmSchema };

/**
 * Converts the TypeScript discriminated `Value` into the Rust-side
 * externally-tagged enum shape expected by `bitspec::value::Value` via
 * `serde_wasm_bindgen`.
 */
export function valueToWasm(v: Value): unknown {
  switch (v.kind) {
    case "u64":
      return { U64: v.value };
    case "i64":
      return { I64: v.value };
    case "f32":
      return { F32: v.value };
    case "f64":
      return { F64: v.value };
    case "bytes":
      return { Bytes: Array.from(v.value) };
    case "string":
      return { String: v.value };
    case "array":
      return { Array: v.value.map(valueToWasm) };
  }
}

/** Inverse of `valueToWasm`: converts the Rust-side shape back into the TS union. */
export function valueFromWasm(raw: unknown): Value {
  if (raw === null || typeof raw !== "object") {
    throw new Error("invalid value: not an object");
  }
  const tagged = raw as Record<string, unknown>;
  const keys = Object.keys(tagged);
  if (keys.length !== 1) {
    throw new Error(`invalid value shape: expected one tag, got ${keys.join(",")}`);
  }
  const [tag] = keys;
  const inner = tagged[tag];
  switch (tag) {
    case "U64":
      return { kind: "u64", value: BigInt(inner as bigint | number | string) };
    case "I64":
      return { kind: "i64", value: BigInt(inner as bigint | number | string) };
    case "F32":
      return { kind: "f32", value: inner as number };
    case "F64":
      return { kind: "f64", value: inner as number };
    case "Bytes":
      return { kind: "bytes", value: new Uint8Array(inner as number[]) };
    case "String":
      return { kind: "string", value: inner as string };
    case "Array":
      return {
        kind: "array",
        value: (inner as unknown[]).map(valueFromWasm),
      };
    default:
      throw new Error(`unknown Value tag: ${tag}`);
  }
}
