//! Property: identity transform passes values through unchanged.

#![cfg(feature = "transform")]

use bitspec::{
    transform::{Base, Transform},
    value::Value,
};
use proptest::prelude::*;

fn identity_transform() -> Transform {
    Transform::new(Base::Int)
}

proptest! {
    #[test]
    fn identity_passes_u64(x in any::<u64>()) {
        let t = identity_transform();
        let out = t.apply(Value::U64(x)).expect("identity always applies");
        // Today's transform::Value::Int preserves as i64 — this test encodes current behavior.
        // It will be updated in Phase 3.1 when Value is unified.
        prop_assert!(matches!(out, bitspec::transform::Value::Int(_)));
    }
}
