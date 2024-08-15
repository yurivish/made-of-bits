use crate::bitblock::BitBlock;
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
    start: u32,
    end: u32,
}

impl<T: BitVec> ZeroPadded<T> {
    fn new(bv: T, universe_size: u32, pad_left: u32, pad_right: u32) -> Self {
        Self {
            bv,
            universe_size,
            start: pad_left,                // index of the first non-padding bit
            end: universe_size - pad_right, // one past the index of the last padding bit
        }
    }
}

impl<T: BitVec> BitVec for ZeroPadded<T> {
    type Builder = ZeroPaddedBuilder<T::Builder>;

    fn rank0(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size() {
            return self.num_zeros();
        }
        if bit_index < self.start {
            return bit_index;
        }
        self.start + self.bv.rank0(bit_index - self.start) + bit_index.saturating_sub(self.end)
    }

    fn ranks(&self, bit_index: u32) -> (u32, u32) {
        self.bv.ranks(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.bv.select1(n).map(|i| self.start + i)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        if n < self.start {
            return Some(n);
        }
        if n >= self.num_zeros() {
            return None;
        }
        self.bv
            .select0(n - self.start)
            .map(|i| self.start + i)
            .or_else(|| Some(n + self.bv.num_ones()))
    }

    fn get(&self, bit_index: u32) -> u32 {
        if bit_index < self.start || bit_index >= self.end {
            return 0;
        }
        self.bv.get(bit_index - self.start)
    }

    fn num_zeros(&self) -> u32 {
        let padding_zeros = self.universe_size() - self.end + self.start;
        self.bv.num_zeros() + padding_zeros
    }

    fn rank1_batch(&self, out: &mut Vec<u32>, bit_indices: &[u32]) {
        self.bv.rank1_batch(out, bit_indices)
    }

    fn rank1(&self, bit_index: u32) -> u32 {
        if bit_index < self.start {
            return 0;
        }
        self.bv.rank1(bit_index - self.start)
    }

    fn universe_size(&self) -> u32 {
        self.universe_size
    }

    fn num_ones(&self) -> u32 {
        self.bv.num_ones()
    }
}

#[derive(Default, Clone)]
pub struct ZeroPaddedOptions<O: Default + Clone> {
    pad_left: u32,
    pad_right: u32,
    options: O, // options for the inner bitvector
}

#[derive(Clone)]
pub struct ZeroPaddedBuilder<B: BitVecBuilder> {
    universe_size: u32,
    options: ZeroPaddedOptions<B::Options>,
    builder: B,
}

impl<B: BitVecBuilder> BitVecBuilder for ZeroPaddedBuilder<B> {
    type Target = ZeroPadded<B::Target>;
    type Options = ZeroPaddedOptions<B::Options>;

    fn new(universe_size: u32, options: Self::Options) -> Self {
        Self {
            universe_size,
            options: options.clone(),
            builder: B::new(
                universe_size - options.pad_left - options.pad_right,
                options.options,
            ),
        }
    }

    fn one(&mut self, bit_index: u32) {
        assert!(bit_index >= self.options.pad_left);
        assert!(bit_index < self.universe_size - self.options.pad_right);
        self.builder.one(bit_index - self.options.pad_left);
    }

    fn build(self) -> Self::Target {
        ZeroPadded::new(
            self.builder.build(),
            self.universe_size,
            self.options.pad_left,
            self.options.pad_right,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::array::ArrayBitVecBuilder;
    use crate::bitvec::BitVecBuilderOf;
    use crate::{bitvec::dense::DenseBitVecBuilder, bitvec::test::*};

    #[test]
    fn zeropadded_interface() {
        // test different collections of ones.
        // we test 10 and 94, which are the first
        // and last elements allowed to be nonzero
        // under the provided padding and universe size
        // (universe size 100, padding left 10, padding right 5).
        let oneses = [
            vec![],
            vec![10, 15, 19, 80, 94],
            vec![10],
            vec![50],
            vec![94],
        ];
        for ones in oneses {
            let options = test_bitvec::<ZeroPaddedBuilder<DenseBitVecBuilder>>(
                100,
                ZeroPaddedOptions {
                    pad_left: 10,
                    pad_right: 5,
                    options: Default::default(),
                },
                ones.clone(),
            );
            test_bitvec::<ZeroPaddedBuilder<BitVecBuilderOf<ArrayBitVecBuilder>>>(
                100,
                ZeroPaddedOptions {
                    pad_left: 10,
                    pad_right: 5,
                    options: Default::default(),
                },
                ones.clone(),
            );
        }
    }
}
