use bitspec::{
    assembly::{Assemble, BitOrder},
    field::{Field, FieldKind},
    fragment::Fragment,
    schema::Schema,
};
use criterion::{Criterion, criterion_group, criterion_main};

fn gen_field(iter: usize) -> Field {
    #[cfg(feature = "transform")]
    let field = Field {
        name: format!("f{}", iter),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment {
            offset_bits: iter * 16,
            len_bits: 16,
            ..Default::default()
        }],
        transform: None,
    };

    #[cfg(not(feature = "transform"))]
    let field = Field {
        name: format!("f{}", iter),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment {
            offset_bits: iter * 16,
            len_bits: 16,
            ..Default::default()
        }],
    };

    field
}

fn gen_schema(field_count: usize) -> Schema {
    let mut fields = Vec::with_capacity(field_count);

    for i in 0..field_count {
        fields.push(gen_field(i));
    }

    Schema::compile(&fields, None).unwrap()
}

fn gen_packet(total_bits: usize) -> Vec<u8> {
    let total_bytes = (total_bits + 7) / 8;
    let mut data = Vec::with_capacity(total_bytes);

    // Deterministic but non-trivial pattern
    for i in 0..total_bytes {
        data.push((i * 31 % 256) as u8);
    }

    data
}

fn bench_schema_parse(c: &mut Criterion) {
    for &field_count in &[1usize, 10, 50, 100] {
        let schema = gen_schema(field_count);
        let packet = gen_packet(field_count * 16);

        c.bench_function(&format!("parse_{}_fields", field_count), |b| {
            b.iter(|| {
                let _ = schema.parse(&packet).unwrap();
            })
        });
    }
}

criterion_group!(benches, bench_schema_parse);
criterion_main!(benches);
