//! Compiled (executable) representation of fields and fragments for fast parsing.

use crate::{
    assembly::{ArrayCount, Assemble, BitOrder, Value},
    bits::{self, reverse_bits_n, sign_extend},
    errors::{CompileError, ReadError, WriteError},
    field::FieldKind,
};

/// Compiled field: either a scalar or an array.
#[derive(Debug, Clone)]
pub enum CompiledFieldKind {
    Scalar(CompiledScalar),
    Array(CompiledArray),
}

/// A field after compilation: name and either scalar or array layout.
#[derive(Debug, Clone)]
pub struct CompiledField {
    pub name: String,
    pub kind: CompiledFieldKind,
}

impl TryFrom<&crate::field::Field> for CompiledField {
    type Error = CompileError;

    fn try_from(value: &crate::field::Field) -> Result<Self, Self::Error> {
        let compiled_scalar: CompiledScalar = value.try_into()?;
        match &value.kind {
            FieldKind::Scalar => Ok(CompiledField {
                name: value.name.clone(),
                kind: CompiledFieldKind::Scalar(compiled_scalar),
            }),
            FieldKind::Array(spec) => {
                if spec.stride_bits < compiled_scalar.total_bits {
                    return Err(CompileError::InvalidArrayStride);
                } else if spec.count == 0 {
                    return Err(CompileError::InvalidArrayCount);
                } else if value.fragments.len() == 0 {
                    return Err(CompileError::EmptyArrayElement);
                }

                Ok(CompiledField {
                    name: value.name.clone(),
                    kind: CompiledFieldKind::Array(CompiledArray {
                        element: compiled_scalar,
                        count: ArrayCount::Fixed(spec.count),
                        stride_bits: spec.stride_bits,
                        offset_bits: spec.offset_bits,
                    }),
                })
            }
        }
    }
}

/// Compiled array: element layout, count, stride, and start offset.
#[derive(Debug, Clone)]
pub struct CompiledArray {
    pub element: CompiledScalar,
    /// Number of elements.
    pub count: ArrayCount,
    /// Bits between the start of consecutive elements.
    /// Works like window size to read fragments from.
    /// Fragments will have offsets relative to the window start.
    pub stride_bits: usize,
    /// Bit offset where the first element starts.
    /// Global offset describes where first window should be read from.
    pub offset_bits: usize,
}

impl CompiledArray {
    /// Assembles the array from `data` into a [Value::Array].
    pub fn assemble(&self, data: &[u8]) -> Result<Value, ReadError> {
        let count = match self.count {
            ArrayCount::Fixed(count) => count,
        };

        let mut values = Vec::<Value>::with_capacity(count);
        for i in 0..count {
            let offset = self.offset_bits + i * self.stride_bits;
            values.push(self.element.assemble_at(data, offset)?);
        }

        Ok(Value::Array(values))
    }
}

impl<'a> CompiledArray {
    pub fn disassemble(
        &self,
        value: &'a Value,
        buf: &'a mut Vec<u8>,
    ) -> Result<&'a Vec<u8>, WriteError> {
        match value {
            Value::Array(values) => {
                for value in values {
                    self.element.disassemble(value, buf)?;
                }
            }
            _ => return Err(WriteError::InvalidValue),
        }

        Ok(buf)
    }
}

/// Compiled scalar: total size, signedness, and list of fragments with shifts.
#[derive(Debug, Clone)]
pub struct CompiledScalar {
    pub signed: bool,
    /// Total bit width (sum of fragment lengths, 1–64).
    pub total_bits: usize,
    /// Fragments with precomputed shift for assembly.
    pub fragments: Vec<CompiledFragment>,
}

impl TryFrom<&crate::field::Field> for CompiledScalar {
    type Error = CompileError;

    fn try_from(value: &crate::field::Field) -> Result<Self, Self::Error> {
        let total_bits: usize = value
            .fragments
            .iter()
            .fold(0, |acc, fragment| acc + fragment.len_bits);

        if total_bits == 0 || total_bits > 64 {
            return Err(CompileError::InvalidFieldSize);
        }

        let mut fragments = Vec::with_capacity(value.fragments.len());

        match value.assemble {
            Assemble::Concat(BitOrder::MsbFirst) => {
                let mut remaining = total_bits;
                for fragment in &value.fragments {
                    remaining -= fragment.len_bits;

                    let mut compiled_fragment = CompiledFragment::try_from(fragment)?;
                    compiled_fragment.shift = remaining;

                    fragments.push(compiled_fragment);
                }
            }
            Assemble::Concat(BitOrder::LsbFirst) => {
                let mut shift = 0;
                for fragment in &value.fragments {
                    let mut compiled_fragment = CompiledFragment::try_from(fragment)?;
                    compiled_fragment.shift = shift;

                    fragments.push(compiled_fragment);

                    shift += fragment.len_bits;
                }
            }
        }

        Ok(CompiledScalar {
            signed: value.signed,
            total_bits,
            fragments,
        })
    }
}

impl CompiledScalar {
    /// Assembles the scalar from `data` starting at bit 0.
    pub fn assemble(&self, data: &[u8]) -> Result<Value, ReadError> {
        self.assemble_at(data, 0)
    }

    /// Assembles the scalar from `data` starting at `offset_bits`.
    pub fn assemble_at(&self, data: &[u8], offset_bits: usize) -> Result<Value, ReadError> {
        let mut value = 0u64;

        for fragment in &self.fragments {
            let mut part =
                bits::read_bits_at(data, fragment.offset_bits + offset_bits, fragment.len_bits)?;

            if fragment.bit_order == BitOrder::LsbFirst {
                part = reverse_bits_n(part, fragment.len_bits);
            }

            value |= part << fragment.shift;
        }

        if self.signed {
            Ok(Value::I64(sign_extend(value, self.total_bits)))
        } else {
            Ok(Value::U64(value))
        }
    }
}

impl<'a> CompiledScalar {
    pub fn disassemble(
        &self,
        value: &'a Value,
        buf: &'a mut Vec<u8>,
    ) -> Result<&'a Vec<u8>, WriteError> {
        let value = match value {
            Value::I64(v) => *v as u64,
            Value::U64(v) => *v,
            Value::Array(_) => return Err(WriteError::InvalidValue),
        };

        for fragment in &self.fragments {
            let mut part = value >> fragment.shift;

            if fragment.bit_order == BitOrder::LsbFirst {
                part = reverse_bits_n(part, fragment.len_bits);
            }

            for i in 0..fragment.len_bits {
                let bit = (part >> (fragment.len_bits - 1 - i)) & 1;
                buf.push(bit as u8);
            }
        }

        Ok(buf)
    }
}

/// A fragment with precomputed shift for merging into the final scalar value.
#[derive(Debug, Clone)]
pub struct CompiledFragment {
    pub offset_bits: usize,
    pub len_bits: usize,
    pub bit_order: crate::assembly::BitOrder,
    /// Shift to apply when OR-ing into the accumulator.
    pub shift: usize,
}

impl TryFrom<&crate::fragment::Fragment> for CompiledFragment {
    type Error = CompileError;

    fn try_from(fragment: &crate::fragment::Fragment) -> Result<Self, Self::Error> {
        if fragment.len_bits == 0 {
            return Err(CompileError::InvalidFragment);
        }

        Ok(CompiledFragment {
            offset_bits: fragment.offset_bits,
            len_bits: fragment.len_bits,
            bit_order: fragment.bit_order,
            shift: 0,
        })
    }
}

#[cfg(all(test, not(feature = "transform")))]
mod tests {
    use crate::{compiled::CompiledScalar, field::Field, fragment::Fragment};

    use super::*;

    #[test]
    fn test_assemble_field() {
        let data = [0b11_000001, 0b10000_101];

        let id_field = Field {
            name: "id".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 2,
                ..Default::default()
            }],
        };

        let value_field = Field {
            name: "value".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 2,
                len_bits: 11,
                ..Default::default()
            }],
        };

        let crc_field = Field {
            name: "crc".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 13,
                len_bits: 3,
                ..Default::default()
            }],
        };

        let compiled_id_field = CompiledScalar::try_from(&id_field).unwrap();
        let compiled_value_field = CompiledScalar::try_from(&value_field).unwrap();
        let compiled_crc_field = CompiledScalar::try_from(&crc_field).unwrap();

        let id = compiled_id_field.assemble(&data).unwrap();
        let value = compiled_value_field.assemble(&data).unwrap();
        let crc = compiled_crc_field.assemble(&data).unwrap();

        assert_eq!(id, Value::U64(3));
        assert_eq!(value, Value::U64(48));
        assert_eq!(crc, Value::U64(5));
    }

    #[test]
    fn test_non_consecutive_fragments() {
        let data: [u8; 4] = [0b00000001, 0b00000010, 0b00000100, 0b00001000];

        let first_value_field = Field {
            name: "first_value".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![
                Fragment {
                    offset_bits: 0,
                    len_bits: 8,
                    ..Default::default()
                },
                Fragment {
                    offset_bits: 16,
                    len_bits: 8,
                    ..Default::default()
                },
            ],
        };

        let second_value_field = Field {
            name: "second_value".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![
                Fragment {
                    offset_bits: 8,
                    len_bits: 8,
                    ..Default::default()
                },
                Fragment {
                    offset_bits: 24,
                    len_bits: 8,
                    ..Default::default()
                },
            ],
        };

        let compiled_first_value_field = CompiledScalar::try_from(&first_value_field).unwrap();
        let compiled_second_value_field = CompiledScalar::try_from(&second_value_field).unwrap();

        let first_value = compiled_first_value_field.assemble(&data).unwrap();
        assert_eq!(first_value, Value::U64(0b00000001_00000100));

        let second_value = compiled_second_value_field.assemble(&data).unwrap();
        assert_eq!(second_value, Value::U64(0b00000010_00001000));
    }

    #[test]
    fn test_assemble_concat_lsb() {
        let data: [u8; 2] = [0b00001001, 0b00001100];

        let value_field = Field {
            name: "value".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::LsbFirst),
            fragments: vec![
                Fragment {
                    offset_bits: 4,
                    len_bits: 4,
                    ..Default::default()
                },
                Fragment {
                    offset_bits: 12,
                    len_bits: 4,
                    ..Default::default()
                },
            ],
        };

        let compiled_value_field = CompiledScalar::try_from(&value_field).unwrap();
        let value = compiled_value_field.assemble(&data).unwrap();
        assert_eq!(value, Value::U64(0b11001001));
    }

    #[test]
    fn compile_scalar_concat_msb_shifts() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![
                Fragment {
                    offset_bits: 0,
                    len_bits: 3,
                    ..Default::default()
                },
                Fragment {
                    offset_bits: 5,
                    len_bits: 5,
                    ..Default::default()
                },
            ],
        };

        let compiled = CompiledScalar::try_from(&field).unwrap();

        // total_bits = 8
        assert_eq!(compiled.fragments[0].shift, 5);
        assert_eq!(compiled.fragments[1].shift, 0);
    }

    #[test]
    fn compile_scalar_concat_lsb_shifts() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::LsbFirst),
            fragments: vec![
                Fragment {
                    offset_bits: 0,
                    len_bits: 3,
                    ..Default::default()
                },
                Fragment {
                    offset_bits: 5,
                    len_bits: 5,
                    ..Default::default()
                },
            ],
        };

        let compiled = CompiledScalar::try_from(&field).unwrap();

        assert_eq!(compiled.fragments[0].shift, 0);
        assert_eq!(compiled.fragments[1].shift, 3);
    }
}
