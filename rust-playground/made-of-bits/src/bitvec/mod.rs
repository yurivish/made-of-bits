pub mod array;
pub mod dense;
pub mod multi;
pub mod rle;
pub mod sparse;
#[cfg(test)]
mod test;
pub mod zeropadded;

use crate::{bitbuf::BitBuf, bits::partition_point};

use self::multi::MultiBuilder;

pub trait BitVec: Clone {
    type Builder: BitVecBuilder<Target = Self>;

    /// The number of 1-bits below `bit_index`
    fn rank1(&self, bit_index: u32) -> u32;

    /// The number of 0-bits below `bit_index`
    fn rank0(&self, bit_index: u32) -> u32 {
        // The implementation below is valid for bit vectors without multiplicity,
        // since otherwise the subtraction in the second branch can go negative.
        if bit_index >= self.universe_size() {
            self.num_zeros()
        } else {
            bit_index - self.rank1(bit_index)
        }
    }

    /// `(rank0(bit_index), rank1(bit_index))`
    /// This means that if x = ranks(index), x.0 is rank0 and x.1 is rank1.
    fn ranks(&self, bit_index: u32) -> (u32, u32) {
        if bit_index >= self.universe_size() {
            return (self.num_zeros(), self.num_ones());
        }
        let num_ones = self.rank1(bit_index);
        let num_zeros = bit_index - num_ones;
        return (num_zeros, num_ones);
    }

    /// The bit index of the k-th occurrence of a 1-bit
    fn select1(&self, n: u32) -> Option<u32> {
        if n >= self.num_ones() {
            return None;
        }
        // Binary search over rank1 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let bit_index = partition_point(universe, |i| self.rank1(i as u32) <= n) - 1;
        Some(bit_index as u32)
    }

    /// The bit index of the k-th occurrence of a 0-bit
    fn select0(&self, n: u32) -> Option<u32> {
        if n >= self.num_zeros() {
            return None;
        }
        // Binary search over rank0 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let bit_index = partition_point(universe, |i| self.rank0(i as u32) <= n) - 1;
        Some(bit_index as u32)
    }

    /// Get the value of the bit at the specified index (0 or 1).
    /// The comparable method on MultiBitVec the presence of multiplicity,
    ///  the count of the bit.
    /// Note: This is rather inefficient since it does two rank calls,
    /// each of which may take O(log(n)) time, depending on the BitVec.
    fn get(&self, bit_index: u32) -> u32 {
        assert!(
            bit_index < self.universe_size(),
            "bit index {} cannot exceed universe size {}",
            bit_index,
            self.universe_size()
        );
        self.rank1(bit_index + 1) - self.rank1(bit_index)
    }

    /// The size of the universe, ie. total number of bits in this vector.
    fn universe_size(&self) -> u32;

    /// The number of 1-bits in this vector.
    fn num_ones(&self) -> u32;

    /// The number of 0-bits in this vector.
    fn num_zeros(&self) -> u32 {
        self.universe_size() - self.num_ones()
    }

    /// Given a slice of sorted bit indices, mutates it to contain the ranks at those indices.
    fn rank1_batch(&self, bit_indices: &mut [u32]) {
        for i in bit_indices {
            *i = self.rank1(*i)
        }
    }
}

/// Represents a multiset. 1-bits may have multiplicity, but 0-bits may not.
pub trait MultiBitVec: Clone {
    type Builder: MultiBitVecBuilder<Target = Self>;

    /// Get the multiplicity (count) of the bit at the specified index.
    fn get(&self, bit_index: u32) -> u32 {
        assert!(bit_index < self.universe_size());
        self.rank1(bit_index + 1) - self.rank1(bit_index)
    }

    /// The number of 1-bits below `bit_index`
    fn rank1(&self, bit_index: u32) -> u32;

    // Return the bit index of the k-th occurrence of a 1-bit
    fn select1(&self, n: u32) -> Option<u32>;

    /// The size of the universe, ie. total number of bits in this vector.
    fn universe_size(&self) -> u32;

    /// The number of 1-bits in this vector.
    /// Due to multiplicity, this may exceed the universe size.
    fn num_ones(&self) -> u32;

    /// The number of 0-bits in this vector.
    fn num_zeros(&self) -> u32 {
        self.universe_size() - self.num_unique_ones()
    }

    /// The number of unique 1-bits in this vector.
    fn num_unique_ones(&self) -> u32;

    /// Given a slice of sorted bit indices, mutates it to contain the ranks at those indices.
    fn rank1_batch(&self, bit_indices: &mut [u32]) {
        for i in bit_indices {
            *i = self.rank1(*i)
        }
    }
}

pub trait BitVecBuilder: Clone {
    type Target: BitVec;
    type Options: Default + Clone;

    /// Universe size must be strictly less than u32::MAX for most BitVec types.
    /// The exception is RLEBitVec, for which the maximum universe size is 2^32-2.
    fn new(universe_size: u32, options: Self::Options) -> Self
    where
        Self: Sized;

    /// Set a 1-bit in this bit vector.
    /// Idempotent; the same bit may be set more than once without effect.
    /// 1-bits may be added in any order.
    fn one(&mut self, bit_index: u32);

    /// Build the bit vector
    fn build(self) -> Self::Target;

    /// Convenience method to directly construct a bit vector with the given bits.
    fn from_ones(universe_size: u32, options: Self::Options, ones: &[u32]) -> Self::Target
    where
        Self: Sized,
    {
        let mut b = Self::new(universe_size, options);
        for one in ones.iter().copied() {
            b.one(one)
        }
        b.build()
    }
}

pub trait MultiBitVecBuilder: Clone {
    type Target: MultiBitVec;
    type Options: Default + Clone;

    /// Construct a new builder from a universe size and type-specific options.
    /// Universe size must be strictly less than u32::MAX.
    fn new(universe_size: u32, options: Self::Options) -> Self
    where
        Self: Sized;

    /// Add `count` 1-bits at index `bit_index`.
    /// 1-bits may be added in any order.
    fn ones(&mut self, bit_index: u32, count: u32);

    /// Build the bit vector
    fn build(self) -> Self::Target;

    /// Convenience method to directly construct a bit vector with the given 1-bit positions and counts.
    fn from_ones_counts(
        universe_size: u32,
        options: Self::Options,
        ones: &[u32],
        counts: &[u32],
    ) -> Self::Target
    where
        Self: Sized,
    {
        let mut b = Self::new(universe_size, options);
        for (&one, &count) in ones.iter().zip(counts.iter()) {
            b.ones(one, count)
        }
        b.build()
    }
}

/// Adapter to allow MultiBitVecs to serve as BitVecs.
/// The blanket impl below provides an implementation that uses the
/// default BitVec methods to provide implementations of rank0/select0
/// which rely on the absence of multiplicity.
#[derive(Clone)]
pub struct BitVecOf<T: MultiBitVec>(T);

/// This trait is used to provide a BitVec implementation for
/// MultiBitVecs via a blanket impl for BitVecOf<T> where T: MultiBitVec.
///
/// A BitVec is a specialization of a MultiBitVec where every
/// bit is present 0 or 1 times. Constructing a BitVecOf performs
/// a uniqueness check to enforce this invariant.
///
/// Note:
/// Some MultiBitVecs afford more efficient implementations
/// in the case without multiplicity; in the future, we can introduce
/// a new `DefaultBitVec` trait to allow them to provide their
/// own BitVec implementations for BitVecOf<T>. (Example: Multi<T>
/// can provide more efficient rank1 and rank0 by looking only at
/// its occupancy vector).
///
/// For now, though, we just impl BitVec for all MultiBitVecs this way.
impl<T> BitVec for BitVecOf<T>
where
    T: MultiBitVec,
{
    type Builder = BitVecBuilderOf<T::Builder>;

    fn rank1(&self, bit_index: u32) -> u32 {
        self.inner().rank1(bit_index)
    }

    fn select1(&self, n: u32) -> Option<u32> {
        self.inner().select1(n)
    }

    fn num_ones(&self) -> u32 {
        self.inner().num_ones()
    }

    fn universe_size(&self) -> u32 {
        self.inner().universe_size()
    }
}

impl<T: MultiBitVec> BitVecOf<T> {
    pub fn new(x: T) -> Self {
        assert_eq!(x.num_ones(), x.num_unique_ones());
        Self(x)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }
}

/// Allows use of a MultiBitVecBuilder as a BitVecBuilder
/// by tracking the ones and disallowing more than 1 count
/// of each individual bit to be added to the builder,
/// enforcing idempotency of `BitVecBuilder::one`.
/// (The idempotency requirement is why we can't just use
/// MultiBitVecBuilder directly).
#[derive(Clone)]
pub struct BitVecBuilderOf<B: MultiBitVecBuilder> {
    builder: B,
    ones: BitBuf,
}

impl<B: MultiBitVecBuilder> BitVecBuilder for BitVecBuilderOf<B>
where
    BitVecOf<B::Target>: BitVec,
{
    type Target = BitVecOf<B::Target>;
    type Options = B::Options;

    fn new(universe_size: u32, options: Self::Options) -> Self {
        Self {
            builder: B::new(universe_size, options),
            ones: BitBuf::new(universe_size),
        }
    }

    fn one(&mut self, bit_index: u32) {
        if !self.ones.get(bit_index) {
            self.builder.ones(bit_index, 1);
            self.ones.set_one(bit_index);
        }
    }

    fn build(self) -> Self::Target {
        BitVecOf(self.builder.build())
    }
}
