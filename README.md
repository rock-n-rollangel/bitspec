# bitspec

Runtime bit-level parser and serializer for binary data described by declarative schemas. Use it from Rust via the `bitspec` crate or from JavaScript/TypeScript via the `bitspec` npm package (WebAssembly under the hood).

Most binary protocols in the wild pack fields across awkward bit boundaries: a 4-bit flag squeezed next to a 12-bit counter, a sign bit hiding on byte 3, a measurement split across two non-adjacent ranges. Existing Rust tools (`deku`, `binrw`, `scroll`) solve this with derive macros, which works beautifully when the layout is known at compile time. `bitspec` targets the other half of the problem: when the schema arrives at runtime — loaded from a JSON file, fetched from a device registry, edited by the user — and especially when the consumer is JavaScript and derive macros are not an option. Define fields as bit ranges, compile once, and reuse the schema across many payloads.

## Features

- **Runtime schemas** — `Vec<Field>` or JSON in, compiled `Schema` out. No derive macros, no code generation.
- **Bit-level fragments** — fields can be any width from 1 to 64 bits, at any bit offset, with MSB- or LSB-first ordering.
- **Non-contiguous fields** — one logical value can span multiple disjoint bit ranges and be reassembled.
- **Fixed-size arrays** — repeated elements with a configurable stride, in bits.
- **Optional transforms** — scale/offset, enum maps, UTF-8/ASCII decoding, and IEEE-754 reinterpretation as a post-parse step (gated behind the `transform` feature).
- **Shared schema shape** — the same JSON schema works from Rust (`bitspec::serde::SchemaDef`) and TypeScript (`SchemaDef`).

## What's in this repo

| Path | Purpose |
|---|---|
| [`crates/bitspec`](./crates/bitspec) | Pure-Rust core. Compile a `Schema`, parse bytes, serialize back. |
| [`crates/bitspec-wasm`](./crates/bitspec-wasm) | WebAssembly bindings. Used internally by the npm package. |
| [`packages/bitspec`](./packages/bitspec) | TypeScript wrapper around the WASM bindings. The npm package. |

Each directory has its own `README.md` with usage details appropriate to that surface.

## Quick start — Rust

```rust
use std::collections::BTreeMap;
use bitspec::assembly::{Assemble, BitOrder};
use bitspec::field::{Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::value::Value;

fn main() {
    let fields = vec![
        Field {
            name: "id".into(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 4)],
            transform: None,
        },
        Field {
            name: "payload".into(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(4, 4)],
            transform: None,
        },
    ];
    let schema = Schema::compile(&fields, None).unwrap();

    let parsed = schema.parse(&[0xA5]).unwrap();
    assert_eq!(parsed.get("id"), Some(&Value::U64(0xA)));
    assert_eq!(parsed.get("payload"), Some(&Value::U64(0x5)));

    let obj: BTreeMap<String, Value> = parsed.clone();
    let bytes = schema.serialize(&obj).unwrap();
    assert_eq!(bytes, vec![0xA5]);
}
```

## Quick start — TypeScript

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
console.log(parsed);
// { id: { kind: 'u64', value: 10n }, payload: { kind: 'u64', value: 5n } }

const bytes = schema.serialize(parsed);
console.log(bytes); // Uint8Array(1) [ 165 ]
```

## Repository layout

This is a Cargo workspace plus a standalone npm package. The two Rust crates share a single `Cargo.lock`; the npm package pulls the wasm crate in as a build step via `wasm-pack`.

```
bitcraft/
├── Cargo.toml              # workspace root
├── LICENSE
├── README.md               # this file
├── crates/
│   ├── bitspec/            # pure-Rust core crate
│   └── bitspec-wasm/       # WebAssembly bindings
└── packages/
    └── bitspec/            # TypeScript wrapper (npm package)
```

## Status

Unpublished. APIs are stabilizing toward a 0.1.0 release but not yet available on crates.io or npm. To try it today, clone this repo and depend on the crate via a path or git dependency, or build the TypeScript package from source (see [`packages/bitspec/README.md`](./packages/bitspec/README.md)).

## Contributing

This is a single-author project right now. Issues and small PRs against `main` are welcome once 0.1.0 ships; until then, expect the API to shift as the release prep shakes out the remaining rough edges.

## License

MIT. See [LICENSE](./LICENSE). Copyright (c) 2026 Valera Dolgov.
