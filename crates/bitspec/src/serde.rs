//! JSON‑deserializable schema description.
//!
//! These types describe the *shape* of the binary data to be parsed. They are
//! intended to be constructed from JSON (for example a schema file shipped
//! with your application) and then compiled into core `bitspec` types.
//!
//! The same shapes are expected when you call `Schema::compile` with a JSON string.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How individual fragments of bits are assembled into a numeric value.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AssembleDef {
    /// Concatenate fragments most‑significant‑bit first.
    ConcatMsb,
    /// Concatenate fragments least‑significant‑bit first.
    ConcatLsb,
}

/// Bit order to use when reading a fragment.
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub enum BitOrderDef {
    #[default]
    /// Most‑significant bit first within the fragment.
    MsbFirst,
    /// Least‑significant bit first within the fragment.
    LsbFirst,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WriteConfigDef {
    #[serde(default)]
    pub bit_order: BitOrderDef,
}

/// Top‑level schema definition consisting of a list of fields.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SchemaDef {
    /// All fields that should be parsed from the payload.
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub write_config: Option<WriteConfigDef>,
}

/// Description of a single parsed field.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FieldDef {
    /// Human‑readable field name; becomes the key in the output map.
    pub name: String,
    /// Whether this is a scalar or fixed‑size array field.
    pub kind: FieldKindDef,
    /// Whether the assembled value should be interpreted as signed.
    pub signed: bool,
    /// Strategy used to assemble fragments into a single value.
    pub assemble: AssembleDef,
    /// Bit fragments that make up this field.
    pub fragments: Vec<FragmentDef>,

    /// Optional post‑processing transform applied after parsing the raw value.
    #[serde(default)]
    pub transform: Option<TransformDef>,
}

/// Kind of field in the schema.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum FieldKindDef {
    /// Single scalar value.
    Scalar,
    /// Fixed‑size array of values laid out with a constant stride.
    Array {
        /// Number of elements in the array.
        count: usize,
        /// Distance in bits between consecutive elements.
        stride_bits: usize,
        /// Bit offset of the first element from the start of the payload.
        offset_bits: usize,
    },
}

/// Bit‑level fragment that contributes to a field value.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FragmentDef {
    /// Offset of the first bit of this fragment from the start of the payload.
    pub offset_bits: usize,
    /// Length of the fragment in bits.
    pub len_bits: usize,
    /// Optional bit order inside the fragment; defaults to MSB‑first.
    #[serde(default)]
    pub bit_order: Option<BitOrderDef>,
}


/// Base type of the value before any transform is applied.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum BaseDef {
    /// Signed/unsigned integer value.
    Int,
    /// 32‑bit floating‑point value.
    Float32,
    /// 64‑bit floating‑point value.
    Float64,
    /// Raw bytes (often used together with [`EncodingDef`]).
    Bytes,
}

/// Text encoding to use when interpreting byte values as strings.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum EncodingDef {
    /// UTF‑8 encoded string.
    Utf8,
    /// ASCII encoded string.
    Ascii,
}

/// Complete description of how to transform a parsed raw value.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransformDef {
    /// Base representation of the raw value.
    pub base: BaseDef,
    /// Optional multiplicative scale applied to numeric values.
    pub scale: Option<f64>,
    /// Optional additive offset applied after scaling.
    pub offset: Option<f64>,

    /// Optional text encoding when interpreting bytes as strings.
    pub encoding: Option<EncodingDef>,
    /// Whether string values should stop at the first zero byte.
    pub zero_terminated: Option<bool>,
    /// Whether leading/trailing whitespace should be trimmed.
    pub trim: Option<bool>,

    /// Optional mapping from integer codes to human‑readable labels.
    pub enum_map: Option<HashMap<i64, String>>,
}
