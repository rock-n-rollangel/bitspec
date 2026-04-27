//! Property: `read_bits_at` matches the simple reference `read_bits_at_slow`.
//!
//! The slow path is the original per-bit implementation, retained as the
//! ground truth when validating the byte-coalesced fast path.

use bitspec::bits::{read_bits_at, read_bits_at_slow};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fast_matches_slow(
        bytes in prop::collection::vec(any::<u8>(), 1..64),
        bit_pos in 0usize..256,
        n in 1usize..=64,
    ) {
        let max_end = bytes.len() * 8;
        prop_assume!(bit_pos + n <= max_end);

        let fast = read_bits_at(&bytes, bit_pos, n);
        let slow = read_bits_at_slow(&bytes, bit_pos, n);
        prop_assert_eq!(fast, slow);
    }
}
