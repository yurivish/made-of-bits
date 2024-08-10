use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;

// NOTE: This is just a renamed ArrayBitVec at the moment.
// The goal is to implement the compressed bit vector as
// described in "Fast, Small, Simple Rank/Select on Bitmaps":
// https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf
// See also: https://observablehq.com/d/5370347688e58b4d

#[derive(Clone)]
pub struct CompressedBitVec {
    ones: Box<[u32]>,
    universe_size: u32,
    num_ones: u32,
    num_unique_ones: u32,
}

impl CompressedBitVec {
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

impl BitVec for CompressedBitVec {
    type Builder = CompressedBitVecBuilder;

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

#[derive(Clone)]
pub struct CompressedBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl BitVecBuilder for CompressedBitVecBuilder {
    type Target = CompressedBitVec;
    type Options = ();

    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }

    fn one(&mut self, bit_index: u32) {
        assert!(bit_index < self.universe_size);
        self.ones.push(bit_index);
    }

    fn build(mut self) -> CompressedBitVec {
        self.ones.sort();
        CompressedBitVec::new(self.ones.into(), self.universe_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::test::*;

    #[test]
    fn multibitvec_interface() {
        test_bitvec_builder::<CompressedBitVecBuilder>();
    }
}
