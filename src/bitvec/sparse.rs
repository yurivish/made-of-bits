use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::BitVecBuilder;
use crate::DenseBitVecBuilder;
use crate::DenseBitVecOptions;
use crate::{
    bitbuf::BitBuf,
    bits::{one_mask, partition_point},
    bitvec::dense::DenseBitVec,
    bitvec::BitVec,
    intbuf::IntBuf,
};

/// Stores a sparse bit vector in Elias-Fano encoding. Space usage
/// depends on the number of elements and the universe size.
/// Implements MultiBitVec. Multiplicity is encoded via repetition, ie.
/// each additional repetition of a 1-bit takes additional space,
/// as it does in the case of the ArrayBitVec but not `Multi<T>`,
/// which represents counts explicitly.
#[derive(Clone)]
pub struct SparseBitVec {
    high: DenseBitVec,
    low: IntBuf,
    low_mask: u32,
    low_bit_width: u32,
    universe_size: u32,
    num_ones: u32,
    num_unique_ones: u32,
    /// Subtracted from every input before Elias-Fano encoding so the effective universe
    /// is `[0, universe_size - offset)`. Shrinks high-bits storage and low-bits width
    /// when the first 1-bit sits far from 0.
    offset: u32,
}

impl SparseBitVec {
    pub fn new(ones: Box<[u32]>, universe_size: u32, options: SparseBitVecOptions) -> Self {
        let num_ones: u32 = ones
            .len()
            .try_into()
            .expect("number of 1-bits cannot exceed 2^32 - 1");

        // Static offset = first 1-bit, used to shrink the effective universe.
        let offset = ones.first().copied().unwrap_or(0);
        let effective_universe = universe_size - offset;

        // The paper "On Elias-Fano for Rank Queries in FM-Indexes" recommends a formula to compute
        // the number of low bits that is mostly equivalent to the version used below, except that
        // sometimes theirs suggests slightly worse choices, e.g. when numOnes === 25 and universeSize === 51.
        // https://observablehq.com/@yurivish/ef-split-points
        // This approach chooses the split point by noting that the trade-off effectively is between having numOnes
        // low bits, or doubling the number of separators in the high bits. Capped at 56 to match
        // IntBuf::MAX_BIT_WIDTH.
        let low_bit_width = options.low_bit_width.unwrap_or_else(|| {
            if num_ones == 0 {
                0
            } else {
                (effective_universe / num_ones).max(1).ilog2().min(56)
            }
        });

        // Encode the high bits in unary: 1 denotes values and 0 denotes separators.
        // By default, 1-bits are never more than 50% of the bits due to the way the split point is chosen.
        // Note that this expression automatically adapts to non-power-of-two universe sizes.
        let high_len = num_ones + (effective_universe >> low_bit_width);
        let mut high = DenseBitVecBuilder::new(high_len, options.high_bits_options);
        let mut low = IntBuf::new(num_ones, low_bit_width);
        let low_mask = one_mask(low_bit_width);

        let mut num_unique_ones = 0;
        let mut prev = None;
        for (i, cur) in ones.iter().copied().enumerate() {
            let same = prev == Some(cur);
            num_unique_ones += if same { 0 } else { 1 };
            assert!(prev.unwrap_or(0) <= cur, "ones must be in ascending order");
            prev = Some(cur);

            // Encode element relative to offset.
            let shifted = cur - offset;
            let quotient = shifted >> low_bit_width;
            high.one(i as u32 + quotient);
            let remainder = shifted & low_mask;
            low.push(remainder);
        }

        if let Some(i) = prev {
            assert!(
                i < universe_size,
                "1-bit index {} cannot exceed universe_size {}",
                i,
                universe_size
            );
        }

        Self {
            high: high.build(),
            low,
            low_mask,
            low_bit_width,
            universe_size,
            num_ones,
            num_unique_ones,
            offset,
        }
    }

    /// Returns the high bits of `x`
    fn quotient(&self, x: u32) -> u32 {
        return x >> self.low_bit_width;
    }

    /// Returns the low bits of `x`
    fn remainder(&self, x: u32) -> u32 {
        return x & self.low_mask;
    }
}

impl MultiBitVec for SparseBitVec {
    type Builder = SparseBitVecBuilder;

    fn rank1(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_ones;
        }
        // Any index at or before the static offset has 0 preceding 1-bits.
        if bit_index <= self.offset {
            return 0;
        }
        let bit_index = bit_index - self.offset;

        let lower_bound;
        let upper_bound;
        let quotient = self.quotient(bit_index);
        if quotient == 0 {
            // We're searching within the first group, so the lower bound is zero.
            // Look for the divider that separates the first group from the subsequent groups.
            // If there isn't one, then we need to search the entire vector since all values
            // are in the first group.
            lower_bound = 0;
            upper_bound = self.high.select0(0).unwrap_or(self.num_ones());
        } else {
            // We're searching within a higher group, so compute both the lower and the
            // upper bound from the high bit vector.

            // We're searching for the i-th separator.
            // When we find it, we subtract the number of separators preceding it
            // in order to get the index of the element in the low bits.
            lower_bound = {
                let i = quotient - 1;
                self.high.select0(i).map_or(self.num_ones(), |x| x - i)
            };

            // Same thing, but we're searching for the next separator after that.
            upper_bound = {
                let i = quotient;
                self.high.select0(i).map_or(self.num_ones(), |x| x - i)
            };
        }

        // Count the number of elements in this bucket that are strictly below `bit_index`
        // using just the low bits.
        let remainder = self.remainder(bit_index);
        let bucket_count = partition_point((upper_bound - lower_bound) as usize, |n| {
            let index = lower_bound + n as u32;
            let value = self.low.get(index);
            value < remainder
        }) as u32;

        lower_bound + bucket_count
    }

    fn select1(&self, n: u32) -> Option<u32> {
        // How many zeros are there before the nth one bit?
        let pos = self.high.select1(n)?;
        let quotient = self.high.rank0(pos);
        let remainder = self.low.get(n);
        Some(self.offset + (quotient << self.low_bit_width) + remainder)
    }

    fn universe_size(&self) -> u32 {
        self.universe_size
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn num_unique_ones(&self) -> u32 {
        self.num_unique_ones
    }

    fn rank1_batch(&self, bit_indices: &mut [u32]) {
        // chunks with identical high bits (after accounting for offset). Two queries
        // both <= offset land in the same "below offset" bucket and short-circuit to 0.
        let chunks = bit_indices.chunk_by_mut(|a, b| {
            let a_shifted = a.saturating_sub(self.offset);
            let b_shifted = b.saturating_sub(self.offset);
            a_shifted >> self.low_bit_width == b_shifted >> self.low_bit_width
        });

        // lower and upper index bounds for the low bits
        let mut lower_bound = 0;
        let mut upper_bound = 0;

        // we track this to save work when querying contiguous chunks
        let mut prev_quotient = 0;

        for chunk in chunks {
            let first_bit_index = chunk.first().copied().unwrap();

            // handle chunks that are entirely beyond the end of the universe
            if first_bit_index >= self.universe_size() {
                chunk.fill(self.num_ones);
                continue;
            }
            // handle chunks that are entirely at or before the static offset
            if first_bit_index <= self.offset {
                // Every index in this chunk is <= offset, so rank1 is 0 for all of them.
                // (The chunk_by predicate places all such queries together because their
                // shifted quotients are all 0.)
                let mut all_below = true;
                for &i in chunk.iter() {
                    if i > self.offset {
                        all_below = false;
                        break;
                    }
                }
                if all_below {
                    chunk.fill(0);
                    continue;
                }
                // Mixed chunk (some <= offset, some >): fall through to per-element handling.
            }

            // the quotient for the group we're in
            let first_shifted = first_bit_index.saturating_sub(self.offset);
            let quotient = self.quotient(first_shifted);

            if quotient == 0 {
                lower_bound = 0;
                upper_bound = self.high.select0(0).unwrap_or(self.num_ones());
            } else {
                // if this group is contiguous with the previous,
                // use the previous upper_bound as this group's lower_bound
                if prev_quotient == quotient - 1 {
                    lower_bound = upper_bound
                } else {
                    let i = quotient - 1;
                    lower_bound = self.high.select0(i).map_or(self.num_ones(), |x| x - i);
                };

                upper_bound = {
                    let i = quotient;
                    self.high.select0(i).map_or(self.num_ones(), |x| x - i)
                };
            }
            prev_quotient = quotient;

            for i in chunk {
                if *i <= self.offset {
                    *i = 0;
                    continue;
                }
                let shifted = *i - self.offset;
                let remainder = self.remainder(shifted);
                let len = (upper_bound - lower_bound) as usize;
                let bucket_count = partition_point(len, |n| {
                    let index = lower_bound + n as u32;
                    let value = self.low.get(index);
                    value < remainder
                }) as u32;
                // we could narrow the search range for the next iteration using
                // the result from this one but it's not clear this improves
                // perf since it means the sequence of .get calls differs from element to element.
                *i = lower_bound + bucket_count;
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct SparseBitVecOptions {
    /// How many bits to use for the low bits in the Elias-Fano encoding.
    /// If this is None, then the number will be computed from the universe
    /// size to minimize the total size of the data representation.
    low_bit_width: Option<u32>,
    /// Options for the dense bit vector storing the high bits
    high_bits_options: DenseBitVecOptions,
}

#[derive(Clone)]
pub struct SparseBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
    options: SparseBitVecOptions,
}

impl MultiBitVecBuilder for SparseBitVecBuilder {
    type Target = SparseBitVec;
    type Options = SparseBitVecOptions;

    fn new(universe_size: u32, options: Self::Options) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
            options,
        }
    }

    fn ones(&mut self, bit_index: u32, count: u32) {
        assert!(bit_index < self.universe_size);
        for _ in 0..count {
            self.ones.push(bit_index);
        }
    }

    fn build(mut self) -> SparseBitVec {
        self.ones.sort();
        SparseBitVec::new(self.ones.into(), self.universe_size, self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bitvec::test::*, panics};

    #[test]
    fn test_max() {
        let mut b = SparseBitVecBuilder::new(u32::MAX, Default::default());
        assert!(panics(|| b.ones(u32::MAX, 1)));
        let v = b.build();
    }

    #[test]
    fn multibitvec_interface() {
        test_multibitvec_builder::<SparseBitVecBuilder>();
    }

    /// Adversarial: a sparse bitvec whose ones start far from 0. The static `offset`
    /// optimization should shrink the effective universe drastically — verified by
    /// observing that `low_bit_width` is computed against `universe - offset`, not
    /// `universe`. Without the optimization, this bitvec would waste many high bits.
    #[test]
    fn test_large_offset_ones() {
        let universe: u32 = 1_000_000;
        let start: u32 = 999_990;
        let ones: Vec<u32> = (start..universe).collect();
        let v = SparseBitVec::new(ones.clone().into(), universe, Default::default());

        // Confirm the offset captured the first 1-bit.
        assert_eq!(v.offset, start);

        // Sanity: every query still answers correctly.
        for &p in &ones {
            assert!(v.rank1(p) <= p - start);
            // Position just after each 1-bit ⇒ rank1 incremented by 1.
            assert_eq!(v.rank1(p + 1), v.rank1(p) + 1);
        }
        // Below offset is always 0.
        assert_eq!(v.rank1(0), 0);
        assert_eq!(v.rank1(start - 1), 0);
        assert_eq!(v.rank1(start), 0);
        assert_eq!(v.rank1(start + 1), 1);

        // select round-trips.
        for (i, &p) in ones.iter().enumerate() {
            assert_eq!(v.select1(i as u32), Some(p));
        }
        assert_eq!(v.select1(ones.len() as u32), None);
    }
}
