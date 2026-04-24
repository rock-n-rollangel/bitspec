//! A [`Transform`] describes how to interpret and optionally modify values:
//! - **Base type**: How to reinterpret raw bytes (integer, float32, float64, or byte array).
//! - **Numeric modifiers**: Optional `scale` and `offset` applied as `value * scale + offset`.
//! - **String decoding**: For byte arrays, optional UTF-8 or ASCII decoding with zero-termination and trim.
//! - **Enum mapping**: For integers, optional mapping from numeric values to string labels.
//!
//! ## Transform order
//!
//! Transforms are applied in the following order:
//! 1. Base reinterpretation
//! 2. Numeric modifiers (scale, offset)
//! 3. Enum mapping
//! 4. String decoding

use std::collections::HashMap;

/// Errors that can occur when applying a transform to a raw value.
#[derive(Debug, PartialEq, Eq)]
pub enum TransformError {
    /// The raw value cannot be interpreted as the requested base type.
    InvalidBase,
    /// The value type does not match what the transform expects (e.g. encoding on non-bytes).
    InvalidType,
    /// An integer value has no entry in the enum map.
    InvalidEnumValue(i64),
    /// Byte sequence is not valid for the chosen encoding (e.g. invalid UTF-8).
    InvalidEncoding,
    /// A byte element is outside 0..=255 (e.g. in a bytes array).
    InvalidByteValue,
    /// An ASCII-encoded byte is outside 0..=0x7F.
    InvalidAsciiByteValue,
    /// Scale or offset is non-finite (NaN or infinity).
    InvalidScaleOffset,
}

/// Base interpretation for raw assembly values.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Base {
    /// Interpret as 64-bit signed or unsigned integer.
    Int,
    /// Reinterpret 32 bits as an IEEE 754 float.
    Float32,
    /// Reinterpret 64 bits as an IEEE 754 double.
    Float64,
    /// Treat an array of byte-sized values as a byte buffer.
    Bytes,
}

impl Default for Base {
    fn default() -> Self {
        Base::Int
    }
}

/// Character encoding for decoding byte arrays to strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8. Any valid UTF-8 byte sequence is accepted.
    Utf8,
    /// ASCII. Every byte must be in 0..=0x7F.
    Ascii,
}

/// Configuration for transforming raw [`crate::value::Value`] into [`crate::value::Value`]s.
///
/// Use the builder-style setters (`set_scale`, `set_encoding`, etc.) to configure,
/// then call [`apply`](Transform::apply) with a raw value.
///
/// Applying scale or offset always produces a floating-point result.
///
/// # Example
///
/// ```
/// use crate_transform::{Transform, Base, Value};
///
/// let mut transform = Transform::new(Base::Int);
/// transform.set_scale(2.0).set_offset(1.0);
/// let raw = crate::value::Value::I64(10);
/// let result = transform.apply(raw).unwrap();
/// assert_eq!(result, Value::Float64(21.0));
/// ```
#[derive(Debug, Clone)]
pub struct Transform {
    /// How to interpret the raw value (int, float32, float64, or bytes).
    pub base: Base,
    /// If set, multiply numeric value by this before adding offset.
    pub scale: Option<f64>,
    /// If set, add this to the (possibly scaled) numeric value.
    pub offset: Option<f64>,

    /// If set (only valid for `Base::Bytes`), decode bytes to a string using this encoding.
    pub encoding: Option<Encoding>,
    /// If true, truncate at the first null byte before decoding. Only used when encoding is set.
    pub zero_terminated: Option<bool>,
    /// If true, trim leading/trailing whitespace from decoded strings.
    pub trim: Option<bool>,

    /// If set (only valid for `Base::Int`), map integer values to string labels.
    pub enum_map: Option<HashMap<i64, String>>,
}

#[cfg(feature = "serde")]
impl TryFrom<crate::serde::TransformDef> for Transform {
    type Error = crate::errors::CompileError;

    fn try_from(value: crate::serde::TransformDef) -> Result<Self, Self::Error> {
        Ok(Transform {
            base: match value.base {
                crate::serde::BaseDef::Int => Base::Int,
                crate::serde::BaseDef::Float32 => Base::Float32,
                crate::serde::BaseDef::Float64 => Base::Float64,
                crate::serde::BaseDef::Bytes => Base::Bytes,
            },
            scale: value.scale,
            offset: value.offset,
            encoding: match value.encoding {
                Some(crate::serde::EncodingDef::Utf8) => Some(Encoding::Utf8),
                Some(crate::serde::EncodingDef::Ascii) => Some(Encoding::Ascii),
                None => None,
            },
            zero_terminated: value.zero_terminated,
            trim: value.trim,
            enum_map: value.enum_map.clone(),
        })
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            base: Default::default(),
            scale: None,
            offset: None,
            encoding: None,
            zero_terminated: None,
            trim: None,
            enum_map: None,
        }
    }
}

impl Transform {
    /// Creates a new transform with the given base type and default options.
    pub fn new(base: Base) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    /// Sets the scale factor: result = value * scale + offset.
    pub fn set_scale(&mut self, scale: f64) -> &mut Self {
        self.scale = Some(scale);
        self
    }

    /// Sets the offset: result = value * scale + offset.
    pub fn set_offset(&mut self, offset: f64) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the encoding for decoding byte arrays to strings (requires `Base::Bytes`).
    pub fn set_encoding(&mut self, encoding: Encoding) -> &mut Self {
        self.encoding = Some(encoding);
        self
    }

    /// If true, truncate at the first null byte before decoding to string.
    pub fn set_zero_terminated(&mut self, zero_terminated: bool) -> &mut Self {
        self.zero_terminated = Some(zero_terminated);
        self
    }

    /// If true, trim leading and trailing whitespace from decoded strings.
    pub fn set_trim(&mut self, trim: bool) -> &mut Self {
        self.trim = Some(trim);
        self
    }

    /// Sets the enum map for mapping integer values to string labels (requires `Base::Int`).
    pub fn set_enum_map(&mut self, enum_map: HashMap<i64, String>) -> &mut Self {
        self.enum_map = Some(enum_map);
        self
    }
}

impl Transform {
    /// Applies the transform to a single scalar value (no array handling).
    fn apply_scalar(&self, raw: crate::value::Value) -> Result<crate::value::Value, TransformError> {
        let mut v = reinterpret_base(&self.base, raw)?;
        v = apply_numeric_modifiers(v, self.scale, self.offset)?;
        v = apply_enum(v, &self.enum_map)?;
        v = apply_string(v, &self.encoding, self.zero_terminated, self.trim)?;
        Ok(v)
    }

    /// Transforms a raw value into a [`crate::value::Value`].
    ///
    /// Validates the transform configuration first. For arrays, applies the transform
    /// to each element. For `Base::Bytes`, expects an array of byte-sized values.
    pub fn apply(&self, raw: crate::value::Value) -> Result<crate::value::Value, TransformError> {
        use crate::value::Value;
        self.validate()?;

        if self.base == Base::Bytes {
            let bytes = extract_bytes(raw)?;
            let v = Value::Bytes(bytes);
            return apply_string(v, &self.encoding, self.zero_terminated, self.trim);
        }

        match raw {
            Value::Array(values) => {
                let mut out = Vec::with_capacity(values.len());
                for v in values {
                    out.push(self.apply_scalar(v)?);
                }
                Ok(Value::Array(out))
            }
            _ => self.apply_scalar(raw),
        }
    }

    /// Checks that scale/offset and base/encoding/enum_map combinations are valid.
    fn validate(&self) -> Result<(), TransformError> {
        if self.scale.is_some() && !self.scale.unwrap().is_finite() {
            return Err(TransformError::InvalidScaleOffset);
        }

        if self.base == Base::Bytes && self.enum_map.is_some() {
            return Err(TransformError::InvalidType);
        }

        if let Some(_) = self.encoding {
            if &self.base != &Base::Bytes {
                return Err(TransformError::InvalidType);
            }
        }

        if let Some(_) = self.enum_map {
            if &self.base != &Base::Int {
                return Err(TransformError::InvalidType);
            }
        }

        Ok(())
    }
}

/// Interprets a raw assembly value according to the given base type (int/float32/float64).
/// Bytes base is not handled here; use `extract_bytes` for that.
fn reinterpret_base(base: &Base, value: crate::value::Value) -> Result<crate::value::Value, TransformError> {
    use crate::value::Value;
    match (base, value) {
        // INT: preserve sign (do NOT collapse U64 into I64).
        (Base::Int, Value::U64(v)) => Ok(Value::U64(v)),
        (Base::Int, Value::I64(v)) => Ok(Value::I64(v)),

        // FLOAT32: reinterpret low 32 bits of U64 as f32.
        (Base::Float32, Value::U64(v)) => Ok(Value::F32(f32::from_bits(v as u32))),

        // FLOAT64: reinterpret all 64 bits of U64 as f64.
        (Base::Float64, Value::U64(v)) => Ok(Value::F64(f64::from_bits(v))),

        // BYTES: handled by caller (extract_bytes path).
        (Base::Bytes, _) => Err(TransformError::InvalidBase),

        _ => Err(TransformError::InvalidBase),
    }
}

/// Extracts a byte vector from an array of byte-sized U64/I64 values.
fn extract_bytes(raw: crate::value::Value) -> Result<Vec<u8>, TransformError> {
    use crate::value::Value;
    match raw {
        Value::Array(values) => {
            let mut bytes = Vec::with_capacity(values.len());
            for v in values {
                match v {
                    Value::U64(x) => {
                        if x > 255 {
                            return Err(TransformError::InvalidByteValue);
                        }
                        bytes.push(x as u8);
                    }
                    Value::I64(x) => {
                        if !(0..=255).contains(&x) {
                            return Err(TransformError::InvalidByteValue);
                        }
                        bytes.push(x as u8);
                    }
                    _ => return Err(TransformError::InvalidType),
                }
            }
            Ok(bytes)
        }
        _ => Err(TransformError::InvalidType),
    }
}

/// Applies scale and offset to numeric values: value * scale + offset.
fn apply_numeric_modifiers(
    value: crate::value::Value,
    scale: Option<f64>,
    offset: Option<f64>,
) -> Result<crate::value::Value, TransformError> {
    use crate::value::Value;
    if scale.is_none() && offset.is_none() {
        return Ok(value);
    }
    let scale = scale.unwrap_or(1.0);
    let offset = offset.unwrap_or(0.0);

    match value {
        Value::U64(v) => Ok(Value::F64(v as f64 * scale + offset)),
        Value::I64(v) => Ok(Value::F64(v as f64 * scale + offset)),
        Value::F32(v) => Ok(Value::F32(v * scale as f32 + offset as f32)),
        Value::F64(v) => Ok(Value::F64(v * scale + offset)),
        other => Ok(other),
    }
}

/// If encoding is set, decodes bytes to a string (UTF-8 or ASCII), optionally zero-terminated and trimmed.
fn apply_string(
    value: crate::value::Value,
    encoding: &Option<Encoding>,
    zero_terminated: Option<bool>,
    trim: Option<bool>,
) -> Result<crate::value::Value, TransformError> {
    use crate::value::Value;
    let encoding = match encoding {
        Some(e) => e,
        None => return Ok(value),
    };
    let mut bytes = match value {
        Value::Bytes(b) => b,
        _ => return Err(TransformError::InvalidType),
    };
    if zero_terminated.unwrap_or(false) {
        if let Some(pos) = bytes.iter().position(|b| *b == 0) {
            bytes.truncate(pos);
        }
    }
    let mut s = match encoding {
        Encoding::Ascii => {
            for b in &bytes {
                if *b > 0x7F {
                    return Err(TransformError::InvalidAsciiByteValue);
                }
            }
            String::from_utf8(bytes).map_err(|_| TransformError::InvalidEncoding)?
        }
        Encoding::Utf8 => String::from_utf8(bytes).map_err(|_| TransformError::InvalidEncoding)?,
    };
    if trim.unwrap_or(false) {
        s = s.trim().to_string();
    }
    Ok(Value::String(s))
}

/// If enum_map is set, maps an integer value to its string label.
fn apply_enum(
    value: crate::value::Value,
    enum_map: &Option<std::collections::HashMap<i64, String>>,
) -> Result<crate::value::Value, TransformError> {
    use crate::value::Value;
    if let Some(map) = enum_map {
        match value {
            Value::I64(v) => map
                .get(&v)
                .map(|s| Value::String(s.clone()))
                .ok_or(TransformError::InvalidEnumValue(v)),
            Value::U64(v) => map
                .get(&(v as i64))
                .map(|s| Value::String(s.clone()))
                .ok_or(TransformError::InvalidEnumValue(v as i64)),
            _ => Err(TransformError::InvalidType),
        }
    } else {
        Ok(value)
    }
}

#[cfg(test)]
use crate::value::Value;

#[test]
fn test_float32_from_bits() {
    let transform = Transform {
        base: Base::Float32,
        scale: None,
        offset: Some(0.1),
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    let raw = crate::value::Value::U64(0x40490FDB);
    let result = transform.apply(raw).unwrap();

    assert_eq!(result, Value::F32(3.2415927));
}

#[test]
fn test_float64_from_bits() {
    let transform = Transform {
        base: Base::Float64,
        scale: None,
        offset: Some(0.1),
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    let raw = crate::value::Value::U64(0x400921FB54442D18);
    let result = transform.apply(raw).unwrap();

    assert_eq!(result, Value::F64(3.241592653589793));
}

#[test]
fn test_floats_failure() {
    let transform = Transform {
        base: Base::Float32,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    let transform_64 = Transform {
        base: Base::Float64,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    assert!(transform.apply(crate::value::Value::I64(0)).is_err());
    assert!(transform_64.apply(crate::value::Value::I64(0)).is_err());
}

#[test]
fn test_int() {
    let mut transform = Transform {
        base: Base::Int,
        scale: Some(2.0),
        offset: Some(1.0),
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };
    assert_eq!(
        transform.apply(crate::value::Value::I64(10)).unwrap(),
        Value::F64(21.0)
    );

    transform.scale = Some(1.0);
    transform.offset = Some(-10.0);
    assert_eq!(
        transform.apply(crate::value::Value::I64(10)).unwrap(),
        Value::F64(0.0)
    );
    assert_eq!(
        transform.apply(crate::value::Value::U64(10)).unwrap(),
        Value::F64(0.0)
    );

    transform.scale = Some(1.0);
    transform.offset = Some(0.0);
    assert_eq!(
        transform.apply(crate::value::Value::U64(0)).unwrap(),
        Value::F64(0.0)
    );
}

#[test]
fn test_bytes() {
    let transform = Transform {
        base: Base::Bytes,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    let value = crate::value::Value::Array(vec![
        crate::value::Value::I64(10),
        crate::value::Value::I64(20),
        crate::value::Value::I64(30),
    ]);
    let result = transform.apply(value).unwrap();
    assert_eq!(result, Value::Bytes(vec![10, 20, 30]));
}

#[test]
fn test_bytes_failure() {
    let transform = Transform {
        base: Base::Bytes,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    let value = crate::value::Value::Array(vec![
        crate::value::Value::I64(10),
        crate::value::Value::I64(20),
        crate::value::Value::I64(300),
    ]);

    assert!(transform.apply(value).is_err());
}

#[test]
fn test_string() {
    let mut transform = Transform {
        base: Base::Bytes,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: Some(Encoding::Utf8),
        zero_terminated: None,
        trim: None,
    };

    let value = crate::value::Value::Array(vec![
        crate::value::Value::I64(String::from("H").as_bytes()[0] as i64),
        crate::value::Value::I64(String::from("e").as_bytes()[0] as i64),
        crate::value::Value::I64(String::from("l").as_bytes()[0] as i64),
        crate::value::Value::I64(String::from("l").as_bytes()[0] as i64),
        crate::value::Value::I64(String::from("o").as_bytes()[0] as i64),
        crate::value::Value::I64(String::from("\n").as_bytes()[0] as i64),
    ]);

    assert_eq!(
        transform.apply(value.clone()).unwrap(),
        Value::String("Hello\n".to_string())
    );

    transform.encoding = Some(Encoding::Ascii);
    transform.trim = Some(true);
    assert_eq!(
        transform.apply(value).unwrap(),
        Value::String("Hello".to_string())
    );
}

#[test]
fn test_string_ascii_failure() {
    let transform = Transform {
        base: Base::Bytes,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: Some(Encoding::Ascii),
        zero_terminated: None,
        trim: None,
    };

    let value = crate::value::Value::Array(
        String::from("Hello❤️\n")
            .as_bytes()
            .iter()
            .map(|b| crate::value::Value::I64(*b as i64))
            .collect(),
    );

    assert!(transform.apply(value).is_err());
}

#[test]
fn test_enum() {
    let transform = Transform {
        base: Base::Int,
        scale: None,
        offset: None,
        enum_map: Some(HashMap::from([
            (1, "one".to_string()),
            (2, "two".to_string()),
        ])),
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    assert_eq!(
        transform.apply(crate::value::Value::I64(1)).unwrap(),
        Value::String("one".to_string())
    );
    assert_eq!(
        transform.apply(crate::value::Value::I64(2)).unwrap(),
        Value::String("two".to_string())
    );
}

#[test]
fn test_array() {
    let transform = Transform {
        base: Base::Int,
        scale: Some(2.0),
        offset: Some(1.0),
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    let value = crate::value::Value::Array(vec![
        crate::value::Value::I64(10),
        crate::value::Value::I64(20),
        crate::value::Value::I64(30),
    ]);
    assert_eq!(
        transform.apply(value).unwrap(),
        Value::Array(vec![
            Value::F64(21.0),
            Value::F64(41.0),
            Value::F64(61.0)
        ])
    );
}

#[test]
fn test_byte_array() {
    let transform = Transform {
        base: Base::Bytes,
        scale: None,
        offset: None,
        enum_map: None,
        encoding: None,
        zero_terminated: None,
        trim: None,
    };

    assert_eq!(
        transform
            .apply(crate::value::Value::Array(
                String::from("Hello")
                    .as_bytes()
                    .iter()
                    .map(|b| crate::value::Value::I64(*b as i64))
                    .collect(),
            ))
            .unwrap(),
        Value::Bytes(String::from("Hello").as_bytes().to_vec())
    );
}
