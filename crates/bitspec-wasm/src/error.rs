//! Flat error shape used at the WASM boundary.
//!
//! Typed Rust errors (`CompileError`, `ReadError`, `WriteError`, `TransformError`)
//! are converted to `WasmError` via `From` impls before being serialized across
//! the WASM boundary as `{ code, message }`.

use bitspec::errors::{CompileError, ReadError, WriteError};
use serde::Serialize;

/// Flat error type serialized to JS as `{ code, message }`.
#[derive(Debug, Serialize, PartialEq)]
pub struct WasmError {
    pub code: &'static str,
    pub message: String,
}

impl WasmError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl From<CompileError> for WasmError {
    fn from(e: CompileError) -> Self {
        let code = match e {
            CompileError::InvalidArrayStride => "INVALID_ARRAY_STRIDE",
            CompileError::InvalidArrayCount => "INVALID_ARRAY_COUNT",
            CompileError::InvalidFieldSize => "INVALID_FIELD_SIZE",
            CompileError::InvalidFragment => "INVALID_FRAGMENT",
            CompileError::InvalidFieldKind => "INVALID_FIELD_KIND",
            CompileError::EmptyArrayElement => "EMPTY_ARRAY_ELEMENT",
            CompileError::InvalidFieldName => "INVALID_FIELD_NAME",
        };
        WasmError::new(code, e.to_string())
    }
}

impl From<ReadError> for WasmError {
    fn from(e: ReadError) -> Self {
        let code = match e {
            ReadError::OutOfBounds => "READ_OUT_OF_BOUNDS",
            ReadError::TooManyBitsRead => "TOO_MANY_BITS_READ",
            ReadError::PacketTooShort => "PACKET_TOO_SHORT",
        };
        WasmError::new(code, e.to_string())
    }
}

impl From<WriteError> for WasmError {
    fn from(e: WriteError) -> Self {
        let code = match &e {
            WriteError::OutOfBounds => "WRITE_OUT_OF_BOUNDS",
            WriteError::InvalidValue => "INVALID_VALUE",
            WriteError::MissingField(_) => "MISSING_FIELD",
            WriteError::UnsupportedValue { .. } => "UNSUPPORTED_VALUE",
        };
        WasmError::new(code, e.to_string())
    }
}

impl From<bitspec::transform::TransformError> for WasmError {
    fn from(e: bitspec::transform::TransformError) -> Self {
        use bitspec::transform::TransformError;
        let code = match e {
            TransformError::InvalidBase => "INVALID_BASE",
            TransformError::InvalidType => "INVALID_TYPE",
            TransformError::InvalidEnumValue(_) => "INVALID_ENUM_VALUE",
            TransformError::InvalidEncoding => "INVALID_ENCODING",
            TransformError::InvalidByteValue => "INVALID_BYTE_VALUE",
            TransformError::InvalidAsciiByteValue => "INVALID_ASCII_BYTE_VALUE",
            TransformError::InvalidScaleOffset => "INVALID_SCALE_OFFSET",
        };
        WasmError::new(code, e.to_string())
    }
}

impl From<serde_wasm_bindgen::Error> for WasmError {
    fn from(e: serde_wasm_bindgen::Error) -> Self {
        WasmError::new("INPUT_CONVERSION_ERROR", e.to_string())
    }
}

impl From<serde_json::Error> for WasmError {
    fn from(e: serde_json::Error) -> Self {
        WasmError::new("SCHEMA_JSON_PARSE_ERROR", e.to_string())
    }
}

impl From<WasmError> for wasm_bindgen::JsValue {
    fn from(e: WasmError) -> Self {
        serde_wasm_bindgen::to_value(&e)
            .unwrap_or_else(|_| wasm_bindgen::JsValue::from_str("serialization of WasmError failed"))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn compile_errors_map_correctly() {
        assert_eq!(WasmError::from(CompileError::InvalidArrayStride).code, "INVALID_ARRAY_STRIDE");
        assert_eq!(WasmError::from(CompileError::InvalidFieldSize).code, "INVALID_FIELD_SIZE");
        assert_eq!(WasmError::from(CompileError::InvalidFieldName).code, "INVALID_FIELD_NAME");
    }

    #[test]
    fn read_errors_map_correctly() {
        assert_eq!(WasmError::from(ReadError::OutOfBounds).code, "READ_OUT_OF_BOUNDS");
        assert_eq!(WasmError::from(ReadError::PacketTooShort).code, "PACKET_TOO_SHORT");
        assert_eq!(WasmError::from(ReadError::TooManyBitsRead).code, "TOO_MANY_BITS_READ");
    }

    #[test]
    fn write_errors_map_correctly() {
        assert_eq!(WasmError::from(WriteError::OutOfBounds).code, "WRITE_OUT_OF_BOUNDS");
        assert_eq!(WasmError::from(WriteError::InvalidValue).code, "INVALID_VALUE");
        assert_eq!(WasmError::from(WriteError::MissingField("x".into())).code, "MISSING_FIELD");
        assert_eq!(
            WasmError::from(WriteError::UnsupportedValue {
                field: "x".into(),
                variant: "F64"
            })
            .code,
            "UNSUPPORTED_VALUE"
        );
    }

    #[test]
    fn transform_errors_map_correctly() {
        use bitspec::transform::TransformError;
        assert_eq!(WasmError::from(TransformError::InvalidBase).code, "INVALID_BASE");
        assert_eq!(WasmError::from(TransformError::InvalidEnumValue(7)).code, "INVALID_ENUM_VALUE");
        assert_eq!(WasmError::from(TransformError::InvalidType).code, "INVALID_TYPE");
    }
}
