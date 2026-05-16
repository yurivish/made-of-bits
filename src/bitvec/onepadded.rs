use crate::bitvec::BitVec;
use crate::bitvec::BitVecBuilder;

/// Wraps a `BitVec` with implicit 1-bits on the right. Positions `[0, inner_len)` come
/// from the inner BitVec; positions `[inner_len, universe_size)` are all 1-bits with
/// no storage cost. Used by the Huffman wavelet matrix so short codes sort to the END
/// of every level — the opposite of [`ZeroPadded<T>`](super::zeropadded::ZeroPadded).
#[derive(Clone)]
pub struct OnePadded<T> {
    bv: T,
    universe_size: u32,
}

impl<T: BitVec> OnePadded<T> {
    pub fn new(bv: T, universe_size: u32) -> Self {
        assert!(
            bv.universe_size() <= universe_size,
            "inner universe {} exceeds OnePadded universe {universe_size}",
            bv.universe_size(),
        );
        Self { bv, universe_size }
    }

    /// One past the last inner-region bit; also the start of the implicit-1s region.
    pub fn inner_len(&self) -> u32 {
        self.bv.universe_size()
    }

    pub fn inner(&self) -> &T {
        &self.bv
    }
}

impl<T: BitVec> BitVec for OnePadded<T> {
    type Builder = OnePaddedBuilder<T::Builder>;

    fn rank1(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size {
            return self.num_ones();
        }
        let il = self.inner_len();
        if bit_index <= il {
            self.bv.rank1(bit_index)
        } else {
            self.bv.num_ones() + (bit_index - il)
        }
    }

    fn rank0(&self, bit_index: u32) -> u32 {
        if bit_index >= self.universe_size {
            return self.num_zeros();
        }
        let il = self.inner_len();
        if bit_index <= il {
            self.bv.rank0(bit_index)
        } else {
            // Padding region has no 0-bits.
            self.bv.num_zeros()
        }
    }

    fn select1(&self, n: u32) -> Option<u32> {
        let inner_ones = self.bv.num_ones();
        if n < inner_ones {
            return self.bv.select1(n);
        }
        let pos = self.inner_len() + (n - inner_ones);
        (pos < self.universe_size).then_some(pos)
    }

    fn select0(&self, n: u32) -> Option<u32> {
        // All 0-bits live in the inner bv.
        self.bv.select0(n)
    }

    fn get(&self, bit_index: u32) -> u32 {
        if bit_index < self.inner_len() { self.bv.get(bit_index) } else { 1 }
    }

    fn universe_size(&self) -> u32 {
        self.universe_size
    }

    fn num_ones(&self) -> u32 {
        self.bv.num_ones() + (self.universe_size - self.inner_len())
    }

    fn num_zeros(&self) -> u32 {
        self.bv.num_zeros()
    }

    fn all_ones_from(&self) -> u32 {
        self.inner_len()
    }

    fn rank1_batch(&self, bit_indices: &mut [u32]) {
        // Inner queries (those at positions <= inner_len) get rank1_batch'd through a
        // side buffer in order. This preserves the monotone-non-decreasing precondition
        // that DenseBitVec/SparseBitVec's chunking arithmetic needs — interleaving
        // padding-region answers in place would break it. Padding-region answers are
        // then computed directly: rank in padding is inner_ones + (bit_index - il),
        // since every padding bit is a 1.
        let il = self.inner_len();
        let inner_ones = self.bv.num_ones();
        let total_ones = self.num_ones();
        let mut inner_buf: Vec<u32> =
            bit_indices.iter().copied().filter(|&v| v <= il).collect();
        self.bv.rank1_batch(&mut inner_buf);
        let mut inner_iter = inner_buf.into_iter();
        for v in bit_indices.iter_mut() {
            *v = if *v <= il {
                inner_iter.next().unwrap()
            } else if *v >= self.universe_size {
                total_ones
            } else {
                inner_ones + (*v - il)
            };
        }
    }
}

#[derive(Default, Clone)]
pub struct OnePaddedOptions<O: Default + Clone> {
    /// The universe of the inner bitvec. Positions in
    /// `[inner_universe_size, universe_size)` are implicit 1-bits.
    pub inner_universe_size: u32,
    pub inner_options: O,
}

#[derive(Clone)]
pub struct OnePaddedBuilder<B: BitVecBuilder> {
    universe_size: u32,
    inner_universe_size: u32,
    builder: B,
}

impl<B: BitVecBuilder> BitVecBuilder for OnePaddedBuilder<B> {
    type Target = OnePadded<B::Target>;
    type Options = OnePaddedOptions<B::Options>;

    fn new(universe_size: u32, options: Self::Options) -> Self {
        assert!(
            options.inner_universe_size <= universe_size,
            "inner universe {} exceeds OnePadded universe {universe_size}",
            options.inner_universe_size,
        );
        Self {
            universe_size,
            inner_universe_size: options.inner_universe_size,
            builder: B::new(options.inner_universe_size, options.inner_options),
        }
    }

    fn one(&mut self, bit_index: u32) {
        assert!(
            bit_index < self.inner_universe_size,
            "builder.one() must address inner bits; padding region is implicit 1s",
        );
        self.builder.one(bit_index);
    }

    fn build(self) -> Self::Target {
        OnePadded::new(self.builder.build(), self.universe_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitvec::array::ArrayBitVecBuilder;
    use crate::bitvec::dense::DenseBitVecBuilder;
    use crate::bitvec::test::assert_invariants;
    use crate::bitvec::BitVecBuilderOf;
    use crate::bitvec::MultiBitVecBuilder;

    /// Build an `ArrayBitVec` reference for `OnePadded`. The expanded ones list is
    /// `inner_ones ++ [inner_universe..universe_size)` — the implicit padding 1-bits
    /// made explicit.
    fn array_reference(universe_size: u32, inner_universe_size: u32, inner_ones: &[u32]) -> impl BitVec {
        let mut all_ones: Vec<u32> = inner_ones.to_vec();
        all_ones.extend(inner_universe_size..universe_size);
        BitVecBuilderOf::<ArrayBitVecBuilder>::from_ones(universe_size, Default::default(), &all_ones)
    }

    /// Build a `OnePadded` using the given inner-builder type and cross-check it
    /// position-for-position against the expanded `ArrayBitVec` reference.
    fn check<B: BitVecBuilder>(
        universe_size: u32,
        inner_universe_size: u32,
        inner_ones: &[u32],
    ) {
        let a = array_reference(universe_size, inner_universe_size, inner_ones);
        let b = OnePaddedBuilder::<B>::from_ones(
            universe_size,
            OnePaddedOptions {
                inner_universe_size,
                inner_options: Default::default(),
            },
            inner_ones,
        );

        assert_eq!(a.num_zeros(), b.num_zeros());
        assert_eq!(a.num_ones(), b.num_ones());
        assert_eq!(a.universe_size(), b.universe_size());

        for i in 0..universe_size {
            assert_eq!(a.get(i), b.get(i), "get({i})");
        }
        for i in 0..universe_size.saturating_add(10) {
            assert_eq!(a.rank1(i), b.rank1(i), "rank1({i})");
            assert_eq!(a.rank0(i), b.rank0(i), "rank0({i})");
            assert_eq!(a.select1(i), b.select1(i), "select1({i})");
            assert_eq!(a.select0(i), b.select0(i), "select0({i})");
        }

        // Batch rank cross-check.
        let mut indices: Vec<u32> = (0..universe_size).step_by(3).collect();
        let expected: Vec<u32> = indices.iter().map(|&i| a.rank1(i)).collect();
        b.rank1_batch(&mut indices);
        assert_eq!(indices, expected, "rank1_batch");

        assert_invariants(&b);
    }

    /// `OnePadded` with various (inner_universe, universe) splits, applied to
    /// arbitrary inner `ones` collections. The padding region's implicit 1-bits
    /// are added to the ArrayBitVec reference so the cross-check is meaningful.
    #[test]
    fn onepadded_interface() {
        // (universe_size, inner_universe_size, inner_ones)
        let configs: &[(u32, u32, &[u32])] = &[
            (100, 100, &[]),                     // no padding, empty inner
            (100, 100, &[0, 50, 99]),            // no padding, sparse inner
            (100, 90, &[]),                      // 10-bit padding, empty inner
            (100, 90, &[0, 45, 89]),             // 10-bit padding, sparse inner
            (100, 50, &[]),                      // 50-bit padding, empty inner
            (100, 50, &[0, 25, 49]),             // 50-bit padding, sparse inner
            (100, 0, &[]),                       // all padding
            (200, 130, &[10, 64, 65, 100, 129]), // crosses block boundaries
        ];
        for &(universe_size, inner_universe_size, inner_ones) in configs {
            check::<DenseBitVecBuilder>(universe_size, inner_universe_size, inner_ones);
            check::<BitVecBuilderOf<ArrayBitVecBuilder>>(
                universe_size,
                inner_universe_size,
                inner_ones,
            );
        }
    }
}
