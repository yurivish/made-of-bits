use crate::{
    bitbuf::BitBuf,
    bits::{one_mask, partition_point},
    bitvec::{BitVec, BitVecBuilder},
    bitvecs::{dense::DenseBitVec, sparse::SparseBitVec},
    intbuf::IntBuf,
};
use std::collections::BTreeMap;
use std::ops::BitAndAssign;

pub struct MultiBuilder {
    /// A BitBuf marking the positions of nonzero bits
    occupancy: BitBuf,
    /// A map from 1-bit index to its multiplicity (count).
    ones: BTreeMap<u32, u32>,
}

impl BitVecBuilder for MultiBuilder {
    type Target = Multi;

    fn new(universe_size: u32) -> Self {
        Self {
            occupancy: BitBuf::new(universe_size),
            ones: BTreeMap::new(),
        }
    }

    fn one(&mut self, bit_index: u32) {
        assert!(bit_index < self.occupancy.universe_size());
        self.occupancy.set_one(bit_index);
        *self.ones.entry(bit_index).or_insert(0) += 1;
    }

    fn build(mut self) -> Multi {
        // Sort the map keys and values in ascending order of 1-bit index
        // Note: We could instead iterate over the set bits of `occupancy` in ascending order,
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

        let occupancy = DenseBitVec::new(self.occupancy, 10, 10);
        let universe_size = if acc > 0 { acc + 1 } else { 0 };
        let multiplicity = SparseBitVec::new(cumulative_counts.into(), universe_size);
        Multi::new(occupancy, multiplicity)
    }
}

#[derive(Clone)]
pub struct Multi {
    // todo: parameterize?
    occupancy: DenseBitVec,
    multiplicity: SparseBitVec,
    num_ones: u32,
}

impl Multi {
    fn num_unique_zeros(&self) -> u32 {
        self.occupancy.num_zeros()
    }

    fn num_unique_ones(&self) -> u32 {
        self.occupancy.num_ones()
    }
}

impl Multi {
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

impl BitVec for Multi {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec_test::*;

    #[test]
    fn test() {
        test_bitvec_builder::<MultiBuilder>();
        property_test_bitvec_builder::<MultiBuilder>(None, None, false);
    }
}
