//! Helpers for converting between bitspec core types and JavaScript values.

use std::collections::BTreeMap;
use wasm_bindgen::JsValue;

/// Converts a map of parsed values directly into a JavaScript object via
/// `serde_wasm_bindgen`. The serde representation of `Value` is externally
/// tagged: `{"U64": 42}`, `{"F64": 3.14}`, etc.
///
/// u64/i64 values are serialized as JS bigint to avoid precision loss above 2^53.
pub fn map_to_js(
    map: BTreeMap<String, bitspec::value::Value>,
) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_large_number_types_as_bigints(true);
    map.serialize(&serializer).map_err(error_to_js)
}

pub fn error_to_js<T>(e: T) -> JsValue
where
    T: std::fmt::Debug,
{
    JsValue::from_str(&format!("{e:?}"))
}
