//! Assembly options for how multi-fragment fields are combined and how bits are ordered.

/// How multiple [crate::fragment::Fragment]s are concatenated to form a single value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assemble {
    Concat(BitOrder),
}

#[cfg(feature = "serde")]
impl From<crate::serde::AssembleDef> for Assemble {
    fn from(value: crate::serde::AssembleDef) -> Self {
        match value {
            crate::serde::AssembleDef::ConcatMsb => Assemble::Concat(BitOrder::MsbFirst),
            crate::serde::AssembleDef::ConcatLsb => Assemble::Concat(BitOrder::LsbFirst),
        }
    }
}

/// Bit order when reading a single fragment from the byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    MsbFirst,
    LsbFirst,
}

#[cfg(feature = "serde")]
impl From<crate::serde::BitOrderDef> for BitOrder {
    fn from(value: crate::serde::BitOrderDef) -> Self {
        match value {
            crate::serde::BitOrderDef::MsbFirst => BitOrder::MsbFirst,
            crate::serde::BitOrderDef::LsbFirst => BitOrder::LsbFirst,
        }
    }
}

impl Default for BitOrder {
    fn default() -> Self {
        BitOrder::MsbFirst
    }
}

/// A value produced when assembling a field from raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    I64(i64),
    U64(u64),
    Array(Vec<Value>),
}

/// Number of elements in an array field.
#[derive(Debug, Clone)]
pub enum ArrayCount {
    Fixed(usize),
}
