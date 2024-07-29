use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::bitvec::BitVecOf;
use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use crate::bitvecs::sparse::SparseBitVec;
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

    // While `MultiBitVec`s do not usually support rank0 and select0,
    // this one does since the zeros of the occupancy bitvec are the
    // zeros of the `Multi`.
    pub fn rank0(&self, bit_index: u32) -> u32 {
        self.occupancy.rank0(bit_index)
    }

    pub fn select0(&self, n: u32) -> Option<u32> {
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
        if n == u32::MAX {
            return None;
        }
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

    fn ones(&mut self, bit_index: u32, count: u32) {
        if count > 0 {
            self.occupancy.one(bit_index);
            *self.multiplicity.entry(bit_index).or_insert(0) += count;
        }
    }

    fn build(self) -> Multi<B::Target> {
        // Sort the map keys and values in ascending order of 1-bit index
        let mut kv: Vec<_> = self.multiplicity.into_iter().collect();
        kv.sort_by_key(|(k, _v)| *k);

        // Construct a parallel array of cumulative counts
        let mut cumulative_counts: Vec<_> = kv.into_iter().map(|(_k, v)| v).collect();
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
    use crate::bitvecs::array::ArrayBitVecBuilder;
    use crate::{bitvec_test::*, bitvecs::dense::DenseBitVecBuilder};

    #[test]
    fn test_interface() {
        test_multi_bitvec_builder::<MultiBuilder<DenseBitVecBuilder>>();
        test_multi_bitvec_builder::<MultiBuilder<BitVecBuilderOf<ArrayBitVecBuilder>>>();
    }
}
