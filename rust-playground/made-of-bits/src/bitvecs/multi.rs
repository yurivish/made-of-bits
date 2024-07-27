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

pub struct MultiBuilder<B> {
    /// A BitBuf marking the positions of nonzero bits
    occupancy: B,
    /// A map from 1-bit index to its multiplicity (count).
    multiplicity: BTreeMap<u32, u32>,
}

impl<B: BitVecBuilder> MultiBitVecBuilder for MultiBuilder<B> {
    type Target = Multiplicity<B::Target, true>;

    fn new(universe_size: u32) -> Self {
        Self {
            occupancy: B::new(universe_size),
            multiplicity: BTreeMap::new(),
        }
    }

    fn one_count(&mut self, bit_index: u32, count: u32) {
        if count > 0 {
            self.occupancy.one(bit_index);
            *self.multiplicity.entry(bit_index).or_insert(0) += count;
        }
    }

    fn build(mut self) -> Multiplicity<B::Target, true> {
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
        Multiplicity::new(occupancy, multiplicity)
    }
}

pub struct MultiBuilder2<B> {
    /// A BitBuf marking the positions of nonzero bits
    occupancy: B,
}

impl<B: BitVecBuilder> BitVecBuilder for MultiBuilder2<B> {
    type Target = Multiplicity<B::Target, false>;

    fn new(universe_size: u32) -> Self {
        Self {
            occupancy: B::new(universe_size),
        }
    }

    fn one(&mut self, bit_index: u32) {
        self.occupancy.one(bit_index);
    }

    fn build(mut self) -> Multiplicity<B::Target, false> {
        let occupancy = self.occupancy.build();
        let universe_size = if occupancy.num_ones() > 0 {
            occupancy.num_ones() + 1
        } else {
            0
        };
        let multiplicity = SparseBitVec::new(
            (1..=occupancy.num_ones()).collect::<Vec<_>>().into(),
            universe_size,
        );
        Multiplicity::new(occupancy, multiplicity)
    }
}

#[derive(Clone)]
pub struct Multiplicity<T, const M: bool> {
    occupancy: T,
    multiplicity: SparseBitVec<false>,
    num_ones: u32,
}

impl<T: BitVec, const M: bool> Multiplicity<T, M> {
    fn new(occupancy: T, multiplicity: SparseBitVec<false>) -> Self {
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

    fn to_bitvec(self) -> Multiplicity<T, false> {
        assert!(self.num_ones == self.occupancy.num_ones());
        Multiplicity {
            occupancy: self.occupancy,
            multiplicity: self.multiplicity,
            num_ones: self.num_ones,
        }
    }
}

// These implementations work in the general case
impl<T: BitVec, const M: bool> Multiplicity<T, M> {
    fn rank1(&self, bit_index: u32) -> u32 {
        let n = self.occupancy.rank1(bit_index);
        if n == 0 {
            0
        } else {
            BitVec::select1(&self.multiplicity, n - 1).unwrap()
        }
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

// Implement BitVec
impl<T: BitVec> BitVec for Multiplicity<T, false> {
    fn rank1(&self, bit_index: u32) -> u32 {
        self.rank1(bit_index)
    }

    fn rank0(&self, bit_index: u32) -> u32 {
        self.occupancy.rank0(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.select1(n)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        self.select0(n)
    }

    fn num_ones(&self) -> u32 {
        self.num_ones()
    }

    fn universe_size(&self) -> u32 {
        self.universe_size()
    }
}

// Implement MultiBitVec
impl<T: BitVec> MultiBitVec for Multiplicity<T, true> {
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
        test_bitvec_builder::<MultiBuilder2<DenseBitVecBuilder>>();
        property_test_bitvec_builder::<MultiBuilder2<DenseBitVecBuilder>>(None, None, false);

        test_bitvec_builder::<MultiBuilder2<SparseBitVecBuilder<false>>>();
        property_test_bitvec_builder::<MultiBuilder2<SparseBitVecBuilder<false>>>(
            None, None, false,
        );
    }
}
