use crate::bitbuf::BitBuf;
use crate::bits::one_mask;
use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::densebitvec::DenseBitVec;
use crate::intbuf::IntBuf;

pub struct SparseBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl BitVecBuilder for SparseBitVecBuilder {
    type Target = SparseBitVec;

    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        assert!(bit_index < self.universe_size);
        for _ in 0..count {
            self.ones.push(bit_index);
        }
    }

    fn build(mut self) -> SparseBitVec {
        self.ones.sort();
        SparseBitVec::new(self.ones.into(), self.universe_size)
    }
}

#[derive(Clone)]
pub struct SparseBitVec {
    high: DenseBitVec,
    low: IntBuf,
    num_ones: u32,
    low_bit_width: u32,
    low_mask: u32,
    universe_size: u32,
    num_zeros: u32,
    num_unique_ones: u32,
    has_multiplicity: bool,
}

impl SparseBitVec {
    fn new(ones: Box<[u32]>, universe_size: u32) -> Self {
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
            has_multiplicity |= same;
            num_unique_ones += if same { 0 } else { 1 };
            if let Some(prev) = prev {
                debug_assert!(prev <= cur, "ones must be sorted")
            }
            assert!(cur < universe_size); // expected 1 - bit(${cur}) to not exceed the universeSize(${universeSize})`
            debug_assert!(prev.unwrap() <= cur); // expected monotonically nondecreasing sequence
            prev = Some(cur);

            // Encode element
            let quotient = cur >> low_bit_width;
            high.set_one(i as u32 + quotient);
            let remainder = cur & low_mask;
            low.push(remainder);
        }

        let num_zeros = universe_size - num_unique_ones;
        Self {
            high: DenseBitVec::new(high, 10, 10),
            low,
            low_bit_width,
            low_mask,
            num_ones,
            num_zeros,
            num_unique_ones,
            has_multiplicity,
            universe_size,
        }
    }
}

impl BitVec for SparseBitVec {
    fn rank1(&self, bit_index: u32) -> u32 {
        todo!()
    }

    fn has_multiplicity(&self) -> bool {
        todo!()
    }

    fn num_ones(&self) -> u32 {
        todo!()
    }

    fn num_zeros(&self) -> u32 {
        todo!()
    }

    fn universe_size(&self) -> u32 {
        todo!()
    }

    fn num_unique_zeros(&self) -> u32 {
        todo!()
    }

    fn num_unique_ones(&self) -> u32 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::test_bitvec::test_bit_vec_builder;
    use crate::test_bitvec::test_bit_vec_builder_arbtest;

    use super::*;

    #[test]
    fn test() {
        test_bit_vec_builder::<SparseBitVecBuilder>();
        // test_bit_vec_builder_arbtest::<SparseBitVecBuilder>(None, None, false);
        // RUST_BACKTRACE=full cargo test -- --nocapture
        test_bit_vec_builder_arbtest::<SparseBitVecBuilder>(Some(0xac70e11d00000005), None, false);
    }
}
