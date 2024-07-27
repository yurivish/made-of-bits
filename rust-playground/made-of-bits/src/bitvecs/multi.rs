use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::{
    bitbuf::BitBuf,
    bits::{one_mask, partition_point},
    bitvec::{BitVec, BitVecBuilder},
    bitvecs::{dense::DenseBitVec, sparse::SparseBitVec},
    intbuf::IntBuf,
};
use std::collections::BTreeMap;
use std::ops::BitAndAssign;

pub struct MultiBuilder<B> {
    /// A BitBuf marking the positions of nonzero bits
    // occupancy: BitBuf,
    occupancy: B,
    /// A map from 1-bit index to its multiplicity (count).
    multiplicity: BTreeMap<u32, u32>,
}

impl<B: BitVecBuilder> MultiBuilder<B> {
    fn new(universe_size: u32) -> Self {
        Self {
            occupancy: B::new(universe_size),
            multiplicity: BTreeMap::new(),
        }
    }
}

impl<B: BitVecBuilder> BitVecBuilder for MultiBuilder<B> {
    type Target = Multi<B::Target, false>;

    fn new(universe_size: u32) -> Self {
        Self::new(universe_size)
    }

    fn one(&mut self, bit_index: u32) {
        self.one_count(bit_index, 1);
    }

    fn build(mut self) -> Multi<B::Target, false> {
        // Since this is a BitVecBuilder and not a MultiBitVecBuilder, set all counts to 1,
        // then call the MultiBitVecBuilder build method.
        // Since one_count counts must exceed zero, all values in this map are positive and
        // can be replaced with 1 to satisfy the requirements of a bitvector without multiplicity.
        for x in self.multiplicity.values_mut() {
            *x = 1;
        }
        MultiBitVecBuilder::build(self).to_bitvec()
    }
}

impl<B: BitVecBuilder> MultiBitVecBuilder for MultiBuilder<B> {
    type Target = Multi<B::Target, true>;

    fn new(universe_size: u32) -> Self {
        Self::new(universe_size)
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        if count > 0 {
            self.occupancy.one(bit_index);
            *self.multiplicity.entry(bit_index).or_insert(0) += count;
        }
    }

    fn build(mut self) -> Multi<B::Target, true> {
        // Sort the map keys and values in ascending order of 1-bit index
        let mut kv: Vec<_> = self.multiplicity.into_iter().collect();
        kv.sort_by_key(|(k, v)| *k);

        // Construct a parallel array of cumulative counts
        let mut cumulative_counts: Vec<_> = kv.into_iter().map(|(k, v)| v).collect();
        let mut acc = 0;
        for x in cumulative_counts.iter_mut() {
            acc += *x;
            *x = acc;
        }

        let occupancy = self.occupancy.build();
        let universe_size = if acc > 0 { acc + 1 } else { 0 };
        let multiplicity = SparseBitVec::new(cumulative_counts.into(), universe_size);
        Multi::new(occupancy, multiplicity)
    }
}

#[derive(Clone)]
pub struct Multi<T, const M: bool> {
    occupancy: T,
    multiplicity: SparseBitVec,
    num_ones: u32,
}

impl<T: BitVec, const M: bool> Multi<T, M> {
    fn new(occupancy: T, multiplicity: SparseBitVec) -> Self {
        let n = multiplicity.num_ones();
        let num_ones = if n == 0 {
            0
        } else {
            BitVec::select1(&multiplicity, n - 1).unwrap()
        };
        Self {
            occupancy,
            multiplicity,
            num_ones,
        }
    }
}

// Cast from a multiplicity-having bitvec that implements `MultiBitVec` to a non-multiplicity-having version that implements `BitVec`
impl<T: BitVec> Multi<T, true> {
    fn to_bitvec(self) -> Multi<T, false> {
        assert!(self.num_ones == self.occupancy.num_ones());
        Multi {
            occupancy: self.occupancy,
            multiplicity: self.multiplicity,
            num_ones: self.num_ones,
        }
    }
}

// These implementations work in the general case
impl<T: BitVec> BitVec for Multi<T, false> {
    fn rank1(&self, bit_index: u32) -> u32 {
        let n = self.occupancy.rank1(bit_index);
        if n == 0 {
            0
        } else {
            BitVec::select1(&self.multiplicity, n - 1).unwrap()
        }
    }

    fn rank0(&self, bit_index: u32) -> u32 {
        self.occupancy.rank0(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        let i = BitVec::rank1(&self.multiplicity, n + 1);
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
}

impl<T: BitVec> MultiBitVec for Multi<T, true> {
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

    fn num_unique_ones(&self) -> u32 {
        self.occupancy.num_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bitvec_test::*,
        bitvecs::{dense::DenseBitVecBuilder, sparse::SparseBitVecBuilder},
    };

    #[test]
    fn test() {
        test_bitvec_builder::<MultiBuilder<DenseBitVecBuilder>>();
        property_test_bitvec_builder::<MultiBuilder<DenseBitVecBuilder>>(None, None, false);

        test_bitvec_builder::<MultiBuilder<SparseBitVecBuilder>>();
        property_test_bitvec_builder::<MultiBuilder<SparseBitVecBuilder>>(None, None, false);
    }
}
