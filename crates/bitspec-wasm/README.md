# bitspec-wasm

WebAssembly bindings for [bitspec](../bitspec/). This crate exposes a `WasmSchema` class that compiles JSON schemas and parses/serializes binary payloads from JavaScript.

Most users should install the [`bitspec` npm package](../../packages/bitspec/), which wraps this crate with typed helpers, a `BitspecError` class, `bigint` support for 64-bit integers, and float-bits helpers. This crate exists as the low-level binding; the TypeScript package is the intended public surface.

## Building from source

Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/).

```bash
wasm-pack build --target web
```

The output lands in `pkg/` and can be imported as an ES module:

```js
import init, { WasmSchema } from "./pkg/bitspec_wasm.js";

await init();

const schema = new WasmSchema(JSON.stringify({
  fields: [
    {
      name: "id",
      kind: { type: "Scalar" },
      signed: false,
      assemble: "ConcatMsb",
      fragments: [{ offset_bits: 0, len_bits: 8 }],
    },
  ],
}));

console.log(schema.parse(new Uint8Array([0x42])));
// Map(1) { 'id' => { U64: 66n } }
```

Errors thrown across the WASM boundary have shape `{ code, message }`, where `code` is a stable string such as `"PACKET_TOO_SHORT"` or `"INVALID_FIELD_SIZE"`. The TypeScript wrapper translates these into `BitspecError` instances; if you consume this crate directly, inspect the object fields yourself.

## Status

Unpublished. Not yet on crates.io.

## License

MIT. See [LICENSE](../../LICENSE).
