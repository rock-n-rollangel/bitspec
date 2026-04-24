//! Error types for schema compilation and bit reading/writing.

use std::fmt;

/// Errors produced when compiling a [`crate::field::Field`] into a [`crate::compiled::CompiledField`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Array stride is smaller than the element size.
    InvalidArrayStride,
    /// Array count is zero.
    InvalidArrayCount,
    /// Scalar field total size is 0 or greater than 64 bits.
    InvalidFieldSize,
    /// Fragment has zero length or is otherwise invalid.
    InvalidFragment,
    /// Field kind is not supported.
    InvalidFieldKind,
    /// Array element has no fragments.
    EmptyArrayElement,
    /// Field name is invalid (e.g. empty or duplicate).
    InvalidFieldName,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArrayStride => write!(f, "array stride is smaller than element size"),
            Self::InvalidArrayCount => write!(f, "array count is zero"),
            Self::InvalidFieldSize => write!(f, "field total size must be 1..=64 bits"),
            Self::InvalidFragment => write!(f, "fragment is invalid (zero length or malformed)"),
            Self::InvalidFieldKind => write!(f, "unsupported field kind"),
            Self::EmptyArrayElement => write!(f, "array element has no fragments"),
            Self::InvalidFieldName => write!(f, "field name is empty or duplicated"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Errors produced when reading bits from a byte slice (e.g. during [`crate::schema::Schema::parse`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Requested bit range is beyond the end of the data.
    OutOfBounds,
    /// More than 64 bits were requested in a single read.
    TooManyBitsRead,
    /// Input data is shorter than the schema's total bit length.
    PacketTooShort,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds => write!(f, "bit range is beyond the end of the data"),
            Self::TooManyBitsRead => write!(f, "requested more than 64 bits in one read"),
            Self::PacketTooShort => write!(f, "input data is shorter than the schema's total bit length"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Errors produced when writing values back to bytes (e.g. during [`crate::schema::Schema::serialize`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// Buffer is too short to write the value, or requested width exceeds 64 bits.
    OutOfBounds,
    /// Value cannot be written to this field (e.g. array length mismatch, or Array variant for a scalar field).
    InvalidValue,
    /// Required field missing from the input object.
    MissingField(String),
    /// The provided value variant (e.g. F32/F64/Bytes/String) is not supported for serialization.
    UnsupportedValue { field: String, variant: &'static str },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds => write!(f, "buffer is too short to write the value"),
            Self::InvalidValue => write!(f, "value cannot be written to this field"),
            Self::MissingField(name) => write!(f, "missing field '{name}' in object"),
            Self::UnsupportedValue { field, variant } => write!(
                f,
                "field '{field}' received Value::{variant}; serialize accepts only U64, I64, and Array"
            ),
        }
    }
}

impl std::error::Error for WriteError {}
