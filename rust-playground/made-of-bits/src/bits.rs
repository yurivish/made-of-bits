// Size (in bits) of basic blocks
pub(crate) const BASIC_BLOCK_SIZE: u32 = u32::BITS;
pub(crate) const BASIC_BLOCK_BITS: u32 = BASIC_BLOCK_SIZE.ilog2();

/// Block index of the block containing the `n`-th bit
pub(crate) fn basic_block_index(n: u32) -> usize {
    (n >> BASIC_BLOCK_BITS) as usize
}

/// Bit index of the `n`-th bit within its block (masking off the high bits)
pub(crate) fn basic_block_offset(n: u32) -> u32 {
    n & (BASIC_BLOCK_SIZE - 1)
}

/// Return the position of the k-th least significant set bit.
/// Assumes that x has at least k set Bits.
/// E.g. select1(0b1100, 0) === 2 and select1(0b1100, 1) === 3
///
/// Will panic due to overflow if the requested bit does not exist,
/// eg. select1(0b1100, 2)
///
/// As an aside, if we're interested in potentially more efficient approaches,
/// there is a broadword select1 implementation in the `succinct` package by
/// Jesse A. Tov, provided under an MIT license: https://github.com/tov/succinct-rs
///
/// An updated version of the paper is here: https://vigna.di.unimi.it/ftp/papers/Broadword.pdf
/// If we use this, here are some items for future work:
/// - Benchmark comparisons with the iterative select1 nelobelow
/// - Use simd128 to accelerate u_le8, le8, and u_nz8
/// - Implement 32-bit, 16-bit, and 8-bit select1
/// - Write my own tests (the original file had tests, but I'd like to practice writing my own)
pub(crate) fn select1(x: u32, k: u32) -> Option<u32> {
    // debug_assert!(x.count_ones() > k);
    // Unset the k-1 preceding 1-bits
    let mut x = x;
    for _ in 0..k {
        // prevent overflow when reaching for a bit that does not exist
        x &= x.max(1) - 1;
    }
    let i = x.trailing_zeros();
    if i == 32 {
        None
    } else {
        Some(i)
    }
}

/// Reverse the first `num_bits` bits of `x`.
pub(crate) fn reverse_low_bits(x: u32, num_bits: u32) -> u32 {
    x.reverse_bits() >> (u32::BITS - num_bits)
}

pub(crate) fn one_mask(n: u32) -> u32 {
    debug_assert!(n <= u32::BITS);
    if n == 0 {
        0
    } else {
        u32::MAX >> (u32::BITS - n)
    }
}

/// Perform binary search over a 0..n using bitwise binary search.
/// See: https://orlp.net/blog/bitwise-binary-search/
pub(crate) fn partition_point(n: usize, pred: impl Fn(usize) -> bool) -> usize {
    let mut b = 0;
    let mut bit = bit_floor(n);
    while bit != 0 {
        let i = (b | bit) - 1;
        if i < n && pred(i) {
            b |= bit
        }
        bit >>= 1;
    }
    b
}

/// If x is not zero, calculates the largest integral power of two that is not
/// greater than x. If x is zero, returns zero.
pub(crate) fn bit_floor(x: usize) -> usize {
    if x == 0 {
        0
    } else {
        let msb = usize::BITS - 1 - x.leading_zeros();
        1 << msb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_block_index() {
        // zero should always be zero, regardless of block size
        assert_eq!(basic_block_index(0), 0);
        // values less than a block size should map to the 0th block.
        assert_eq!(basic_block_index(15), 0);
        assert_eq!(basic_block_index(31), 0);
        // multiples of the block size should map to that block
        assert_eq!(basic_block_index(32), 1);
        assert_eq!(basic_block_index(32 + 15), 1);
        assert_eq!(basic_block_index(32 + 31), 1);

        assert_eq!(basic_block_index(2 * 32), 2);
        assert_eq!(basic_block_index(2 * 32 + 15), 2);
        assert_eq!(basic_block_index(2 * 32 + 31), 2);
    }

    #[test]
    fn test_bit_block_offset() {
        // zero should always be zero, regardless of block size
        assert_eq!(basic_block_offset(0), 0);
        // values less than a block size should be returned as they are.
        assert_eq!(basic_block_offset(15), 15);
        assert_eq!(basic_block_offset(31), 31);
        // multiples of the block size should be zero
        assert_eq!(basic_block_offset(32), 0);
        // values above that should wrap
        assert_eq!(basic_block_offset(32 + 15), 15);
        assert_eq!(basic_block_offset(32 + 31), 31);
    }

    #[test]
    fn test_one_mask() {
        for n in 0..32 {
            assert_eq!(one_mask(n), 2u32.pow(n) - 1);
            assert_eq!(one_mask(32), u32::MAX);
        }
    }

    #[test]
    fn test_select1() {
        {
            // returns None for a non-existent bit
            assert_eq!(select1(0, 0), None);
            assert_eq!(select1(0b11111, 5), None);
            assert_eq!(select1(0, 0), None);
            assert_eq!(select1(0, 0), None);
        }

        {
            // returns the index of the k-th bit (from the LSB up)
            let n = 0b0111000110010;
            assert_eq!(select1(n, 0), Some(1));
            assert_eq!(select1(n, 1), Some(4));
            assert_eq!(select1(n, 2), Some(5));
            assert_eq!(select1(n, 3), Some(9));
            assert_eq!(select1(n, 4), Some(10));
            assert_eq!(select1(n, 5), Some(11));
            assert_eq!(select1(n, 6), None);
        }
    }

    #[test]
    fn test_reverse_low_bits() {
        // reverses low bits and drops high bits
        assert_eq!(
            reverse_low_bits(0b11100000000000000000000000000001, 2),
            0b0000000000000000000000000000010
        );
        assert_eq!(
            reverse_low_bits(0b11100000000000000000000000000001, 5),
            0b0000000000000000000000000010000
        );
        assert_eq!(
            reverse_low_bits(0b00000000000000000000000000000001, 3),
            0b0000000000000000000000000000100
        );
        assert_eq!(
            reverse_low_bits(0b00000000000000000000000000000101, 6),
            0b0000000000000000000000000101000
        );
    }

    #[test]
    fn test_bit_floor() {
        assert_eq!(bit_floor(0), 0);
        assert_eq!(bit_floor(1), 1);
        assert_eq!(bit_floor(2), 2);
        assert_eq!(bit_floor(3), 2);
        assert_eq!(bit_floor(4), 4);
        assert_eq!(bit_floor(5), 4);
    }

    #[test]
    fn test_partition_point() {
        let n = 100;
        let target = 60;
        assert_eq!(partition_point(n, |i| i < target), target);
        assert_eq!(partition_point(target - 1, |i| i < target), target - 1);

        assert_eq!(partition_point(0, |_| true), 0);
        assert_eq!(partition_point(1, |_| true), 1);
    }
}
