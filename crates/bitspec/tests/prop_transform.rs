//! Property: identity transform (Base::Int, no scale/offset, no enum, no encoding)
//! preserves sign and value.

#![cfg(feature = "transform")]

use bitspec::{
    transform::{Base, Transform},
    value::Value,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn identity_passes_u64(x in any::<u64>()) {
        let t = Transform::new(Base::Int);
        let out = t.apply(Value::U64(x)).expect("identity always applies");
        prop_assert_eq!(out, Value::U64(x));
    }

    #[test]
    fn identity_passes_i64(x in any::<i64>()) {
        let t = Transform::new(Base::Int);
        let out = t.apply(Value::I64(x)).expect("identity always applies");
        prop_assert_eq!(out, Value::I64(x));
    }
}
