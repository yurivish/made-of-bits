use std::cmp::PartialOrd;
use std::ops::{BitAndAssign, Shr, Sub};

/// Represents a block of a BitBuf or an IntBuf. Implemented by u32 and u64.
pub(crate) trait BitBlock:
    Copy + Sized + PartialOrd<Self> + BitAndAssign + Sub<Self> + Shr<u32, Output = Self>
{
    /// Number of bits in this bit block
    const BITS: u32;
    /// Power of 2 of the number of bits in this bit block
    const BITS_LOG2: u32 = Self::BITS.ilog2();

    const ZERO: Self;
    const ONE: Self;
    const MAX: Self;

    // Delegated to the underlying block type by the individual impls
    fn saturating_sub(self, rhs: Self) -> Self;
    fn trailing_zeros(self) -> u32;

    /// Block index of the block containing the `n`-th bit
    fn block_index(n: u32) -> usize {
        // We use 'as usize' here since `n` is a u32 so this will work
        // regardless of whether usize is u32 or u64 (and we assume it
        // to be no less than u32).
        (n >> Self::BITS_LOG2) as usize
    }

    /// Bit index of the `n`-th bit within its block (masking off the high bits)
    fn block_bit_index(n: u32) -> u32;
}

macro_rules! bit_block_impl {
    ($type:ty) => {
        impl BitBlock for $type {
            const BITS: u32 = Self::BITS;
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = Self::MAX;

            fn saturating_sub(self, rhs: Self) -> Self {
                Self::saturating_sub(self, rhs)
            }

            fn trailing_zeros(self) -> u32 {
                Self::trailing_zeros(self)
            }

            fn block_bit_index(n: u32) -> u32 {
                n & (Self::BITS - 1)
            }
        }
    };
}

bit_block_impl!(u32);
bit_block_impl!(u64);
bit_block_impl!(u128);

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
/// - Benchmark comparisons with the iterative select1 below
/// - Use simd128 to accelerate u_le8, le8, and u_nz8
/// - Implement 32-bit, 16-bit, and 8-bit select1
/// - Write my own tests (the original file had tests, but I'd like to practice writing my own)
pub(crate) fn select1<T: BitBlock>(x: T, k: u32) -> Option<u32> {
    // Unset the k-1 preceding 1-bits
    let mut x = x;
    for _ in 0..k {
        // prevent overflow when reaching for a bit that does not exist
        x &= x.saturating_sub(T::ONE);
    }
    let i = x.trailing_zeros();
    if i == T::BITS {
        // x is 0; there is no k-th bit
        None
    } else {
        Some(i)
    }
}

// todo: this doctest fails because this function is not exported!
//
// /// Reverse the lowest `num_bits` bits of `x`.
// ///
// /// ```
// /// assert_eq!(reverse_low_bits(0b0000100100, 6), 0b0000001001)
// /// //                                ^^^^^^            ^^^^^^
// /// ```
// ///
pub(crate) const fn reverse_low_bits(x: usize, num_bits: usize) -> usize {
    x.reverse_bits() >> (usize::BITS as usize - num_bits)
}

pub(crate) fn one_mask<T: BitBlock>(n: u32) -> T {
    debug_assert!(n <= T::BITS);
    if n == 0 {
        T::ZERO
    } else {
        T::MAX >> (T::BITS - n)
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
pub(crate) const fn bit_floor(x: usize) -> usize {
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
    fn test_one_mask() {
        for n in 0..32 {
            assert_eq!(one_mask::<u32>(n), 2u32.pow(n) - 1);
            assert_eq!(one_mask::<u32>(32), u32::MAX);
        }

        for n in 0..64 {
            assert_eq!(one_mask::<u64>(n), 2u64.pow(n) - 1);
            assert_eq!(one_mask::<u64>(64), u64::MAX);
        }
    }

    #[test]
    fn test_select1() {
        // TODO: Test other block sizes. Can use a macro to repeat tests.
        // TODO: Test for bits near the end (eg. bit 31, bit 63).

        {
            // returns None for a non-existent bit
            assert_eq!(select1::<u32>(0, 0), None);
            assert_eq!(select1::<u32>(0b11111, 5), None);
            assert_eq!(select1::<u32>(0, 0), None);
            assert_eq!(select1::<u32>(0, 0), None);
        }

        {
            // returns the index of the k-th bit (from the LSB up)
            let n = 0b0111000110010;
            assert_eq!(select1::<u32>(n, 0), Some(1));
            assert_eq!(select1::<u32>(n, 1), Some(4));
            assert_eq!(select1::<u32>(n, 2), Some(5));
            assert_eq!(select1::<u32>(n, 3), Some(9));
            assert_eq!(select1::<u32>(n, 4), Some(10));
            assert_eq!(select1::<u32>(n, 5), Some(11));
            assert_eq!(select1::<u32>(n, 6), None);
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

    fn test_block_index<T: BitBlock>() {
        // zero should always be zero, regardless of block size
        assert_eq!(T::block_index(0), 0);
        // values less than a block size should map to the 0th block.
        assert_eq!(T::block_index(15), 0);
        assert_eq!(T::block_index(31), 0);
        // multiples of the block size should map to that block
        assert_eq!(T::block_index(T::BITS), 1);
        assert_eq!(T::block_index(T::BITS + 15), 1);
        assert_eq!(T::block_index(T::BITS + 31), 1);

        assert_eq!(T::block_index(2 * T::BITS), 2);
        assert_eq!(T::block_index(2 * T::BITS + 15), 2);
        assert_eq!(T::block_index(2 * T::BITS + 31), 2);
    }

    fn test_block_offset<T: BitBlock>() {
        // zero should always be zero, regardless of block size
        assert_eq!(T::block_bit_index(0), 0);
        // values less than a block size should be returned as they are.
        assert_eq!(T::block_bit_index(15), 15);
        assert_eq!(T::block_bit_index(31), 31);
        // multiples of the block size should be zero
        assert_eq!(T::block_bit_index(T::BITS), 0);
        // values above that should wrap
        assert_eq!(T::block_bit_index(T::BITS + 15), 15);
        assert_eq!(T::block_bit_index(T::BITS + 31), 31);
    }

    #[test]
    fn test_block_index_and_offset() {
        test_block_index::<u32>();
        test_block_offset::<u32>();
        test_block_index::<u64>();
        test_block_offset::<u64>();
    }
}
