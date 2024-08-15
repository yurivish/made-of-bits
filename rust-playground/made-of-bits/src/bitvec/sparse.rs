use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::{
    bitbuf::BitBuf,
    bits::{one_mask, partition_point},
    bitvec::dense::DenseBitVec,
    bitvec::BitVec,
    intbuf::IntBuf,
};

#[derive(Clone)]
pub struct SparseBitVec {
    high: DenseBitVec,
    low: IntBuf,
    low_mask: u32,
    low_bit_width: u32,
    universe_size: u32,
    num_ones: u32,
    num_unique_ones: u32,
}

impl SparseBitVec {
    pub fn new(ones: Box<[u32]>, universe_size: u32, low_bit_width: Option<u32>) -> Self {
        let num_ones: u32 = ones
            .len()
            .try_into()
            .expect("number of 1-bits cannot exceed 2^32 - 1");

        // The paper "On Elias-Fano for Rank Queries in FM-Indexes" recommends a formula to compute
        // the number of low bits that is mostly equivalent to the version used below, except that
        // sometimes theirs suggests slightly worse choices, e.g. when numOnes === 25 and universeSize === 51.
        // https://observablehq.com/@yurivish/ef-split-points
        // This approach chooses the split point by noting that the trade-off effectively is between having numOnes
        // low bits, or the next power of two of the universe size separators in the high bits. Hopefully this will
        // be explained clearly in the accompanying design & background documentation.
        let low_bit_width = low_bit_width.unwrap_or_else(|| {
            if num_ones == 0 {
                0
            } else {
                (universe_size / num_ones).max(1).ilog2()
            }
        });

        // unary coding; 1 denotes values and 0 denotes separators, since that way
        // encoding becomes more efficient and we have a chance of saving space due to runs of
        // zeros at either end, if the values are clustered away from the domain edges.
        // NOTE: ^ The above comment is only true if we use a padded dense bitvector, which we currently do not.
        // By default, values are never more than 50% of the bits due to the way the split point is chosen.
        // Note that this expression automatically adapts to non-power-of-two universe sizes.
        let high_len = num_ones + (universe_size >> low_bit_width);
        dbg!((universe_size >> low_bit_width));
        dbg!(low_bit_width);
        dbg!((universe_size >> low_bit_width).saturating_sub(1));
        dbg!(high_len, universe_size, num_ones);
        let mut high = BitBuf::new(high_len);
        let mut low = IntBuf::new(num_ones, low_bit_width);
        let low_mask = one_mask(low_bit_width);

        // this check assumes sorted order, which is verified in the loop using debug_assert.
        assert!(
            ones.last().map(|&i| i < universe_size).unwrap_or(true),
            "1-bit indices cannot exceed universeSize ({})",
            universe_size
        );

        let mut num_unique_ones = 0;
        let mut prev = None;
        for (i, cur) in ones.iter().copied().enumerate() {
            let same = prev == Some(cur);
            num_unique_ones += if same { 0 } else { 1 };
            if let Some(prev) = prev {
                debug_assert!(prev <= cur, "ones must be in ascending order")
            }
            debug_assert!(prev.is_none() || prev.unwrap() <= cur); // expected monotonically nondecreasing sequence
            prev = Some(cur);

            // Encode element
            let quotient = cur >> low_bit_width;
            dbg!(i, quotient);
            high.set_one(i as u32 + quotient);
            let remainder = cur & low_mask;
            low.push(remainder);
        }

        Self {
            high: DenseBitVec::new(high, 10, 10),
            low,
            low_mask,
            low_bit_width,
            universe_size,
            num_ones,
            num_unique_ones,
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
            let i = quotient - 1;
            let n = self.high.select0(i).map(|x| x - i);
            lower_bound = n.unwrap_or(0);

            // Same thing, but we're searching for the next separator after that.
            let i = quotient;
            let n = self.high.select0(i).map(|x| x - i);
            upper_bound = n.unwrap_or(self.num_ones());
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
        Some((quotient << self.low_bit_width) + remainder)
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
}

#[derive(Default, Clone)]
pub struct SparseBitVecOptions {
    /// How many bits to use for the low bits in the Elias-Fano encoding.
    /// If this is None, then the number will be computed from the universe
    /// size to minimize the total size of the data representation.
    low_bit_width: Option<u32>,
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
        SparseBitVec::new(
            self.ones.into(),
            self.universe_size,
            self.options.low_bit_width,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::test::*;

    #[test]
    fn multibitvec_interface() {
        test_multibitvec_builder::<SparseBitVecBuilder>();
    }
}
