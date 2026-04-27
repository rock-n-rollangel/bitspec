# bitspec

TypeScript wrapper around the [bitspec](https://github.com/somebytes/bitspec) schema engine, compiled to WebAssembly. Declare binary layouts as JSON, parse `Uint8Array` payloads into typed `Value` objects, and serialize them back.

`bitspec` targets protocols whose layout is decided at runtime — telemetry frames described by a config file, sensor payloads pulled from a device registry, or binary formats a user can edit. The same schema shape works in Rust (via the [`bitspec`](https://crates.io/crates/bitspec) crate) and in TypeScript, so a single definition can drive both a producer and a consumer.

## Installation

```bash
npm install bitspec
```

> Note: `bitspec` is not yet published to npm. For now, clone the repo and run `npm run build` inside `packages/bitspec` to produce `dist/` and `wasm/`, then depend on the package via a local path or `npm link`.

## Quick start

```ts
import { init, Schema } from "bitspec";

await init();

const schema = Schema.compile({
  fields: [
    { name: "id", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 0, len_bits: 4 }] },
    { name: "payload", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 4, len_bits: 4 }] },
  ],
});

const parsed = schema.parse(new Uint8Array([0xA5]));
// { id: { kind: 'u64', value: 10n }, payload: { kind: 'u64', value: 5n } }

const bytes = schema.serialize(parsed);
// Uint8Array(1) [ 165 ]
```

`Schema.compile` accepts either a `SchemaDef` object or a JSON string. Both forms validate on compile — an invalid schema throws `BitspecError` before you get a `Schema` back.

## Async initialization

The engine is compiled to WebAssembly and must be instantiated before any `Schema` call. `init()` returns a promise and memoizes the result, so it is safe (and cheap) to `await` repeatedly. A common pattern is to put it behind a one-off top-level `await` in your entrypoint, or inside a `beforeAll` in your test suite.

```ts
import { init } from "bitspec";
await init(); // call once at startup; subsequent calls return the same promise
```

The package ships a `target=web` wasm artifact. In browsers it is loaded via `fetch`; in Node.js (including Vitest's default environment) the loader detects the runtime and hands the init function the raw `.wasm` bytes read from disk, so no extra setup is required.

## The `Value` type

Parsed values and serialize inputs share the same discriminated union. Each variant is tagged by a `kind` string and carries a `value` of the appropriate type.

| `kind` | `value` type | Emitted by | Accepted by |
|---|---|---|---|
| `"u64"` | `bigint` | parse (unsigned fields) | serialize |
| `"i64"` | `bigint` | parse (signed fields) | serialize |
| `"f32"` | `number` | transforms (`Float32`, scale/offset on F32) | not by serialize |
| `"f64"` | `number` | transforms (`Float64`, scale/offset on ints) | not by serialize |
| `"bytes"` | `Uint8Array` | transforms (`Base: "Bytes"`) | not by serialize |
| `"string"` | `string` | transforms (enum map or `encoding`) | not by serialize |
| `"array"` | `Value[]` | parse (array fields), transforms | serialize (for array fields) |

Serialize accepts only `u64`, `i64`, and `array`. Transform outputs like `f64`/`string` are intended for the parse-and-display path; to write a float or string back, convert it to a `u64` bit pattern yourself (see `floatBits32` / `floatBits64` below).

## The `bigint` caveat

64-bit integers come back as `BigInt`, not `number`. JavaScript `number` loses precision above 2^53, and `bitspec` supports fields up to 64 bits wide, so the wrapper uses `bigint` uniformly for `u64` and `i64` — even when the value would fit in a `number`. Remember the `n` suffix when constructing literals:

```ts
schema.serialize({
  id: { kind: "u64", value: 42n },        // note the `n`
  big: { kind: "u64", value: (1n << 60n) + 12345n },
});
```

Arithmetic on bigints cannot mix with `number`; convert explicitly with `Number()` or `BigInt()` when you need to.

## Writing float fields

Schemas ultimately serialize 64-bit-or-smaller integer bit patterns. If your on-wire format is a 32-bit IEEE float, use `floatBits32` to pack the float into a `u64` before handing it to `serialize`:

```ts
import { floatBits32, init, Schema } from "bitspec";

await init();

const schema = Schema.compile({
  fields: [
    { name: "voltage", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 0, len_bits: 32 }],
      transform: { base: "Float32" } },
  ],
});

const bytes = schema.serialize({ voltage: floatBits32(3.14) });
const parsed = schema.parse(bytes);
console.log(parsed.voltage);
// { kind: 'f32', value: 3.140000104904175 }
```

`floatBits64` works the same way for 64-bit fields. Both helpers simply reinterpret the IEEE-754 bit pattern as a `u64`; the `Float32` / `Float64` transform on the schema side turns it back into a `number` during parse.

## Transforms from TypeScript

Attach a `transform` object to any field. Scale/offset, enum maps, and text encoding all work the same way they do from Rust.

```ts
const schema = Schema.compile({
  fields: [
    { name: "t", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 0, len_bits: 8 }],
      transform: { base: "Int", scale: 0.5, offset: 10 } },
    { name: "status", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 8, len_bits: 8 }],
      transform: { base: "Int", enum_map: { 0: "idle", 1: "running", 2: "error" } } },
  ],
});

const parsed = schema.parse(new Uint8Array([20, 1]));
// { t: { kind: 'f64', value: 20 }, status: { kind: 'string', value: 'running' } }
```

Transforms run automatically during `parse`. There is no separate `applyTransforms` call from TypeScript; the WASM layer invokes them before returning to you.

## Schema shape

The `SchemaDef` type mirrors the Rust `bitspec::serde::SchemaDef` one-for-one, so a JSON file authored for one side parses on the other.

```ts
interface SchemaDef {
  fields: FieldDef[];
  write_config?: { bit_order?: "MsbFirst" | "LsbFirst" };
}

interface FieldDef {
  name: string;
  kind: { type: "Scalar" }
      | { type: "Array"; count: number; stride_bits: number; offset_bits: number };
  signed: boolean;
  assemble: "ConcatMsb" | "ConcatLsb";
  fragments: { offset_bits: number; len_bits: number; bit_order?: "MsbFirst" | "LsbFirst" }[];
  transform?: TransformDef;
}
```

See [`src/types.ts`](./src/types.ts) for the full set of exported types, including `TransformDef`, `BaseDef`, and `EncodingDef`.

## Errors

Every error thrown by `Schema.compile`, `schema.parse`, and `schema.serialize` is a `BitspecError` instance with a stable `.code` string. Catch it and branch on the code:

```ts
import { BitspecError, Schema } from "bitspec";

try {
  schema.parse(new Uint8Array([0]));
} catch (e) {
  if (e instanceof BitspecError) {
    if (e.code === "PACKET_TOO_SHORT") { /* request more data */ }
    else { throw e; }
  }
}
```

The full set of `BitspecErrorCode` values:

| Code | Meaning |
|---|---|
| `INVALID_ARRAY_STRIDE` | Array `stride_bits` is smaller than the element size. |
| `INVALID_ARRAY_COUNT` | Array `count` is zero. |
| `INVALID_FIELD_SIZE` | Scalar field total size is 0 or larger than 64 bits. |
| `INVALID_FRAGMENT` | Fragment has zero length or is otherwise malformed. |
| `INVALID_FIELD_KIND` | Field kind is unsupported. |
| `EMPTY_ARRAY_ELEMENT` | An array element has no fragments. |
| `INVALID_FIELD_NAME` | Field name is empty or duplicates another. |
| `READ_OUT_OF_BOUNDS` | A fragment's bit range extends past the end of the payload. |
| `TOO_MANY_BITS_READ` | More than 64 bits were requested in a single read. |
| `PACKET_TOO_SHORT` | Payload is shorter than the schema's total bit length. |
| `WRITE_OUT_OF_BOUNDS` | The output buffer is too small for the requested write. |
| `INVALID_VALUE` | A value cannot be written to its field (e.g. array length mismatch). |
| `MISSING_FIELD` | `serialize` received an object missing a schema field. |
| `UNSUPPORTED_VALUE` | `serialize` received an `f32`/`f64`/`bytes`/`string` for a scalar field. |
| `INVALID_BASE` | Transform's base type cannot be applied to the given value. |
| `INVALID_TYPE` | Transform config is internally inconsistent (e.g. encoding on non-bytes). |
| `INVALID_ENUM_VALUE` | An integer value has no entry in the transform's enum map. |
| `INVALID_ENCODING` | Bytes are not valid for the chosen encoding (UTF-8 or ASCII). |
| `INVALID_BYTE_VALUE` | A byte element is outside 0..=255. |
| `INVALID_ASCII_BYTE_VALUE` | An ASCII-encoded byte is outside 0..=0x7F. |
| `INVALID_SCALE_OFFSET` | `scale` or `offset` is NaN or infinite. |
| `SCHEMA_JSON_PARSE_ERROR` | `Schema.compile` received a string that is not valid JSON. |
| `INPUT_CONVERSION_ERROR` | A value failed to cross the JS/WASM boundary. |

## Building from source

The package lives inside the [bitcraft repo](https://github.com/somebytes/bitspec). From `packages/bitspec`:

```bash
npm install
npm run build:wasm   # runs wasm-pack against ../../crates/bitspec-wasm
npm run build:ts     # compiles TypeScript to dist/
npm test             # runs the Vitest suite
```

`npm run build` is a shorthand that does the wasm and TS steps in order. `wasm-pack` must be installed globally (see the [wasm-pack installer](https://rustwasm.github.io/wasm-pack/installer/)). The generated `wasm/` and `dist/` directories are what ship in the final package.

## License

MIT. See [LICENSE](https://github.com/somebytes/bitspec/blob/main/LICENSE).
