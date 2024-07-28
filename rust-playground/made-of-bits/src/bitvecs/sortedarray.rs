use crate::bitvec::DefaultBitVec;
use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::bitvec::{BitVec, BitVecBuilder};

pub struct SortedArrayBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl MultiBitVecBuilder for SortedArrayBitVecBuilder {
    type Target = SortedArrayBitVec;

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

// Implement BitVec for BitVecOf<SortedArrayBitVec>
impl DefaultBitVec for SortedArrayBitVec {}

impl MultiBitVec for SortedArrayBitVec {
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

    fn num_unique_ones(&self) -> u32 {
        self.num_unique_ones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::BitVecBuilderOf;
    use crate::bitvec_test::*;

    #[test]
    fn test() {
        test_bitvec_builder::<BitVecBuilderOf<SortedArrayBitVecBuilder>>();
        property_test_bitvec_builder::<BitVecBuilderOf<SortedArrayBitVecBuilder>>(
            None, None, false,
        );
    }
}
