use crate::bits::partition_point;

pub trait BitVec: Clone {
    /// Get the value of the bit at the specified index (0 or 1).
    /// Note: This is rather inefficient since it does two rank calls,
    /// each of which takes O(log(n)) time.
    ///
    /// The comparable method on MultiBitVec the presence of multiplicity, returns the count of the bit.
    fn get(&self, bit_index: u32) -> u32 {
        assert!(
            bit_index < self.universe_size(),
            "bit index {} cannot exceed universe size {}",
            bit_index,
            self.universe_size()
        );

        self.rank1(bit_index + 1) - self.rank1(bit_index)
    }

    /// Return the number of 1-bits below `bit_index`
    fn rank1(&self, bit_index: u32) -> u32;

    /// Return the number of 0-bits below `bit_index`
    fn rank0(&self, bit_index: u32) -> u32 {
        // The implementation below assumes no multiplicity; otherwise,
        // subtracting rank1 from the bit index can go negative.
        if bit_index >= self.universe_size() {
            self.num_zeros()
        } else {
            bit_index - self.rank1(bit_index)
        }
    }

    // Return the bit index of the k-th occurrence of a 1-bit
    fn select1(&self, n: u32) -> Option<u32> {
        if n >= self.num_ones() {
            return None;
        }
        // Binary search over rank1 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let bit_index = partition_point(universe, |i| self.rank1(i as u32) <= n) - 1;
        Some(bit_index as u32)
    }

    // Return the bit index of the k-th occurrence of a 0-bit
    fn select0(&self, n: u32) -> Option<u32> {
        // assert!(self.has_select0());
        if n >= self.num_zeros() {
            return None;
        }
        // Binary search over rank0 to determine the position of the n-th 0-bit.
        let universe = self.universe_size() as usize;
        let bit_index = partition_point(universe, |i| self.rank0(i as u32) <= n) - 1;
        Some(bit_index as u32)
    }

    fn universe_size(&self) -> u32;
    fn num_ones(&self) -> u32;
    fn num_zeros(&self) -> u32 {
        self.universe_size() - self.num_ones()
    }
}

pub trait BitVecBuilder {
    type Target: BitVec;
    fn new(universe_size: u32) -> Self;
    /// Set a 1-bit in this bit vector.
    /// Idempotent; the same bit may be set more than once without effect.
    /// 1-bits may be added in any order.
    fn one(&mut self, bit_index: u32);
    fn build(self) -> Self::Target;
}

pub trait MultiBitVecBuilder {
    type Target: MultiBitVec;
    fn new(universe_size: u32) -> Self;
    // todo: test zero counts for one_count
    fn one_count(&mut self, bit_index: u32, count: u32);
    fn build(self) -> Self::Target;
}

/// Represents a multiset. 1-bits may have multiplicity, but 0-bits may not.
pub trait MultiBitVec {
    fn get(&self, bit_index: u32) -> u32 {
        assert!(bit_index < self.universe_size());
        self.rank1(bit_index + 1) - self.rank1(bit_index)
    }

    fn rank1(&self, bit_index: u32) -> u32;
    fn select1(&self, n: u32) -> Option<u32>;

    fn universe_size(&self) -> u32;
    fn num_unique_ones(&self) -> u32;
    fn num_unique_zeros(&self) -> u32 {
        self.universe_size() - self.num_unique_ones()
    }
}
