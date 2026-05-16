use crate::bitblock::BitBlock;

/// `SELECT_IN_BYTE[k * 256 + b]` = position (0-7) of the `k`-th set bit (zero-indexed)
/// in byte value `b`. Entries where `b` has fewer than `k+1` set bits are 8 (sentinel).
/// Used by the broadword `select64` algorithm to look up the final byte's bit position.
///
/// 2 KiB total, computed at compile time.
const SELECT_IN_BYTE: [u8; 2048] = {
    let mut table = [8u8; 2048];
    let mut b = 0usize;
    while b < 256 {
        let mut rank = 0usize;
        let mut i = 0u8;
        while i < 8 {
            if b & (1 << i) != 0 {
                table[rank * 256 + b] = i;
                rank += 1;
            }
            i += 1;
        }
        b += 1;
    }
    table
};

/// Vigna's broadword `select` for 64-bit words: returns the position of the `k`-th
/// (zero-indexed) set bit in `x`. Assumes `x` has at least `k + 1` set bits;
/// behavior on insufficient inputs is unspecified (callers should bounds-check first
/// or use [`select64_checked`]).
///
/// O(1) — uses byte-level SWAR popcount, broadword parallel comparison to locate the
/// target byte, then [`SELECT_IN_BYTE`] for the in-byte position.
///
/// Reference: <https://vigna.di.unimi.it/ftp/papers/Broadword.pdf>.
/// Ported from `Select64` in `madeofbits/bits.go`.
#[inline]
pub(crate) fn select64(x: u64, k: u32) -> u32 {
    debug_assert!(
        (k as u64) < x.count_ones() as u64,
        "select64: k={k} but x has {} set bits",
        x.count_ones(),
    );

    const ONES: u64 = 0x0101010101010101;
    const MSBS: u64 = 0x8080808080808080;

    // Byte-level popcount via SWAR (sideways addition).
    let mut s = x - ((x >> 1) & 0x5555555555555555);
    s = (s & 0x3333333333333333) + ((s >> 2) & 0x3333333333333333);
    s = (s + (s >> 4)) & 0x0F0F0F0F0F0F0F0F;

    // Prefix-sum the byte popcounts: byte i of byte_sums = popcount(x[0:i+1]).
    let byte_sums = s.wrapping_mul(ONES);

    // Broadword parallel comparison: byte i has MSB set iff k >= cumulative_sum[i].
    // count_ones of the MSB pattern is the index of the first byte whose cumulative
    // sum exceeds k.
    let k64 = k as u64;
    let geq = ((k64 * ONES) | MSBS).wrapping_sub(byte_sums) & MSBS;

    // Bit offset of the target byte within the word (multiple of 8).
    let place = (geq.count_ones() as u64) << 3;

    // Adjust k to be relative within the target byte:
    //   k -= cumulative count of 1-bits *before* the target byte.
    let local_k = k64 - (((byte_sums << 8) >> place) & 0xFF);

    place as u32
        + SELECT_IN_BYTE[(local_k as usize) * 256 + (((x >> place) & 0xFF) as usize)] as u32
}

/// Safe variant: returns `Some(pos)` if `x` has at least `k + 1` set bits, else `None`.
#[inline]
pub(crate) fn select64_checked(x: u64, k: u32) -> Option<u32> {
    if (k as u64) < x.count_ones() as u64 {
        Some(select64(x, k))
    } else {
        None
    }
}

/// `floor(log2(x))`. Panics if `x == 0`.
#[inline]
pub(crate) fn ilog2(x: u64) -> u32 {
    x.ilog2()
}

/// Linear O(popcount) reference implementation of [`select64`], kept as ground truth for
/// the broadword version's tests. Returns `Some(pos)` if found, `None` otherwise.
/// Ported from `Select64Simple` in `madeofbits/bits.go`.
pub(crate) fn select64_simple(x: u64, k: u32) -> Option<u32> {
    let mut x = x;
    for _ in 0..k {
        x &= x.wrapping_sub(1);
    }
    if x == 0 { None } else { Some(x.trailing_zeros()) }
}

/// Return the position of the k-th least significant set bit.
/// Assumes that x has at least k set Bits.
/// E.g. select1(0b1100, 0) === 2 and select1(0b1100, 1) === 3
///
/// Will panic due to overflow if the requested bit does not exist,
/// eg. select1(0b1100, 2)
///
/// Generic across u32/u64/u128. For u64 specifically, [`select64`] is much faster.
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

/// Reverse the lowest `num_bits` bits of `x`.
///
/// For example, `reverse_low_bits(0b0000100100, 6)` is `0b0000001001`.
///                                      ^^^^^^                ^^^^^^
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
    fn test_select64_vs_simple_exhaustive_low_popcount() {
        // Exhaustive cross-check for all u64 values with popcount <= 4 — covers every
        // bit position pattern up to small popcount, ~600k cases. Runs in ~milliseconds.
        for popcount in 0..=4u32 {
            for mask in low_popcount_words(popcount, 64) {
                for k in 0..popcount {
                    assert_eq!(
                        select64_checked(mask, k),
                        select64_simple(mask, k),
                        "select64({mask:016x}, {k})",
                    );
                }
                // One past the last is None.
                assert_eq!(select64_checked(mask, popcount), None);
            }
        }
    }

    /// Generate every u64 with exactly `popcount` set bits, restricted to the lowest
    /// `width` positions. For popcount <= 4 and width = 64 this is ~635k values.
    fn low_popcount_words(popcount: u32, width: u32) -> Vec<u64> {
        fn rec(popcount: u32, width: u32, base: u64, out: &mut Vec<u64>) {
            if popcount == 0 {
                out.push(base);
                return;
            }
            if popcount > width {
                return;
            }
            // Choose where the highest remaining bit lives.
            for hi in (popcount - 1)..width {
                rec(popcount - 1, hi, base | (1u64 << hi), out);
            }
        }
        let mut out = Vec::new();
        rec(popcount, width, 0, &mut out);
        out
    }

    #[test]
    fn test_select64_boundary_positions() {
        // Single bit at each of {0,1,30,31,32,33,62,63}: select64(_, 0) returns that
        // position. Pins down off-by-ones at byte and u32 boundaries.
        for pos in [0u32, 1, 30, 31, 32, 33, 62, 63] {
            let x = 1u64 << pos;
            assert_eq!(select64(x, 0), pos, "select64(1 << {pos}, 0)");
            assert_eq!(select64_checked(x, 1), None);
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
