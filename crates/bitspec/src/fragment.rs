//! A contiguous bit range within a byte slice, with optional bit order.
//!
//! Used as building blocks for [crate::field::Field] definitions.

/// A contiguous range of bits: start offset (in bits) and length. Bit order is configurable.
#[derive(Debug, Clone, Copy)]
pub struct Fragment {
    /// Bit offset from the start of the data.
    pub offset_bits: usize,
    /// Number of bits in this fragment.
    pub len_bits: usize,
    pub bit_order: crate::assembly::BitOrder,
}

#[cfg(feature = "serde")]
impl From<crate::serde::FragmentDef> for Fragment {
    fn from(value: crate::serde::FragmentDef) -> Self {
        Fragment {
            offset_bits: value.offset_bits,
            len_bits: value.len_bits,
            bit_order: match value.bit_order {
                Some(bit_order) => bit_order.into(),
                None => Default::default(),
            },
        }
    }
}

impl Fragment {
    pub fn new(offset_bits: usize, len_bits: usize) -> Self {
        Fragment {
            offset_bits,
            len_bits,
            bit_order: Default::default(),
        }
    }

    pub fn new_with_bit_order(
        offset_bits: usize,
        len_bits: usize,
        bit_order: crate::assembly::BitOrder,
    ) -> Self {
        Fragment {
            offset_bits,
            len_bits,
            bit_order,
        }
    }
}

impl Default for Fragment {
    fn default() -> Self {
        Fragment {
            offset_bits: 0,
            len_bits: 0,
            bit_order: Default::default(),
        }
    }
}
