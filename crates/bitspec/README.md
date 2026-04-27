# bitspec

Runtime bit-level parser and serializer for binary data described by declarative schemas. Define fields as bit ranges (possibly non-contiguous), compile them into a `Schema`, then parse byte slices into named values or serialize a map of values back to bytes.

`bitspec` is aimed at protocols whose layout is only known at runtime: telemetry frames loaded from a configuration file, sensor payloads described by a device registry, or binary formats that a user can edit. The core is a plain Rust library, but the crate is also the engine behind a WebAssembly binding and an npm package — so the same schema shape can be used from both Rust and JavaScript.

## Why bitspec

The Rust ecosystem already has excellent binary-format libraries. They divide cleanly by where the schema lives:

- [`deku`](https://docs.rs/deku) and [`binrw`](https://docs.rs/binrw) use derive macros to turn a Rust struct into a reader/writer. The schema is the struct definition, fixed at compile time. Fantastic when you know the layout up front and want the compiler to check it.
- [`scroll`](https://docs.rs/scroll) gives you fast, endian-aware `Pread`/`Pwrite` traits over byte slices for a fixed set of primitive types. Also compile-time; schemas are code.
- [`bitvec`](https://docs.rs/bitvec) paired with [`winnow`](https://docs.rs/winnow) or [`nom`](https://docs.rs/nom) lets you assemble a bit-level parser by hand. Maximum flexibility, but you own every decision.

`bitspec` occupies a different niche: the schema is data, not code. You build a `Vec<Field>` at runtime, hand it to `Schema::compile`, and reuse the compiled schema across many payloads. The schema can also be deserialized from JSON (`serde` feature), which is what the WebAssembly binding and the TypeScript wrapper lean on. Use `bitspec` when your schemas come from outside your binary — a file, a remote service, user input — or when you need to share the exact same layout description between Rust and JavaScript. Reach for `deku`/`binrw` when your layouts are static and you want the full power of the Rust type system; reach for `scroll` when you are doing simple endian-aware reads over primitive types; reach for `bitvec` + a parser combinator crate when your encoding is unusual enough to need hand-written logic.

## Installation

```toml
[dependencies]
bitspec = "0.1"
```

> Note: `bitspec` is not yet published to crates.io. For now, depend on it via git or a local path, for example:
>
> ```toml
> [dependencies]
> bitspec = { git = "https://github.com/somebytes/bitspec" }
> ```

## Feature flags

- `serde` — enables the `serde::SchemaDef` family of JSON-deserializable types and `Serialize`/`Deserialize` impls on `Value`.
- `transform` — enables `Schema::apply_transforms` and `Transform::apply` so you can attach scale/offset, enum maps, and text decoding to fields.

Both flags are off by default. For full functionality:

```toml
[dependencies]
bitspec = { version = "0.1", features = ["serde", "transform"] }
```

## Core concepts

- **`Fragment`** — a contiguous bit range (`offset_bits`, `len_bits`) with an optional per-fragment `BitOrder`. The building block every field is made of.
- **`Field`** — a named `Scalar` or fixed-size `Array` of scalars. Points at one or more fragments and says how they combine (`Assemble::Concat(BitOrder::MsbFirst | LsbFirst)`), whether the assembled value is signed, and optionally carries a `Transform`.
- **`Schema`** — the compiled result. Produced by `Schema::compile(&[Field], Option<WriteConfig>)`, it knows the total bit length and exposes `parse`, `serialize`, and (with `transform`) `apply_transforms`.
- **`Value`** — a 7-variant enum (`U64`, `I64`, `F32`, `F64`, `Bytes`, `String`, `Array`) used for both parse output and serialize input. Parse emits `U64`/`I64`/`Array`; transforms can widen the type set; serialize currently accepts `U64`/`I64`/`Array`.

## Parsing bytes

```rust
use bitspec::assembly::{Assemble, BitOrder};
use bitspec::field::{Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::value::Value;

let fields = vec![
    Field {
        name: "version".into(),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(0, 4)],
        transform: None,
    },
    Field {
        name: "length".into(),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(4, 12)],
        transform: None,
    },
];

let schema = Schema::compile(&fields, None).unwrap();
let parsed = schema.parse(&[0x21, 0x5A]).unwrap();

assert_eq!(parsed.get("version"), Some(&Value::U64(0x2)));
assert_eq!(parsed.get("length"), Some(&Value::U64(0x15A)));
```

`parse` returns a `BTreeMap<String, Value>`. Fields are inserted in compile order, but the `BTreeMap` will iterate alphabetically — rely on lookup by name, not iteration order, if layout order matters to you.

## Serializing values

`serialize` is the inverse. Construct a `BTreeMap<String, Value>` keyed by field name, and you get the minimum number of bytes needed to hold every field.

```rust
use std::collections::BTreeMap;
use bitspec::assembly::{Assemble, BitOrder};
use bitspec::field::{Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::value::Value;

let fields = vec![
    Field {
        name: "a".into(), kind: FieldKind::Scalar, signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(0, 4)], transform: None,
    },
    Field {
        name: "b".into(), kind: FieldKind::Scalar, signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(4, 4)], transform: None,
    },
];
let schema = Schema::compile(&fields, None).unwrap();

let obj = BTreeMap::from([
    ("a".to_string(), Value::U64(0b1010)),
    ("b".to_string(), Value::U64(0b0101)),
]);

let bytes = schema.serialize(&obj).unwrap();
assert_eq!(bytes, vec![0b1010_0101]);

// Roundtrip back to a map of values.
let parsed = schema.parse(&bytes).unwrap();
assert_eq!(parsed, obj);
```

`serialize` accepts `Value::U64`, `Value::I64`, and `Value::Array`. Passing a `Value::F32`, `Value::F64`, `Value::Bytes`, or `Value::String` (which transforms can produce) returns `WriteError::UnsupportedValue` — if you want to write those, use `floatBits32`/`floatBits64`-style conversion on the caller side to pack them into a `U64` first.

## Arrays

Use `FieldKind::Array(ArraySpec { count, stride_bits, offset_bits })` to describe a fixed-count array whose elements sit at regular intervals. The element layout is whatever the field's `fragments` describe; the array repeats that layout `count` times with `stride_bits` between starts, beginning at `offset_bits`.

```rust
use bitspec::assembly::{Assemble, BitOrder};
use bitspec::field::{ArraySpec, Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::value::Value;

let samples = Field {
    name: "samples".into(),
    kind: FieldKind::Array(ArraySpec { count: 4, stride_bits: 8, offset_bits: 0 }),
    signed: false,
    assemble: Assemble::Concat(BitOrder::MsbFirst),
    fragments: vec![Fragment::new(0, 8)],
    transform: None,
};
let schema = Schema::compile(&[samples], None).unwrap();

let parsed = schema.parse(&[0x01, 0x02, 0x03, 0x04]).unwrap();
assert_eq!(
    parsed.get("samples"),
    Some(&Value::Array(vec![
        Value::U64(1), Value::U64(2), Value::U64(3), Value::U64(4),
    ])),
);
```

## Non-contiguous fragments

Real protocols occasionally scatter the bits of one logical value across a payload — a 12-bit counter split 4+8 across two bytes because the byte boundary was forced by some other field. Multiple fragments, listed in MSB-first order, let you reassemble the value cleanly.

```rust
use bitspec::assembly::{Assemble, BitOrder};
use bitspec::field::{Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::value::Value;

// High 4 bits of the counter live in the low nibble of byte 0.
// Low 8 bits live in byte 1.
let counter = Field {
    name: "counter".into(),
    kind: FieldKind::Scalar,
    signed: false,
    assemble: Assemble::Concat(BitOrder::MsbFirst),
    fragments: vec![
        Fragment::new(4, 4),   // high nibble, in bits 4..8
        Fragment::new(8, 8),   // low byte, in bits 8..16
    ],
    transform: None,
};
let schema = Schema::compile(&[counter], None).unwrap();

let parsed = schema.parse(&[0x0A, 0xBC]).unwrap();
assert_eq!(parsed.get("counter"), Some(&Value::U64(0xABC)));
```

## Transforms

A `Transform` is an optional per-field post-processor. Reinterpret the raw bits as a float, scale and offset an integer measurement, map integer codes to string labels, or decode a byte array as UTF-8/ASCII.

```rust
use std::collections::BTreeMap;
use bitspec::assembly::{Assemble, BitOrder};
use bitspec::field::{Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::transform::{Base, Transform};
use bitspec::value::Value;

let mut transform = Transform::new(Base::Int);
transform.set_scale(0.5).set_offset(10.0);

let temperature = Field {
    name: "temperature".into(),
    kind: FieldKind::Scalar,
    signed: false,
    assemble: Assemble::Concat(BitOrder::MsbFirst),
    fragments: vec![Fragment::new(0, 8)],
    transform: Some(transform),
};

let schema = Schema::compile(&[temperature], None).unwrap();
let raw = schema.parse(&[20]).unwrap();
assert_eq!(raw.get("temperature"), Some(&Value::U64(20)));

let cooked = schema.apply_transforms(raw).unwrap();
assert_eq!(cooked.get("temperature"), Some(&Value::F64(20.0))); // 20 * 0.5 + 10
```

`apply_transforms` takes ownership of the map so it can move values through without cloning. For an enum map:

```rust
use std::collections::HashMap;
use bitspec::transform::{Base, Transform};

let mut status = Transform::new(Base::Int);
status.set_enum_map(HashMap::from([
    (0, "idle".to_string()),
    (1, "running".to_string()),
    (2, "error".to_string()),
]));
```

For decoding a fixed-length byte array as a string, pair `Base::Bytes` with an `Encoding` — see [`src/transform.rs`](./src/transform.rs) for the full feature set (zero-termination, whitespace trimming, ASCII vs UTF-8).

## JSON-described schemas (`serde` feature)

With the `serde` feature, every schema shape has a `*Def` twin that implements `Deserialize`. Read a schema from JSON and compile it in two lines.

```rust
use bitspec::schema::Schema;
use bitspec::serde::SchemaDef;
use bitspec::value::Value;

let json = r#"{
    "fields": [
        {
            "name": "id",
            "kind": { "type": "Scalar" },
            "signed": false,
            "assemble": "ConcatMsb",
            "fragments": [{ "offset_bits": 0, "len_bits": 8 }]
        }
    ]
}"#;

let def: SchemaDef = serde_json::from_str(json).unwrap();
let schema: Schema = def.try_into().unwrap();
let parsed = schema.parse(&[0x42]).unwrap();
assert_eq!(parsed.get("id"), Some(&Value::U64(0x42)));
```

`Value` also gains `Serialize` and `Deserialize` with the `serde` feature, using the same externally-tagged shape the TypeScript wrapper expects: `{"U64": 42}`, `{"Array": [{"U64": 1}, ...]}`, and so on.

## Error handling

Four error types cover the four phases of use:

- **`CompileError`** — returned by `Schema::compile`. Invalid field size (0 or >64 bits), invalid fragment, array stride smaller than element size, empty or duplicate field names, etc.
- **`ReadError`** — returned by `Schema::parse`. `PacketTooShort` if the input is smaller than the schema's total bit length; `OutOfBounds` / `TooManyBitsRead` for lower-level read issues.
- **`WriteError`** — returned by `Schema::serialize`. `MissingField` when the input map is missing a name; `UnsupportedValue` when a value variant (e.g. `F64`) cannot be serialized; `InvalidValue` for type/shape mismatches like array length.
- **`TransformError`** — returned by `Schema::apply_transforms` (and `Transform::apply`). Covers invalid base/type combinations, missing enum map entries, non-UTF-8 bytes, etc.

All four implement `std::error::Error` and `Display`.

## Performance

The internal `read_bits_at` routine coalesces adjacent byte reads where possible, and `Schema::parse` walks the field list without allocating beyond the result map. On the write side there is still a known opportunity for a byte-level fast path in `write_bits_at` when the fragment is byte-aligned, which would avoid the per-bit shift loop. Formal benchmarks live under `benches/schema_parse.rs` and will gain hard numbers ahead of the 0.1.0 release.

## Minimum supported Rust version

Latest stable Rust, edition 2024.

## License

MIT. See [LICENSE](../../LICENSE).
