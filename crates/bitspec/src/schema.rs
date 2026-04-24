//! Schema: compiled set of fields used to parse byte slices into named values.

use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "transform")]
use crate::transform::TransformError;
use crate::{
    assembly::{ArrayCount, BitOrder, Value},
    bits,
    compiled::{CompiledField, CompiledFieldKind},
    errors::{CompileError, ReadError, WriteError},
    field::Field,
};

#[derive(Debug, Clone)]
pub struct WriteConfig {
    pub bit_order: BitOrder,
}

#[cfg(feature = "serde")]
impl From<crate::serde::WriteConfigDef> for WriteConfig {
    fn from(value: crate::serde::WriteConfigDef) -> Self {
        WriteConfig {
            bit_order: value.bit_order.into(),
        }
    }
}

impl Default for WriteConfig {
    fn default() -> Self {
        WriteConfig {
            bit_order: BitOrder::MsbFirst,
        }
    }
}

/// A compiled schema: list of [CompiledField]s and total bit length. Use [Schema::compile] to build from [Field]s, then [Schema::parse] to parse bytes.
#[derive(Debug, Clone)]
#[cfg(not(feature = "transform"))]
pub struct Schema {
    total_bits: usize,
    /// Compiled fields in definition order.
    pub fields: Vec<CompiledField>,
    pub write_config: Option<WriteConfig>,
}

#[derive(Debug, Clone)]
#[cfg(feature = "transform")]
pub struct Schema {
    total_bits: usize,
    /// Compiled fields in definition order.
    pub fields: Vec<CompiledField>,
    pub write_config: Option<WriteConfig>,
    transforms: HashMap<String, crate::transform::Transform>,
}

#[cfg(feature = "serde")]
impl TryFrom<crate::serde::SchemaDef> for Schema {
    type Error = CompileError;

    fn try_from(value: crate::serde::SchemaDef) -> Result<Self, Self::Error> {
        let fields: Vec<Field> = value.fields.into_iter().map(Into::into).collect();
        let write_config = value.write_config.map(Into::into);
        return Self::compile(&fields, write_config);
    }
}

impl Schema {
    /// Compiles a slice of [Field]s into a schema. Fails if any field is invalid.
    #[cfg(not(feature = "transform"))]
    pub fn compile(
        fields: &[Field],
        write_config: Option<WriteConfig>,
    ) -> Result<Self, CompileError> {
        let mut compiled_fields: Vec<CompiledField> = Vec::with_capacity(fields.len());
        let mut total_bits = 0;

        for field in fields {
            let compiled_field: CompiledField = field.try_into()?;

            match &compiled_field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    for frag in &scalar.fragments {
                        let end = frag.offset_bits + frag.len_bits;
                        total_bits = total_bits.max(end);
                    }
                }
                CompiledFieldKind::Array(array) => {
                    let ArrayCount::Fixed(count) = array.count;

                    let end = array.offset_bits
                        + array.element.total_bits
                        + array.stride_bits * (count - 1);

                    total_bits = total_bits.max(end);
                }
            }

            compiled_fields.push(compiled_field);
        }

        Ok(Self {
            fields: compiled_fields,
            total_bits,
            write_config,
        })
    }

    #[cfg(feature = "transform")]
    pub fn compile(
        fields: &[Field],
        write_config: Option<WriteConfig>,
    ) -> Result<Self, CompileError> {
        let mut compiled_fields: Vec<CompiledField> = Vec::with_capacity(fields.len());
        let mut total_bits = 0;
        let mut transforms = HashMap::<String, crate::transform::Transform>::new();

        for field in fields {
            let compiled_field: CompiledField = field.try_into()?;

            match &compiled_field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    for frag in &scalar.fragments {
                        let end = frag.offset_bits + frag.len_bits;
                        total_bits = total_bits.max(end);
                    }
                }
                CompiledFieldKind::Array(array) => {
                    let ArrayCount::Fixed(count) = array.count;

                    let end = array.offset_bits
                        + array.element.total_bits
                        + array.stride_bits * (count - 1);

                    total_bits = total_bits.max(end);
                }
            }

            if let Some(transform) = &field.transform {
                transforms.insert(field.name.clone(), transform.clone());
            }

            compiled_fields.push(compiled_field);
        }

        Ok(Self {
            fields: compiled_fields,
            total_bits,
            write_config,
            transforms,
        })
    }

    #[cfg(feature = "transform")]
    pub fn apply_transforms(
        &self,
        obj: BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, crate::transform::Value>, TransformError> {
        let mut map: BTreeMap<String, crate::transform::Value> = BTreeMap::new();

        for (name, value) in obj {
            let transform = self.transforms.get(&name);
            match transform {
                Some(transform) => {
                    let value = transform.apply(value)?;
                    map.insert(name, value);
                }
                None => {
                    map.insert(
                        name.clone(),
                        crate::transform::value_to_transform_value(value),
                    );
                }
            }
        }

        Ok(map)
    }

    /// Parses `data` according to this schema. Returns a map of field names to [Value]s. Fails if `data` is too short.
    pub fn parse(&self, data: &[u8]) -> Result<BTreeMap<String, Value>, ReadError> {
        if data.len() * 8 < self.total_bits {
            return Err(ReadError::PacketTooShort);
        }

        let mut map: BTreeMap<String, Value> = BTreeMap::new();

        for field in &self.fields {
            match &field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    map.insert(field.name.clone(), scalar.assemble(data)?);
                }
                CompiledFieldKind::Array(array) => {
                    map.insert(field.name.clone(), array.assemble(data)?);
                }
            }
        }

        Ok(map)
    }

    pub fn serialize(&self, obj: &HashMap<String, Value>) -> Result<Vec<u8>, WriteError> {
        let mut bits: Vec<u8> = Vec::new();

        for field in &self.fields {
            let value = obj
                .get(&field.name)
                .ok_or_else(|| WriteError::MissingField(field.name.clone()))?;

            match &field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    scalar.disassemble(value, &mut bits)?;
                }
                CompiledFieldKind::Array(array) => {
                    array.disassemble(value, &mut bits)?;
                }
            }
        }

        Ok(bits::bits_to_bytes(
            &bits,
            match &self.write_config {
                Some(config) => config.bit_order,
                None => BitOrder::MsbFirst,
            },
        ))
    }
}

#[cfg(all(test, not(feature = "transform")))]
mod tests {
    use crate::{
        assembly::{Assemble, BitOrder},
        field::{ArraySpec, Field, FieldKind},
        fragment::Fragment,
    };

    use super::*;

    #[test]
    fn test_get_all_empty() {
        let schema = Schema::compile(&vec![], None).unwrap();
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = schema.parse(&data);
        assert_eq!(result, Ok(BTreeMap::new()));
    }

    #[test]
    fn test_get_all_one_field() {
        let field = Field {
            name: "test".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 1,
                ..Default::default()
            }],
        };
        let schema = Schema::compile(&vec![field], None).unwrap();
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([("test".to_string(), Value::U64(0))]))
        );
    }

    #[test]
    fn test_get_multiple_fields() {
        let field1 = Field {
            name: "test1".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };
        let field2 = Field {
            name: "test2".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 8,
                len_bits: 16,
                ..Default::default()
            }],
        };
        let schema = Schema::compile(&vec![field1, field2], None).unwrap();
        let data = vec![0x01, 0x00, 0x01, 0x04];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([
                ("test1".to_string(), Value::U64(1)),
                ("test2".to_string(), Value::U64(1))
            ]))
        );
    }

    #[test]
    fn test_get_all_array() {
        let field = Field {
            name: "test".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 4,
                stride_bits: 8,
                offset_bits: 0,
            }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&vec![field], None).unwrap();
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([(
                "test".to_string(),
                Value::Array(vec![
                    Value::U64(1),
                    Value::U64(2),
                    Value::U64(3),
                    Value::U64(4)
                ])
            )]))
        );
    }

    #[test]
    fn test_get_all_array_with_stride() {
        let id_field = Field {
            name: "id".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 16,
                ..Default::default()
            }],
        };

        let temperature_field = Field {
            name: "temperature".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 16,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let values_field = Field {
            name: "values".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 5,
                stride_bits: 8,
                offset_bits: 24,
            }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema =
            Schema::compile(&vec![id_field, temperature_field, values_field], None).unwrap();

        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([
                ("id".to_string(), Value::U64(258)),
                ("temperature".to_string(), Value::U64(3)),
                (
                    "values".to_string(),
                    Value::Array(vec![
                        Value::U64(4),
                        Value::U64(5),
                        Value::U64(6),
                        Value::U64(7),
                        Value::U64(8)
                    ])
                )
            ]))
        );
    }

    #[test]
    fn test_serialize_single_scalar() {
        let field = Field {
            name: "a".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&[field], None).unwrap();

        let obj = HashMap::from([("a".to_string(), Value::U64(0xAB))]);

        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0xAB]);
    }

    #[test]
    fn test_serialize_multiple_scalars_linear() {
        let a = Field {
            name: "a".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 4,
                ..Default::default()
            }],
        };

        let b = Field {
            name: "b".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 4,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&[a, b], None).unwrap();

        let obj = HashMap::from([
            ("a".to_string(), Value::U64(0b1010)),
            ("b".to_string(), Value::U64(0b0101)),
        ]);

        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0b1010_0101]);
    }

    #[test]
    fn test_serialize_non_sequential_fragments() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![
                Fragment {
                    offset_bits: 4,
                    len_bits: 2,
                    ..Default::default()
                },
                Fragment {
                    offset_bits: 0,
                    len_bits: 2,
                    ..Default::default()
                },
            ],
        };

        let schema = Schema::compile(&[field], None).unwrap();

        // value = 0b1101
        let obj = HashMap::from([("x".to_string(), Value::U64(0b1101))]);

        // take bits [4..6] then [0..2] → 11 01
        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0b1101_0000]);
    }

    #[test]
    fn test_serialize_array_dense() {
        let field = Field {
            name: "arr".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 3,
                stride_bits: 8,
                offset_bits: 0, // irrelevant for serialize
            }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&[field], None).unwrap();

        let obj = HashMap::from([(
            "arr".to_string(),
            Value::Array(vec![Value::U64(1), Value::U64(2), Value::U64(3)]),
        )]);

        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);
    }

    #[test]
    fn test_serialize_parse_roundtrip_dense() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&[field], None).unwrap();

        let obj = HashMap::from([("x".to_string(), Value::U64(42))]);

        let bytes = schema.serialize(&obj).unwrap();
        let parsed = schema.parse(&bytes).unwrap();

        assert_eq!(parsed.get("x"), Some(&Value::U64(42)));
    }
}
