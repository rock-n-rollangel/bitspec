//! Low-level bit read and manipulation utilities for byte slices.
//!
//! Bits are addressed in MSB-first order: bit 0 is the high bit of the first byte.

use crate::{assembly::BitOrder, errors::ReadError};

/// Reads a single bit at `bit_pos` (0 = MSB of first byte). Returns 0 or 1.
pub fn read_bit_at(data: &[u8], bit_pos: usize) -> Result<u8, ReadError> {
    if bit_pos >= data.len() * 8 {
        return Err(ReadError::OutOfBounds);
    }

    let byte_index = bit_pos / 8;
    let bit_index = bit_pos % 8;

    Ok((data[byte_index] >> (7 - bit_index)) & 1)
}

/// Reference implementation of `read_bits_at` using a per-bit loop.
///
/// Slower than `read_bits_at` but kept as the ground-truth oracle for
/// property tests (see `tests/prop_read_bits.rs`).
pub fn read_bits_at_slow(data: &[u8], bit_pos: usize, n: usize) -> Result<u64, ReadError> {
    if n > 64 {
        return Err(ReadError::TooManyBitsRead);
    }

    if bit_pos
        .checked_add(n)
        .map_or(true, |end| end > data.len() * 8)
    {
        return Err(ReadError::OutOfBounds);
    }

    let mut value = 0u64;
    let mut pos = bit_pos;

    for _ in 0..n {
        let bit = read_bit_at(&data, pos)? as u64;
        value = (value << 1) | bit;
        pos += 1;
    }

    Ok(value)
}

/// Reads `n` bits starting at `bit_pos` as an unsigned value (max 64 bits).
///
/// Uses byte-coalesced accumulation: at most 9 byte reads for `n <= 64`.
pub fn read_bits_at(data: &[u8], bit_pos: usize, n: usize) -> Result<u64, ReadError> {
    if n > 64 {
        return Err(ReadError::TooManyBitsRead);
    }
    let end = bit_pos
        .checked_add(n)
        .ok_or(ReadError::OutOfBounds)?;
    if end > data.len() * 8 {
        return Err(ReadError::OutOfBounds);
    }
    if n == 0 {
        return Ok(0);
    }

    let byte_start = bit_pos / 8;
    let byte_end = (end + 7) / 8;
    let bit_offset = bit_pos % 8;

    let mut acc: u128 = 0;
    for i in byte_start..byte_end {
        acc = (acc << 8) | data[i] as u128;
    }

    let trailing = (byte_end - byte_start) * 8 - (bit_offset + n);
    let mask = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
    Ok(((acc >> trailing) as u64) & mask)
}

/// Writes the low `n` bits of `value` into `data` starting at bit position
/// `bit_pos`, MSB-first. Bits outside the write range are left unchanged.
///
/// Returns `Err(WriteError::OutOfBounds)` if the write would exceed `data`,
/// or if `n > 64`.
pub fn write_bits_at(
    data: &mut [u8],
    bit_pos: usize,
    n: usize,
    value: u64,
) -> Result<(), crate::errors::WriteError> {
    if n > 64 {
        return Err(crate::errors::WriteError::OutOfBounds);
    }
    let end = bit_pos
        .checked_add(n)
        .ok_or(crate::errors::WriteError::OutOfBounds)?;
    if end > data.len() * 8 {
        return Err(crate::errors::WriteError::OutOfBounds);
    }

    for i in 0..n {
        let bit = ((value >> (n - 1 - i)) & 1) as u8;
        let pos = bit_pos + i;
        let byte_index = pos / 8;
        let bit_index_in_byte = 7 - (pos % 8);
        if bit == 1 {
            data[byte_index] |= 1 << bit_index_in_byte;
        } else {
            data[byte_index] &= !(1 << bit_index_in_byte);
        }
    }
    Ok(())
}

/// Sign-extends the low `bits` of `value` to a full `i64`.
pub fn sign_extend(value: u64, bits: usize) -> i64 {
    let shift = 64 - bits;
    ((value << shift) as i64) >> shift
}

/// Reverses the low `n` bits of `x` (LSB becomes MSB of the result).
pub fn reverse_bits_n(mut x: u64, n: usize) -> u64 {
    let mut r = 0u64;
    for _ in 0..n {
        r = (r << 1) | (x & 1);
        x >>= 1;
    }

    r
}

/// Reverses the bit order within each byte of `data`. MSB becomes LSB and vice-versa.
pub fn reverse_bits_in_bytes(data: &mut [u8]) {
    for byte in data.iter_mut() {
        *byte = byte.reverse_bits();
    }
}

/// Converts a slice of bits to a byte vector.
pub fn bits_to_bytes(bits: &[u8], bit_order: BitOrder) -> Vec<u8> {
    let n_bytes = (bits.len() + 7) / 8;
    let mut out = vec![0u8; n_bytes];

    for (i, &bit) in bits.iter().enumerate() {
        let byte_index = i / 8;
        let bit_in_byte = match bit_order {
            BitOrder::MsbFirst => 7 - (i % 8),
            BitOrder::LsbFirst => i % 8,
        };
        out[byte_index] |= bit << bit_in_byte;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bit_at() {
        let data = [0b11111111];
        assert_eq!(read_bit_at(&data, 0).unwrap(), 1);
    }

    #[test]
    fn test_read_bits_at() {
        let data = [0b11111111];
        assert_eq!(read_bits_at(&data, 0, 8).unwrap(), 0b11111111);
    }

    #[test]
    fn test_read_bits_out_of_bounds() {
        let data = [0b11111111];
        assert_eq!(
            read_bits_at(&data, 0, 9).unwrap_err(),
            ReadError::OutOfBounds
        );
    }

    #[test]
    fn test_read_bits_more_than_64() {
        let data = [0b11111111];
        assert_eq!(
            read_bits_at(&data, 0, 65).unwrap_err(),
            ReadError::TooManyBitsRead
        );
    }

    #[test]
    fn test_read_bits_at_out_of_bounds() {
        let data = [0b11111111];
        assert_eq!(
            read_bits_at(&data, 0, 9).unwrap_err(),
            ReadError::OutOfBounds
        );
    }

    #[test]
    fn test_read_bits_at_more_than_64() {
        let data = [0b11111111];
        assert_eq!(
            read_bits_at(&data, 0, 65).unwrap_err(),
            ReadError::TooManyBitsRead
        );
    }

    #[test]
    fn test_sign_extend() {
        assert_eq!(sign_extend(0b11111111, 8), -1);
    }

    #[test]
    fn test_reverse_bits_n() {
        assert_eq!(reverse_bits_n(0b10101010, 8), 0b01010101);
    }

    #[test]
    fn test_write_bits_at_aligned() {
        let mut buf = vec![0u8; 2];
        write_bits_at(&mut buf, 0, 8, 0xAB).unwrap();
        assert_eq!(buf, vec![0xAB, 0x00]);
    }

    #[test]
    fn test_write_bits_at_unaligned() {
        let mut buf = vec![0u8; 2];
        // write 0b1011 (4 bits) starting at bit 4 of byte 0
        write_bits_at(&mut buf, 4, 4, 0b1011).unwrap();
        assert_eq!(buf, vec![0b0000_1011, 0x00]);
    }

    #[test]
    fn test_write_bits_at_crosses_byte() {
        let mut buf = vec![0u8; 2];
        // write 0b1111_1111 (8 bits) starting at bit 4 of byte 0 → straddles byte boundary
        write_bits_at(&mut buf, 4, 8, 0xFF).unwrap();
        assert_eq!(buf, vec![0b0000_1111, 0b1111_0000]);
    }
}
