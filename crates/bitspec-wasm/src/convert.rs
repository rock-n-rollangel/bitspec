//! Helpers for converting JSON schema definitions into core `bitspec` types
//! and JavaScript‑friendly values.
//!
//! This module is internal; its functions back the public
//! [`WasmSchema`](crate::WasmSchema) API by:
//!
//! - Converting [`SchemaDef`](bitspec::serde::SchemaDef) into
//!   `bitspec::field::Field` values.
//! - Building `bitspec::transform::Transform` values from
//!   [`TransformDef`](bitspec::serde::TransformDef).
//! - Converting parsed values into `JsValue` so they can be consumed
//!   ergonomically from JavaScript/TypeScript.
use std::collections::BTreeMap;

use serde::Serialize;
use wasm_bindgen::JsValue;

/// Serializable representation of a parsed value that can be converted to `JsValue`.
///
/// This mirrors [`bitspec::value::Value`] but uses concrete Rust types that
/// can be serialized via `serde` and then passed through `serde_wasm_bindgen`
/// into JavaScript.
#[derive(Serialize)]
#[serde(untagged)]
pub enum JsValueOut {
    Int(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<JsValueOut>),
}

/// Converts a `bitspec::value::Value` into the serializable [`JsValueOut`] shape.
fn value_to_js(v: bitspec::transform::Value) -> JsValueOut {
    match v {
        bitspec::transform::Value::Int(x) => JsValueOut::Int(x),
        bitspec::transform::Value::Float32(x) => JsValueOut::Float32(x),
        bitspec::transform::Value::Float64(x) => JsValueOut::Float64(x),
        bitspec::transform::Value::String(x) => JsValueOut::String(x),
        bitspec::transform::Value::Bytes(x) => JsValueOut::Bytes(x),
        bitspec::transform::Value::Array(xs) => {
            JsValueOut::Array(xs.into_iter().map(value_to_js).collect())
        }
    }
}

/// Converts a map of parsed values into a JavaScript object.
///
/// Keys are field names and values are first converted into [`JsValueOut`]
/// and then into `JsValue` via `serde_wasm_bindgen`.
pub fn map_to_js(map: BTreeMap<String, bitspec::transform::Value>) -> Result<JsValue, JsValue> {
    let out: BTreeMap<String, JsValueOut> =
        map.into_iter().map(|(k, v)| (k, value_to_js(v))).collect();

    serde_wasm_bindgen::to_value(&out).map_err(error_to_js)
}

/// Converts any debug‑printable error into a `JsValue` with a human‑readable message.
///
/// This keeps the surface area of error handling small on the JavaScript side
/// while still retaining detailed information that can be logged or surfaced
/// in developer tools.
pub fn error_to_js<T>(e: T) -> JsValue
where
    T: std::fmt::Debug,
{
    JsValue::from_str(&format!("{e:?}"))
}

pub fn convert_json_value(v: serde_json::Value) -> Result<bitspec::value::Value, JsValue> {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(bitspec::value::Value::I64(i));
            }

            if let Some(u) = n.as_u64() {
                return Ok(bitspec::value::Value::U64(u));
            }

            if let Some(f) = n.as_f64() {
                return Ok(bitspec::value::Value::U64(f.to_bits())); // float as raw bits
            }

            Err(JsValue::from_str("Invalid number"))
        }

        serde_json::Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(convert_json_value(item)?);
            }
            Ok(bitspec::value::Value::Array(out))
        }

        _ => Err(JsValue::from_str("Unsupported value type")),
    }
}
