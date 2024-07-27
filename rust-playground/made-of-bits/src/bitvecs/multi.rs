use crate::{
    bitbuf::BitBuf,
    bits::{one_mask, partition_point},
    bitvec::{BitVec, BitVecBuilder},
    bitvecs::{dense::DenseBitVec, sparse::SparseBitVec},
    intbuf::IntBuf,
};
use std::collections::BTreeMap;
use std::ops::BitAndAssign;

pub struct MultiBitVecBuilder {
    buf: BitBuf,
    // Stores a mal from 1-bit index to its multiplicity (count).
    ones: BTreeMap<u32, u32>,
}

impl BitVecBuilder for MultiBitVecBuilder {
    type Target = MultiBitVec;

    fn new(universe_size: u32) -> Self {
        Self {
            buf: BitBuf::new(universe_size),
            ones: BTreeMap::new(),
        }
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        if count == 0 {
            return;
        }
        assert!(bit_index < self.buf.universe_size());
        self.buf.set_one(bit_index);
        *self.ones.entry(bit_index).or_insert(0) += count;
    }

    fn build(mut self) -> MultiBitVec {
        // Sort the map keys and values in ascending order of 1-bit index
        // Note: We could instead iterate over the set bits of `buf` in ascending order,
        // resulting in a linear-time "sort".
        let mut kv: Vec<_> = self.ones.into_iter().collect();
        kv.sort_by_key(|(k, v)| *k);

        // Construct a parallel array of cumulative counts
        let mut cumulative_counts: Vec<_> = kv.into_iter().map(|(k, v)| v).collect();
        let mut acc = 0;
        for x in cumulative_counts.iter_mut() {
            acc += *x;
            *x = acc;
        }

        let occupancy = DenseBitVec::new(self.buf, 10, 10);
        let universe_size = if acc > 0 { acc + 1 } else { 0 };
        let multiplicity = SparseBitVec::new(cumulative_counts.into(), universe_size);
        MultiBitVec::new(occupancy, multiplicity)
    }
}

// todo

#[derive(Clone)]
pub struct MultiBitVec {
    occupancy: DenseBitVec,
    multiplicity: SparseBitVec,
    num_ones: u32,
}

impl MultiBitVec {
    fn new(occupancy: DenseBitVec, multiplicity: SparseBitVec) -> Self {
        let n = multiplicity.num_ones();
        let num_ones = if n == 0 {
            0
        } else {
            multiplicity.select1(n - 1).unwrap()
        };
        Self {
            occupancy,
            multiplicity,
            num_ones,
        }
    }
}

impl BitVec for MultiBitVec {
    fn rank1(&self, bit_index: u32) -> u32 {
        let n = self.occupancy.rank1(bit_index);
        if n == 0 {
            0
        } else {
            self.multiplicity.select1(n - 1).unwrap()
        }
    }

    fn rank0(&self, bit_index: u32) -> u32 {
        self.occupancy.rank0(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        let i = self.multiplicity.rank1(n + 1);
        self.occupancy.select1(i)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        self.occupancy.select0(n)
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn universe_size(&self) -> u32 {
        self.occupancy.universe_size()
    }

    fn num_unique_zeros(&self) -> u32 {
        self.occupancy.num_zeros()
    }

    fn num_unique_ones(&self) -> u32 {
        self.occupancy.num_ones()
    }

    fn get(&self, bit_index: u32) -> u32 {
        assert!(bit_index < self.universe_size());
        self.rank1(bit_index + 1) - self.rank1(bit_index)
    }

    fn has_rank0(&self) -> bool {
        true
    }

    fn has_select0(&self) -> bool {
        !self.has_multiplicity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec_test::*;

    #[test]
    fn test() {
        test_bitvvec_builder::<MultiBitVecBuilder>();
        test_bitvec_builder_arbtest::<MultiBitVecBuilder>(None, None, false);
        // RUST_BACKTRACE=full cargo test -- --nocapture
        // test_bit_vec_builder_arbtest::<MultiBitVecBuilder>(Some(0xac70e11d00000005), None, false);
    }
}
