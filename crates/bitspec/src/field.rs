//! Definition of logical fields used to build a [crate::Schema].

/// A single named field in a schema: either a scalar or an array of scalars.
#[derive(Debug, Clone)]
#[cfg(not(feature = "transform"))]
pub struct Field {
    /// Name used in the parsed result map.
    pub name: String,
    /// Whether this is a scalar or an array, and array parameters.
    pub kind: FieldKind,
    /// If true, the assembled value is interpreted as signed and sign-extended.
    pub signed: bool,
    /// How [crate::fragment::Fragment]s are concatenated (MSB-first or LSB-first).
    pub assemble: crate::assembly::Assemble,
    /// Bit ranges that make up this field (one or more, possibly non-contiguous).
    pub fragments: Vec<crate::fragment::Fragment>,
}

#[derive(Debug, Clone)]
#[cfg(feature = "transform")]
pub struct Field {
    /// Name used in the parsed result map.
    pub name: String,
    /// Whether this is a scalar or an array, and array parameters.
    pub kind: FieldKind,
    /// If true, the assembled value is interpreted as signed and sign-extended.
    pub signed: bool,
    /// How [crate::fragment::Fragment]s are concatenated (MSB-first or LSB-first).
    pub assemble: crate::assembly::Assemble,
    /// Bit ranges that make up this field (one or more, possibly non-contiguous).
    pub fragments: Vec<crate::fragment::Fragment>,
    pub transform: Option<crate::transform::Transform>,
}

#[cfg(all(feature = "serde", not(feature = "transform")))]
impl From<crate::serde::FieldDef> for Field {
    fn from(value: crate::serde::FieldDef) -> Self {
        Field {
            name: value.name,
            kind: value.kind.into(),
            signed: value.signed,
            assemble: value.assemble.into(),
            fragments: value.fragments.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(all(feature = "serde", feature = "transform"))]
impl From<crate::serde::FieldDef> for Field {
    fn from(value: crate::serde::FieldDef) -> Self {
        use crate::transform::Transform;

        Field {
            name: value.name,
            kind: value.kind.into(),
            signed: value.signed,
            assemble: value.assemble.into(),
            fragments: value.fragments.into_iter().map(Into::into).collect(),
            transform: value.transform.map(|def| Transform::try_from(def).unwrap()),
        }
    }
}

/// Distinguishes scalar fields from fixed-length array fields.
#[derive(Debug, Clone)]
pub enum FieldKind {
    /// Single value assembled from one or more fragments.
    Scalar,
    /// Repeated element with fixed count and stride.
    Array(ArraySpec),
}

#[cfg(feature = "serde")]
impl From<crate::serde::FieldKindDef> for FieldKind {
    fn from(value: crate::serde::FieldKindDef) -> Self {
        match value {
            crate::serde::FieldKindDef::Scalar => FieldKind::Scalar,
            crate::serde::FieldKindDef::Array {
                count,
                stride_bits,
                offset_bits,
            } => FieldKind::Array(ArraySpec {
                count,
                stride_bits,
                offset_bits,
            }),
        }
    }
}

/// Parameters for an array field: count, stride, and start offset in bits.
#[derive(Debug, Clone)]
pub struct ArraySpec {
    /// Number of elements.
    pub count: usize,
    /// Distance in bits between the start of consecutive elements.
    pub stride_bits: usize,
    /// Bit offset where the first element starts.
    pub offset_bits: usize,
}
