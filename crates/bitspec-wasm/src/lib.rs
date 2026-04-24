//! WASM bindings for the `bitspec` binary schema engine.
//!
//! This crate exposes a compact API to JavaScript for parsing binary
//! payloads according to a JSON schema definition. Internally it uses
//! the `bitspec` crate to describe how bits are laid out in a payload
//! and the `bitspec::transform` module to turn raw values into
//! human‑friendly data (scaling, offsets, text decoding, enums, etc.).
//!
//! At a high level you:
//! - **Describe your fields** in JSON using the shape in `bitspec::serde::SchemaDef`
//!   (field name, kind, bit fragments, signedness, etc.).
//! - **Optionally attach transforms** using the shape in `bitspec::serde::TransformDef`
//!   (base type, scaling, encoding, enum map, …).
//! - **Compile** the schema once, and **parse** binary payloads many
//!   times from JavaScript.
//!
//! The entry point from JS is the [`WasmSchema`] type:
//!
//! ```text
//! // Pseudo TypeScript example
//! //
//! // const schemaJson = JSON.stringify({
//! //   fields: [
//! //     {
//! //       name: "id",
//! //       kind: { type: "Scalar" },
//! //       signed: false,
//! //       assemble: "ConcatMsb",
//! //       fragments: [{ offset_bits: 0, len_bits: 16 }],
//! //       transform: { base: "Int", scale: 0.5, offset: 100 }
//! //     }
//! //   ]
//! // });
//! //
//! // const wasmSchema = new WasmSchema(schemaJson);
//! // const result = wasmSchema.parse(someUint8Array);
//! // // result is a JS object: { id: 123.5 }
//! ```
//!
//! Error values are converted to `JsValue` with a `Debug` representation,
//! which makes it easy to inspect failures from JavaScript.

mod convert;
mod error;

use bitspec::serde::SchemaDef;
use wasm_bindgen::prelude::*;

/// Compiled schema that can be used from JavaScript to parse binary data.
///
/// A `WasmSchema` owns a compiled [`bitspec::schema::Schema`] plus any
/// per‑field transforms that should be applied to the raw values.
///
/// Typical usage from JavaScript/TypeScript is:
///
/// ```text
/// // const schema = new WasmSchema(schemaJson);
/// // const parsed = schema.parse(bytes);
/// // console.log(parsed.someField);
/// ```
#[wasm_bindgen]
pub struct WasmSchema {
    /// Compiled bit‑level schema describing how to read the payload.
    schema: bitspec::schema::Schema,
}

#[wasm_bindgen]
impl WasmSchema {
    /// Creates a new compiled schema from a JSON definition.
    ///
    /// The `schema_json` string must deserialize into [`SchemaDef`], which
    /// in turn describes:
    ///
    /// - **Fields**: their name, kind (scalar or fixed‑size array),
    ///   signedness and assemble strategy.
    /// - **Fragments**: the bit ranges that make up each field.
    /// - **Transforms** (optional): how to post‑process raw values using
    ///   `bitspec::transform` (base type, scale/offset, encodings, enums).
    ///
    /// On success this compiles the schema and prepares any transforms so
    /// that it can be reused to parse many payloads efficiently.
    #[wasm_bindgen(constructor)]
    pub fn new(schema_json: &str) -> Result<WasmSchema, JsValue> {
        let def: SchemaDef = serde_json::from_str(schema_json)
            .map_err(|e| JsValue::from(error::WasmError::from(e)))?;
        let schema = bitspec::schema::Schema::try_from(def)
            .map_err(|e| JsValue::from(error::WasmError::from(e)))?;
        Ok(WasmSchema { schema })
    }

    /// Parses a binary payload according to this compiled schema.
    ///
    /// - `data` is the raw byte slice (for example a `Uint8Array` passed from JS).
    /// - The return value is a JavaScript object (`JsValue`) where keys are
    ///   field names and values have been converted through any configured
    ///   transforms (see [`schema_def_to_transforms`](crate::convert::schema_def_to_transforms)).
    ///
    /// On error a `JsValue` containing a debug string is returned.
    pub fn parse(&self, data: &[u8]) -> Result<JsValue, JsValue> {
        let result = self
            .schema
            .parse(data)
            .map_err(|e| JsValue::from(error::WasmError::from(e)))?;
        let transformed = self
            .schema
            .apply_transforms(result)
            .map_err(|e| JsValue::from(error::WasmError::from(e)))?;
        convert::map_to_js(transformed)
    }

    pub fn serialize(&self, obj: JsValue) -> Result<Vec<u8>, JsValue> {
        let map: std::collections::BTreeMap<String, bitspec::value::Value> =
            serde_wasm_bindgen::from_value(obj)
                .map_err(|e| JsValue::from(error::WasmError::from(e)))?;
        self.schema
            .serialize(&map)
            .map_err(|e| JsValue::from(error::WasmError::from(e)))
    }
}
