use crate::{
    bits::partition_point,
    bitvec::{BitVec, BitVecBuilder},
};

struct SortedArrayBitVecBuilder {
    universe_size: u32,
    ones: Vec<u32>,
}

impl BitVecBuilder for SortedArrayBitVecBuilder {
    type Target = SortedArrayBitVec;

    fn new(universe_size: u32) -> Self {
        Self {
            universe_size,
            ones: Vec::new(),
        }
    }

    fn one_count(&mut self, index: u32, count: u32) {
        assert!(index < self.universe_size);
        for _ in 0..count {
            self.ones.push(index);
        }
    }

    fn build(mut self) -> SortedArrayBitVec {
        self.ones.sort();
        SortedArrayBitVec::new(self.ones.into(), self.universe_size)
    }
}

#[derive(Clone)]
struct SortedArrayBitVec {
    ones: Box<[u32]>,
    universe_size: u32,
    num_ones: u32,
    num_zeros: u32,
    has_multiplicity: bool,
    num_unique_ones: u32,
    num_unique_zeros: u32,
}

impl SortedArrayBitVec {
    fn new(ones: Box<[u32]>, universe_size: u32) -> Self {
        let mut num_unique_ones = 0;
        let mut has_multiplicity = false;
        let mut prev = None;
        for (i, cur) in ones.iter().copied().enumerate() {
            let same = prev == Some(cur);
            has_multiplicity |= same;
            num_unique_ones += if same { 0 } else { 1 };
            if let Some(prev) = prev {
                debug_assert!(prev <= cur, "ones must be sorted")
            }
            prev = Some(cur);
        }

        let num_ones = ones.len() as u32;
        // Zeros are never repeated, so any non-one bits are singleton zeros.
        let num_zeros = universe_size - num_unique_ones;

        Self {
            ones,
            universe_size,
            num_ones,
            num_zeros,
            has_multiplicity,
            num_unique_ones,
            num_unique_zeros: num_zeros,
        }
    }
}

// select: select the k-th occurrence of a 0/1 bit.
// rank: return the number of bits below "universe index" i
// todo: visualize the "stacked" image:
//
// bitvec:
//
//  bits:  1   1  1
// index: 0123456789
//
// multibitvec:
//
//      :         1
//      :  1      1
//  bits:  1   1  1
// index: 0123456789
//  rank: 0022223336
//
// sorted ones:
// [1, 1, 5, 8, 8, 8]

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

    fn num_zeros(&self) -> u32 {
        self.num_zeros
    }

    fn universe_size(&self) -> u32 {
        self.universe_size
    }

    fn has_multiplicity(&self) -> bool {
        self.has_multiplicity
    }

    fn num_unique_zeros(&self) -> u32 {
        self.num_unique_zeros
    }

    fn num_unique_ones(&self) -> u32 {
        self.num_unique_ones
    }

    fn has_rank0(&self) -> bool {
        !self.has_multiplicity()
    }
}

#[cfg(test)]
mod tests {
    use crate::bitvec_test::test_bit_vec_builder;

    use super::*;

    #[test]
    fn test() {
        test_bit_vec_builder::<SortedArrayBitVecBuilder>()
    }
}
