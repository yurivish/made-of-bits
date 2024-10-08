use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;

/// Implements a MultiBitVec backed by an array of 1-bits.
/// Takes 32 bits per 1-bit.
#[derive(Clone)]
pub struct ArrayBitVec {
    ones: Box<[u32]>,
    universe_size: u32,
    num_ones: u32,
    num_unique_ones: u32,
}

impl ArrayBitVec {
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

        let num_ones = ones.len() as u32;

        Self {
            ones,
            universe_size,
            num_ones,
            num_unique_ones,
        }
    }
}

impl MultiBitVec for ArrayBitVec {
    type Builder = ArrayBitVecBuilder;

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

#[derive(Clone)]
pub struct ArrayBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl MultiBitVecBuilder for ArrayBitVecBuilder {
    type Target = ArrayBitVec;
    type Options = ();

    fn new(universe_size: u32, options: Self::Options) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }

    fn ones(&mut self, bit_index: u32, count: u32) {
        assert!(bit_index < self.universe_size);
        for _ in 0..count {
            self.ones.push(bit_index);
        }
    }

    fn build(mut self) -> ArrayBitVec {
        self.ones.sort();
        ArrayBitVec::new(self.ones.into(), self.universe_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::test::*;

    #[test]
    fn multibitvec_interface() {
        test_multibitvec_builder::<ArrayBitVecBuilder>();
    }
}
