use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::bitvec::{BitVec, BitVecBuilder};

pub struct SortedArrayBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl SortedArrayBitVecBuilder {
    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }
}

impl BitVecBuilder for SortedArrayBitVecBuilder {
    type Target = SortedArrayBitVec;

    fn new(universe_size: u32) -> Self {
        Self::new(universe_size)
    }

    fn one(&mut self, bit_index: u32) {
        self.one_count(bit_index, 1);
    }

    fn build(mut self) -> SortedArrayBitVec {
        // Sort the vec before deduplicating any repeated 1-bits.
        // The sort in `MultiBitVecBuilder::build` will be redundant
        // but likely linear-time due to optimizations in the sorting algorithm.
        self.ones.sort();
        self.ones.dedup();
        MultiBitVecBuilder::build(self)
    }
}

impl MultiBitVecBuilder for SortedArrayBitVecBuilder {
    type Target = SortedArrayBitVec;

    fn new(universe_size: u32) -> Self {
        Self::new(universe_size)
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        assert!(bit_index < self.universe_size);
        for _ in 0..count {
            self.ones.push(bit_index);
        }
    }

    fn build(mut self) -> SortedArrayBitVec {
        self.ones.sort();
        SortedArrayBitVec::new(self.ones.into(), self.universe_size)
    }
}

#[derive(Clone)]
pub struct SortedArrayBitVec {
    ones: Box<[u32]>,
    universe_size: u32,
    num_ones: u32,
    num_unique_ones: u32,
}

impl SortedArrayBitVec {
    fn new(ones: Box<[u32]>, universe_size: u32) -> Self {
        let mut num_unique_ones = 0;
        let mut prev = None;

        for cur in ones.iter().copied() {
            let same = prev == Some(cur);
            num_unique_ones += if same { 0 } else { 1 };
            if let Some(prev) = prev {
                debug_assert!(prev <= cur, "ones must be sorted")
            }
            prev = Some(cur);
        }

        // Zeros are never repeated, so any non-one bits are singleton zeros.
        let num_zeros = universe_size - num_unique_ones;
        let num_ones = ones.len() as u32;

        Self {
            ones,
            universe_size,
            num_ones,
            num_unique_ones,
        }
    }

    pub(crate) fn ones(&self) -> &Box<[u32]> {
        &self.ones
    }
}

// todo
impl BitVec for SortedArrayBitVec {
    fn rank1(&self, bit_index: u32) -> u32 {
        self.ones.partition_point(|x| *x < bit_index) as u32
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.ones.get(n as usize).copied()
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn universe_size(&self) -> u32 {
        self.universe_size
    }
}

impl MultiBitVec for SortedArrayBitVec {
    fn get(&self, bit_index: u32) -> u32 {
        BitVec::get(self, bit_index)
    }

    fn rank1(&self, bit_index: u32) -> u32 {
        BitVec::rank1(self, bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        BitVec::select1(self, n)
    }

    fn universe_size(&self) -> u32 {
        BitVec::universe_size(self)
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
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
        test_bitvec_builder::<SortedArrayBitVecBuilder>();
        property_test_bitvec_builder::<SortedArrayBitVecBuilder>(None, None, false);
    }
}
