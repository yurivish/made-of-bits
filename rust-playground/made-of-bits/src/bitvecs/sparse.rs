use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::{
    bitbuf::BitBuf,
    bits::{one_mask, partition_point},
    bitvec::{BitVec, BitVecBuilder},
    bitvecs::dense::DenseBitVec,
    intbuf::IntBuf,
};

pub struct SparseBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl SparseBitVecBuilder {
    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }
}

impl BitVecBuilder for SparseBitVecBuilder {
    type Target = SparseBitVec<false>;

    fn new(universe_size: u32) -> Self {
        Self::new(universe_size)
    }

    fn one(&mut self, bit_index: u32) {
        self.one_count(bit_index, 1);
    }

    fn build(mut self) -> SparseBitVec<false> {
        self.ones.sort();
        self.ones.dedup();
        SparseBitVec::new(self.ones.into(), self.universe_size)
    }
}

impl MultiBitVecBuilder for SparseBitVecBuilder {
    type Target = SparseBitVec<true>;

    fn new(universe_size: u32) -> Self {
        Self::new(universe_size)
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        assert!(bit_index < self.universe_size);
        for _ in 0..count {
            self.ones.push(bit_index);
        }
    }

    fn build(mut self) -> SparseBitVec<true> {
        self.ones.sort();
        SparseBitVec::new(self.ones.into(), self.universe_size)
    }
}

#[derive(Clone)]
pub struct SparseBitVec<const M: bool> {
    high: DenseBitVec,
    low: IntBuf,
    num_ones: u32,
    low_bit_width: u32,
    low_mask: u32,
    universe_size: u32,
    num_zeros: u32,
    num_unique_ones: u32,
}

impl<const M: bool> SparseBitVec<M> {
    pub fn new(ones: Box<[u32]>, universe_size: u32) -> Self {
        let num_ones = ones.len() as u32;

        // The paper "On Elias-Fano for Rank Queries in FM-Indexes" recommends a formula to compute
        // the number of low bits that is mostly equivalent to the version used below, except that
        // sometimes theirs suggests slightly worse choices, e.g. when numOnes === 25 and universeSize === 51.
        // https://observablehq.com/@yurivish/ef-split-points
        // This approach chooses the split point by noting that the trade-off effectively is between having numOnes
        // low bits, or the next power of two of the universe size separators in the high bits. Hopefully this will
        // be explained clearly in the accompanying design & background documentation.
        let low_bit_width = if num_ones == 0 {
            0
        } else {
            (universe_size / num_ones).max(1).ilog2()
        };

        // unary coding; 1 denotes values and 0 denotes separators, since that way
        // encoding becomes more efficient and we have a chance of saving space due to runs of
        // zeros at either end, if the values are clustered away from the domain edges.
        // By default, values are never more than 50% of the bits due to the way the split point is chosen.
        // Note that this expression automatically adapts to non-power-of-two universe sizes.
        let high_len = num_ones + (universe_size >> low_bit_width);
        let mut high = BitBuf::new(high_len);
        let mut low = IntBuf::new(num_ones, low_bit_width);
        let low_mask = one_mask(low_bit_width);

        let mut num_unique_ones = 0;
        let mut has_multiplicity = false;
        let mut prev = None;
        for (i, &cur) in ones.iter().enumerate() {
            let same = prev == Some(cur);
            num_unique_ones += if same { 0 } else { 1 };
            if let Some(prev) = prev {
                debug_assert!(prev <= cur, "ones must be sorted")
            }
            assert!(
                cur < universe_size,
                "expected 1-bit index ({}) to not exceed the universeSize ({})",
                cur,
                universe_size
            );
            debug_assert!(prev.is_none() || prev.unwrap() <= cur); // expected monotonically nondecreasing sequence
            prev = Some(cur);

            // Encode element
            let quotient = cur >> low_bit_width;
            high.set_one(i as u32 + quotient);
            let remainder = cur & low_mask;
            low.push(remainder);
        }

        assert!(
            !has_multiplicity || M, // either we have no multiplicity, or M is true and allows multiplicity
            "cannot construct a SparseBitVec with multiplicity when the type parameter M is false"
        );

        let num_zeros = universe_size - num_unique_ones;
        Self {
            high: DenseBitVec::new(high, 10, 10),
            low,
            low_bit_width,
            low_mask,
            num_ones,
            num_zeros,
            num_unique_ones,
            universe_size,
        }
    }

    fn num_unique_ones(&self) -> u32 {
        self.num_unique_ones
    }

    // todo: document this and remainder
    fn quotient(&self, x: u32) -> u32 {
        return x >> self.low_bit_width;
    }

    fn remainder(&self, x: u32) -> u32 {
        return x & self.low_mask;
    }
}

impl<const M: bool> SparseBitVec<M> {
    fn rank1(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_ones;
        }

        let quotient = self.quotient(bit_index);
        let lower_bound;
        let upper_bound;
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

        // Count the number of elements in this bucket that are strictly below i
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
}

impl BitVec for SparseBitVec<false> {
    fn rank1(&self, bit_index: u32) -> u32 {
        self.rank1(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.select1(n)
    }

    fn universe_size(&self) -> u32 {
        self.universe_size()
    }

    fn num_ones(&self) -> u32 {
        self.num_ones()
    }
}

impl MultiBitVec for SparseBitVec<true> {
    fn rank1(&self, bit_index: u32) -> u32 {
        self.rank1(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.select1(n)
    }

    fn universe_size(&self) -> u32 {
        self.universe_size()
    }

    fn num_unique_ones(&self) -> u32 {
        self.num_unique_ones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec_test::*;

    #[test]
    fn test() {
        test_bitvec_builder::<SparseBitVecBuilder>();
        property_test_bitvec_builder::<SparseBitVecBuilder>(None, None, false);
    }
}