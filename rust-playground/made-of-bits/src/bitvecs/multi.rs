use crate::bitvec::BitVecOf;
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

#[derive(Clone)]
pub struct Multi<T> {
    occupancy: T,
    multiplicity: BitVecOf<SparseBitVec>,
    num_ones: u32,
}

impl<T: BitVec> Multi<T> {
    fn new(occupancy: T, multiplicity: BitVecOf<SparseBitVec>) -> Self {
        // `multiplicity` represents the cumulative multiplicity of each 1-bit, so
        // the number of ones in this bitvector is the index of its last 1-bit.
        let num_ones = match multiplicity.num_ones() {
            0 => 0,
            n => multiplicity.select1(n - 1).unwrap(),
        };
        Self {
            occupancy,
            multiplicity,
            num_ones,
        }
    }

    fn rank0(&self, bit_index: u32) -> u32 {
        self.occupancy.rank0(bit_index)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        self.occupancy.select0(n)
    }
}

impl<T: BitVec> MultiBitVec for Multi<T> {
    fn rank1(&self, bit_index: u32) -> u32 {
        match self.occupancy.rank1(bit_index) {
            0 => 0,
            n => self.multiplicity.select1(n - 1).unwrap(),
        }
    }

    fn select1(&self, n: u32) -> Option<u32> {
        let i = self.multiplicity.rank1(n + 1);
        self.occupancy.select1(i)
    }

    fn num_ones(&self) -> u32 {
        self.num_ones
    }

    fn num_unique_ones(&self) -> u32 {
        self.occupancy.num_ones()
    }

    fn universe_size(&self) -> u32 {
        self.occupancy.universe_size()
    }
}

pub struct MultiBuilder<B> {
    /// BitBuf marking the positions of nonzero bits
    occupancy: B,
    /// Map from 1-bit index to its multiplicity (count).
    multiplicity: BTreeMap<u32, u32>,
}

impl<B: BitVecBuilder> MultiBitVecBuilder for MultiBuilder<B> {
    type Target = Multi<B::Target>;

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

    fn build(mut self) -> Multi<B::Target> {
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
        Multi::new(occupancy, BitVecOf::new(multiplicity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::BitVecBuilderOf;
    use crate::{
        bitvec_test::*,
        bitvecs::{dense::DenseBitVecBuilder, sparse::SparseBitVecBuilder},
    };

    #[test]
    fn test() {
        test_bitvec_builder::<BitVecBuilderOf<MultiBuilder<DenseBitVecBuilder>>>();
        property_test_bitvec_builder::<BitVecBuilderOf<MultiBuilder<DenseBitVecBuilder>>>(
            None, None, false,
        );

        test_bitvec_builder::<BitVecBuilderOf<MultiBuilder<BitVecBuilderOf<SparseBitVecBuilder>>>>(
        );
        property_test_bitvec_builder::<
            BitVecBuilderOf<MultiBuilder<BitVecBuilderOf<SparseBitVecBuilder>>>,
        >(None, None, false);
    }
}
