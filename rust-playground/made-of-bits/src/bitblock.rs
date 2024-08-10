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
