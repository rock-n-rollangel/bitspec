//! Error types for schema compilation and bit reading.

/// Errors produced when compiling a [crate::field::Field] into a [crate::compiled::CompiledField].
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

/// Errors produced when reading bits from a byte slice (e.g. during [crate::Schema::parse]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Requested bit range is beyond the end of the data.
    OutOfBounds,
    /// More than 64 bits were requested in a single read.
    TooManyBitsRead,
    /// Input data is shorter than the schema’s total bit length.
    PacketTooShort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// Buffer is too short to write the value.
    OutOfBounds,
    InvalidValue,
    MissingField(String),
}
