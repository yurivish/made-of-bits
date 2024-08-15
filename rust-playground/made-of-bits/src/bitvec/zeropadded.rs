use crate::bitvec::sparse::SparseBitVec;
use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;
use crate::bitvec::BitVecOf;
use crate::bitvec::MultiBitVec;
use crate::bitvec::MultiBitVecBuilder;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ZeroPadded<T> {
    bv: T,
    universe_size: u32,
    offset: u32, // start offset
}

impl<T: BitVec> ZeroPadded<T> {
    fn new(bv: T, universe_size: u32, offset: u32) -> Self {
        Self {
            bv,
            universe_size,
            offset,
        }
    }
}

// TODO: account for offset and padding (ie. left and right zero padding) in trait impl
impl<T: BitVec> BitVec for ZeroPadded<T> {
    type Builder = ZeroPaddedBuilder<T::Builder>;

    fn rank0(&self, bit_index: u32) -> u32 {
        self.bv.rank0(bit_index)
    }

    fn ranks(&self, bit_index: u32) -> (u32, u32) {
        self.bv.ranks(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.bv.select1(n)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        self.bv.select0(n)
    }

    fn get(&self, bit_index: u32) -> u32 {
        self.bv.get(bit_index)
    }

    fn num_zeros(&self) -> u32 {
        self.bv.num_zeros()
    }

    fn rank1_batch(&self, out: &mut Vec<u32>, bit_indices: &[u32]) {
        self.bv.rank1_batch(out, bit_indices)
    }

    fn rank1(&self, bit_index: u32) -> u32 {
        self.bv.rank1(bit_index)
    }

    fn universe_size(&self) -> u32 {
        self.bv.universe_size()
    }

    fn num_ones(&self) -> u32 {
        self.bv.num_ones()
    }
}

#[derive(Default, Clone)]
pub struct ZeroPaddedOptions<O: Default + Clone> {
    universe_size: u32,
    offset: u32,
    options: O,
}

#[derive(Clone)]
struct ZeroPaddedBuilder<B: BitVecBuilder> {
    universe_size: u32,
    offset: u32,
    builder: B,
}

impl<B: BitVecBuilder> BitVecBuilder for ZeroPaddedBuilder<B> {
    type Target = ZeroPadded<B::Target>;
    type Options = ZeroPaddedOptions<B::Options>;

    fn new(universe_size: u32, options: Self::Options) -> Self {
        Self {
            universe_size,
            offset: options.offset,
            builder: B::new(universe_size - options.offset, options.options),
        }
    }

    fn one(&mut self, bit_index: u32) {
        self.builder.one(bit_index - self.offset);
    }

    fn build(self) -> Self::Target {
        ZeroPadded::new(self.builder.build(), self.universe_size, self.offset)
    }
}

// TODO
#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::bitvec::array::ArrayBitVecBuilder;
    // use crate::bitvec::BitVecBuilderOf;
    // use crate::{bitvec::dense::DenseBitVecBuilder, bitvec::test::*};

    // #[test]
    // fn multibitvec_interface() {
    //     test_multibitvec_builder::<MultiBuilder<DenseBitVecBuilder>>();
    //     test_multibitvec_builder::<MultiBuilder<BitVecBuilderOf<ArrayBitVecBuilder>>>();
    // }
}
